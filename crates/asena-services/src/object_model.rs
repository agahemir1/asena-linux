//! ObjectManager cache, updated by deltas and cleared on service ownership loss.
use crate::property_model::{Bus, Endpoint};
use asena_core::{i18n::gettext, operation::OperationGate};
use futures_lite::StreamExt;
use glib::{
    prelude::*,
    subclass::{prelude::*, Signal},
};
use std::{
    cell::{Cell, RefCell},
    collections::{BTreeMap, HashMap},
    sync::OnceLock,
};
use zbus::zvariant::{OwnedObjectPath, OwnedValue};
pub type Properties = HashMap<String, OwnedValue>;
pub type Interfaces = HashMap<String, Properties>;
pub type Objects = HashMap<OwnedObjectPath, Interfaces>;
mod imp {
    use super::*;
    #[derive(Default, glib::Properties)]
    #[properties(wrapper_type=super::ObjectModel)]
    pub struct ObjectModel {
        #[property(get)]
        pub available: Cell<bool>,
        #[property(get)]
        pub busy: Cell<bool>,
        #[property(get)]
        pub last_error: RefCell<String>,
        pub objects: RefCell<Objects>,
        pub endpoint: RefCell<Option<Endpoint>>,
        pub connection: RefCell<Option<zbus::Connection>>,
        pub owner: RefCell<Option<String>>,
        pub epoch: Cell<u64>,
        pub revisions: RefCell<BTreeMap<(String, String), u64>>,
        pub gate: RefCell<OperationGate>,
        pub task: RefCell<Option<glib::JoinHandle<()>>>,
    }
    #[glib::object_subclass]
    impl ObjectSubclass for ObjectModel {
        const NAME: &'static str = "AsenaObjectModel";
        type Type = super::ObjectModel;
    }
    #[glib::derived_properties]
    impl ObjectImpl for ObjectModel {
        fn signals() -> &'static [Signal] {
            static SIGNALS: OnceLock<Vec<Signal>> = OnceLock::new();
            SIGNALS.get_or_init(|| vec![Signal::builder("changed").build()])
        }
        fn dispose(&self) {
            if let Some(task) = self.task.borrow_mut().take() {
                task.abort();
            }
        }
    }
}
glib::wrapper! {pub struct ObjectModel(ObjectSubclass<imp::ObjectModel>);}
impl ObjectModel {
    pub fn new(endpoint: Endpoint) -> Self {
        let model: Self = glib::Object::new();
        model.imp().endpoint.replace(Some(endpoint));
        model
    }
    fn changed(&self) {
        self.emit_by_name::<()>("changed", &[]);
    }
    fn owner_changed(&self, owner: Option<String>) {
        self.imp().epoch.set(self.imp().epoch.get().wrapping_add(1));
        self.imp().gate.borrow_mut().owner_changed(owner.is_some());
        self.imp().owner.replace(owner);
        self.imp().objects.borrow_mut().clear();
        self.imp().revisions.borrow_mut().clear();
        self.imp().available.set(false);
        self.notify_available();
        self.imp().busy.set(false);
        self.notify_busy();
        self.changed();
    }
    pub fn with_objects<T>(&self, callback: impl FnOnce(&Objects) -> T) -> T {
        callback(&self.imp().objects.borrow())
    }
    pub fn connect_changed(&self, callback: impl Fn(&Self) + 'static) -> glib::SignalHandlerId {
        self.connect_local("changed", false, move |values| {
            if let Ok(model) = values[0].get::<Self>() {
                callback(&model);
            }
            None
        })
    }
    fn failure(&self, error: impl std::fmt::Display) {
        self.imp().last_error.replace(error.to_string());
        self.notify_last_error();
        self.changed();
    }
    fn bump(&self, path: &str, interface: &str) -> u64 {
        let mut revisions = self.imp().revisions.borrow_mut();
        let revision = revisions
            .entry((path.into(), interface.into()))
            .or_default();
        *revision = revision.wrapping_add(1);
        *revision
    }
    fn message(&self, message: zbus::Message) {
        let header = message.header();
        let Some(path) = header.path().map(ToString::to_string) else {
            return;
        };
        let interface = header.interface().map(|v| v.as_str()).unwrap_or("");
        let member = header.member().map(|v| v.as_str()).unwrap_or("");
        match (interface, member) {
            ("org.freedesktop.DBus.ObjectManager", "InterfacesAdded") => {
                if let Ok((path, interfaces)) = message
                    .body()
                    .deserialize::<(OwnedObjectPath, Interfaces)>()
                {
                    for interface in interfaces.keys() {
                        self.bump(path.as_str(), interface);
                    }
                    self.imp()
                        .objects
                        .borrow_mut()
                        .entry(path)
                        .or_default()
                        .extend(interfaces);
                    self.changed();
                }
            }
            ("org.freedesktop.DBus.ObjectManager", "InterfacesRemoved") => {
                if let Ok((path, interfaces)) = message
                    .body()
                    .deserialize::<(OwnedObjectPath, Vec<String>)>()
                {
                    for interface in &interfaces {
                        self.bump(path.as_str(), interface);
                    }
                    let mut objects = self.imp().objects.borrow_mut();
                    if let Some(object) = objects.get_mut(&path) {
                        for interface in interfaces {
                            object.remove(&interface);
                        }
                        if object.is_empty() {
                            objects.remove(&path);
                        }
                    }
                    drop(objects);
                    self.changed();
                }
            }
            ("org.freedesktop.DBus.Properties", "PropertiesChanged") => {
                if let Ok((interface, values, invalidated)) =
                    message
                        .body()
                        .deserialize::<(String, Properties, Vec<String>)>()
                {
                    let revision = self.bump(&path, &interface);
                    let key = OwnedObjectPath::try_from(path.clone());
                    if let Ok(key) = key {
                        if let Some(properties) = self
                            .imp()
                            .objects
                            .borrow_mut()
                            .get_mut(&key)
                            .and_then(|object| object.get_mut(&interface))
                        {
                            properties.extend(values);
                            for property in &invalidated {
                                properties.remove(property);
                            }
                        }
                    }
                    self.changed();
                    if !invalidated.is_empty() {
                        self.reread(path, interface, revision);
                    }
                }
            }
            _ => {}
        }
    }
    fn reread(&self, path: String, interface: String, revision: u64) {
        let (Some(connection), Some(owner)) = (
            self.imp().connection.borrow().clone(),
            self.imp().owner.borrow().clone(),
        ) else {
            return;
        };
        let epoch = self.imp().epoch.get();
        let weak = self.downgrade();
        glib::spawn_future_local(async move {
            let result: zbus::fdo::Result<Properties> = async {
                let proxy = zbus::fdo::PropertiesProxy::builder(&connection)
                    .destination(owner)?
                    .path(path.as_str())?
                    .build()
                    .await?;
                proxy
                    .get_all(
                        zbus::names::InterfaceName::try_from(interface.as_str())
                            .map_err(zbus::Error::from)?,
                    )
                    .await
            }
            .await;
            if let (Ok(values), Some(model)) = (result, weak.upgrade()) {
                if model.imp().epoch.get() != epoch
                    || model
                        .imp()
                        .revisions
                        .borrow()
                        .get(&(path.clone(), interface.clone()))
                        .copied()
                        != Some(revision)
                {
                    return;
                }
                if let Ok(key) = OwnedObjectPath::try_from(path) {
                    if let Some(object) = model.imp().objects.borrow_mut().get_mut(&key) {
                        if object.contains_key(&interface) {
                            object.insert(interface, values);
                        }
                    }
                }
                model.changed();
            }
        });
    }
    pub fn start(&self) {
        if self.imp().task.borrow().is_some() {
            return;
        }
        let Some(endpoint) = self.imp().endpoint.borrow().clone() else {
            return;
        };
        let weak = self.downgrade();
        self.imp()
            .task
            .replace(Some(glib::spawn_future_local(async move {
                let result: zbus::Result<()> = async {
                    let connection = match endpoint.bus {
                        Bus::System => zbus::Connection::system().await?,
                        Bus::Session => zbus::Connection::session().await?,
                    };
                    if let Some(model) = weak.upgrade() {
                        model.imp().connection.replace(Some(connection.clone()));
                    }
                    let watch = zbus::Proxy::new_owned(
                        connection.clone(),
                        endpoint.name.clone(),
                        endpoint.path.clone(),
                        "org.freedesktop.DBus.ObjectManager".to_owned(),
                    )
                    .await?;
                    let mut owners = watch.receive_owner_changed().await?;
                    let bus = zbus::fdo::DBusProxy::new(&connection).await?;
                    let _ = bus
                        .start_service_by_name(
                            zbus::names::WellKnownName::try_from(endpoint.name.as_str())?,
                            0,
                        )
                        .await;
                    let mut owner = bus
                        .get_name_owner(zbus::names::BusName::try_from(endpoint.name.as_str())?)
                        .await
                        .ok()
                        .map(|v| v.to_string());
                    loop {
                        let Some(model) = weak.upgrade() else {
                            return Ok(());
                        };
                        model.owner_changed(owner.clone());
                        drop(model);
                        let Some(current) = owner.clone() else {
                            let Some(next) = owners.next().await else {
                                return Ok(());
                            };
                            owner = next.map(|v| v.to_string());
                            continue;
                        };
                        let rule = zbus::MatchRule::builder()
                            .msg_type(zbus::message::Type::Signal)
                            .sender(current.clone())?
                            .build();
                        let mut signals =
                            zbus::MessageStream::for_match_rule(rule, &connection, Some(128))
                                .await?;
                        let proxy = zbus::fdo::ObjectManagerProxy::builder(&connection)
                            .destination(current.clone())?
                            .path(endpoint.path.as_str())?
                            .build()
                            .await?;
                        enum Initial<T> {
                            Objects(T),
                            Owner(Option<Option<zbus::names::UniqueName<'static>>>),
                        }
                        match futures_lite::future::race(
                            async { Initial::Objects(proxy.get_managed_objects().await) },
                            async { Initial::Owner(owners.next().await) },
                        )
                        .await
                        {
                            Initial::Owner(None) => return Ok(()),
                            Initial::Owner(Some(next)) => {
                                owner = next.map(|v| v.to_string());
                                continue;
                            }
                            Initial::Objects(Ok(objects)) => {
                                if let Some(model) = weak.upgrade() {
                                    model.imp().objects.replace(
                                        objects
                                            .into_iter()
                                            .map(|(path, interfaces)| {
                                                (
                                                    path,
                                                    interfaces
                                                        .into_iter()
                                                        .map(|(name, values)| {
                                                            (name.to_string(), values)
                                                        })
                                                        .collect(),
                                                )
                                            })
                                            .collect(),
                                    );
                                    model.imp().available.set(true);
                                    model.notify_available();
                                    model.changed();
                                }
                            }
                            Initial::Objects(Err(error)) => {
                                if let Some(model) = weak.upgrade() {
                                    model.failure(error);
                                }
                            }
                        }
                        loop {
                            enum Event {
                                Owner(Option<Option<zbus::names::UniqueName<'static>>>),
                                Signal(Option<zbus::Result<zbus::Message>>),
                            }
                            match futures_lite::future::race(
                                async { Event::Owner(owners.next().await) },
                                async { Event::Signal(signals.next().await) },
                            )
                            .await
                            {
                                Event::Owner(None) => return Ok(()),
                                Event::Owner(Some(next)) => {
                                    owner = next.map(|v| v.to_string());
                                    break;
                                }
                                Event::Signal(None) => return Ok(()),
                                Event::Signal(Some(Err(error))) => return Err(error),
                                Event::Signal(Some(Ok(message))) => {
                                    if message
                                        .header()
                                        .sender()
                                        .is_some_and(|sender| sender.as_str() == current)
                                    {
                                        if let Some(model) = weak.upgrade() {
                                            model.message(message);
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
                .await;
                if let Some(model) = weak.upgrade() {
                    model.owner_changed(None);
                    if let Err(error) = result {
                        model.failure(error);
                    }
                }
            })));
    }
    pub async fn set_bool(
        &self,
        path: &str,
        interface: &str,
        property: &str,
        value: bool,
    ) -> Result<(), crate::error::ServiceError> {
        self.perform(path, interface, Some((property, value)), None)
            .await
    }
    pub async fn call_unit(
        &self,
        path: &str,
        interface: &str,
        method: &str,
    ) -> Result<(), crate::error::ServiceError> {
        self.perform(path, interface, None, Some(method)).await
    }
    pub async fn call<B, R>(
        &self,
        path: &str,
        interface: &str,
        method: &str,
        body: &B,
    ) -> Result<R, crate::error::ServiceError>
    where
        B: serde::ser::Serialize + zbus::zvariant::DynamicType,
        R: for<'d> zbus::zvariant::DynamicDeserialize<'d>,
    {
        let exists = self.with_objects(|objects| {
            objects
                .iter()
                .any(|(key, interfaces)| key.as_str() == path && interfaces.contains_key(interface))
        });
        if !self.available() || !exists {
            return Err(gettext("The device is no longer available.").into());
        }
        let ticket = self
            .imp()
            .gate
            .borrow_mut()
            .begin()
            .ok_or_else(|| gettext("The service is unavailable or busy."))?;
        self.imp().busy.set(true);
        self.notify_busy();
        self.changed();
        let result: zbus::Result<R> = async {
            let (Some(connection), Some(owner)) = (
                self.imp().connection.borrow().clone(),
                self.imp().owner.borrow().clone(),
            ) else {
                return Err(zbus::Error::Failure(gettext("Service disconnected.")));
            };
            let proxy =
                zbus::Proxy::new_owned(connection, owner, path.to_owned(), interface.to_owned())
                    .await?;
            proxy.call(method, body).await
        }
        .await;
        if !self.imp().gate.borrow_mut().finish(ticket) {
            return Err(gettext("The service changed while the operation was pending.").into());
        }
        self.imp().busy.set(false);
        self.notify_busy();
        self.changed();
        match result {
            Ok(value) => Ok(value),
            Err(error) => {
                self.failure(&error);
                Err(error.to_string().into())
            }
        }
    }
    async fn perform(
        &self,
        path: &str,
        interface: &str,
        property: Option<(&str, bool)>,
        method: Option<&str>,
    ) -> Result<(), crate::error::ServiceError> {
        let exists = self.with_objects(|objects| {
            objects
                .iter()
                .any(|(key, interfaces)| key.as_str() == path && interfaces.contains_key(interface))
        });
        if !self.available() || !exists {
            return Err(gettext("The device is no longer available.").into());
        }
        let ticket = self
            .imp()
            .gate
            .borrow_mut()
            .begin()
            .ok_or_else(|| gettext("The service is unavailable or busy."))?;
        self.imp().busy.set(true);
        self.notify_busy();
        self.changed();
        let result: zbus::Result<()> = async {
            let (Some(connection), Some(owner)) = (
                self.imp().connection.borrow().clone(),
                self.imp().owner.borrow().clone(),
            ) else {
                return Err(zbus::Error::Failure(gettext("Service disconnected.")));
            };
            let proxy =
                zbus::Proxy::new_owned(connection, owner, path.to_owned(), interface.to_owned())
                    .await?;
            if let Some((property, value)) = property {
                proxy.set_property(property, &value).await?;
            } else if let Some(method) = method {
                proxy.call::<_, _, ()>(method, &()).await?;
            }
            Ok(())
        }
        .await;
        if !self.imp().gate.borrow_mut().finish(ticket) {
            return Err(gettext("The service changed while the operation was pending.").into());
        }
        self.imp().busy.set(false);
        self.notify_busy();
        self.changed();
        if let Err(error) = result {
            self.failure(&error);
            return Err(error.to_string().into());
        }
        Ok(())
    }
}
