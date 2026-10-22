//! Reconnecting, event-driven root-property model shared by system services.
use asena_core::{i18n::gettext, operation::OperationGate};
use futures_lite::StreamExt;
use glib::{
    prelude::*,
    subclass::{prelude::*, Signal},
};
use std::{
    cell::{Cell, RefCell},
    collections::HashMap,
    sync::OnceLock,
};
use zbus::{proxy::CacheProperties, zvariant::OwnedValue};

#[derive(Clone, Copy, Debug)]
pub enum Bus {
    System,
    Session,
}
#[derive(Clone, Debug)]
pub struct Endpoint {
    pub bus: Bus,
    pub name: String,
    pub path: String,
    pub interface: String,
}
impl Endpoint {
    pub fn system(name: &str, path: &str, interface: &str) -> Self {
        Self {
            bus: Bus::System,
            name: name.into(),
            path: path.into(),
            interface: interface.into(),
        }
    }
}
mod imp {
    use super::*;
    #[derive(Default, glib::Properties)]
    #[properties(wrapper_type=super::PropertyModel)]
    pub struct PropertyModel {
        #[property(get)]
        pub available: Cell<bool>,
        #[property(get)]
        pub busy: Cell<bool>,
        #[property(get)]
        pub status: RefCell<String>,
        #[property(get)]
        pub last_error: RefCell<String>,
        pub values: RefCell<HashMap<String, OwnedValue>>,
        pub endpoint: RefCell<Option<Endpoint>>,
        pub owner: RefCell<Option<String>>,
        pub connection: RefCell<Option<zbus::Connection>>,
        pub gate: RefCell<OperationGate>,
        pub task: RefCell<Option<glib::JoinHandle<()>>>,
    }
    #[glib::object_subclass]
    impl ObjectSubclass for PropertyModel {
        const NAME: &'static str = "AsenaPropertyModel";
        type Type = super::PropertyModel;
    }
    #[glib::derived_properties]
    impl ObjectImpl for PropertyModel {
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
glib::wrapper! {pub struct PropertyModel(ObjectSubclass<imp::PropertyModel>);}
impl PropertyModel {
    pub fn new(endpoint: Endpoint) -> Self {
        let model: Self = glib::Object::new();
        model.imp().endpoint.replace(Some(endpoint));
        model.set_status("waiting");
        model
    }
    fn set_status(&self, status: &str) {
        if *self.imp().status.borrow() != status {
            self.imp().status.replace(status.into());
            self.notify_status();
        }
    }
    fn changed(&self) {
        self.emit_by_name::<()>("changed", &[]);
    }
    fn owner_changed(&self, owner: Option<String>) {
        self.imp().gate.borrow_mut().owner_changed(owner.is_some());
        self.imp().owner.replace(owner);
        self.imp().values.borrow_mut().clear();
        self.imp().busy.set(false);
        self.notify_busy();
        self.imp().available.set(false);
        self.notify_available();
        self.set_status("missing");
        self.changed();
    }
    fn apply(&self, values: HashMap<String, OwnedValue>) {
        self.imp().values.replace(values);
        self.imp().available.set(true);
        self.notify_available();
        self.set_status("ready");
        self.changed();
    }
    fn failure(&self, error: &impl std::fmt::Display) {
        let message = error.to_string();
        self.set_status(
            if message.contains("AccessDenied") || message.contains("NotAuthorized") {
                "denied"
            } else {
                "failed"
            },
        );
        self.imp().last_error.replace(message);
        self.notify_last_error();
        self.changed();
    }
    pub fn boolean(&self, key: &str) -> Option<bool> {
        self.imp()
            .values
            .borrow()
            .get(key)
            .and_then(|v| bool::try_from(v).ok())
    }
    pub fn number(&self, key: &str) -> Option<f64> {
        self.imp()
            .values
            .borrow()
            .get(key)
            .and_then(|v| f64::try_from(v).ok())
    }
    pub fn unsigned(&self, key: &str) -> Option<u32> {
        self.imp()
            .values
            .borrow()
            .get(key)
            .and_then(|v| u32::try_from(v).ok())
    }
    pub fn string(&self, key: &str) -> Option<String> {
        self.imp()
            .values
            .borrow()
            .get(key)
            .and_then(|v| <&str>::try_from(v).ok())
            .map(ToOwned::to_owned)
    }
    pub fn with_values<T>(&self, callback: impl FnOnce(&HashMap<String, OwnedValue>) -> T) -> T {
        callback(&self.imp().values.borrow())
    }
    pub fn connect_changed(&self, callback: impl Fn(&Self) + 'static) -> glib::SignalHandlerId {
        self.connect_local("changed", false, move |values| {
            if let Ok(model) = values[0].get::<Self>() {
                callback(&model);
            }
            None
        })
    }
    pub fn start(&self) {
        if self.imp().task.borrow().is_some() {
            return;
        }
        let Some(endpoint) = self.imp().endpoint.borrow().clone() else {
            return;
        };
        let weak = self.downgrade();
        let task = glib::spawn_future_local(async move {
            let connection = match match endpoint.bus {
                Bus::System => zbus::Connection::system().await,
                Bus::Session => zbus::Connection::session().await,
            } {
                Ok(connection) => connection,
                Err(error) => {
                    if let Some(model) = weak.upgrade() {
                        model.failure(&error);
                    }
                    return;
                }
            };
            if let Some(model) = weak.upgrade() {
                model.imp().connection.replace(Some(connection.clone()));
            }
            let result: zbus::Result<()> = async {
                let watch = zbus::Proxy::new_owned(
                    connection.clone(),
                    endpoint.name.clone(),
                    endpoint.path.clone(),
                    endpoint.interface.clone(),
                )
                .await?;
                let mut owners = watch.receive_owner_changed().await?;
                let bus = zbus::fdo::DBusProxy::new(&connection).await?;
                if let Ok(name) = zbus::names::WellKnownName::try_from(endpoint.name.as_str()) {
                    let _ = bus.start_service_by_name(name, 0).await;
                }
                let mut owner = bus
                    .get_name_owner(zbus::names::BusName::try_from(endpoint.name.as_str())?)
                    .await
                    .ok()
                    .map(|s| s.to_string());
                let interface = zbus::names::InterfaceName::try_from(endpoint.interface.as_str())?;
                loop {
                    let Some(model) = weak.upgrade() else {
                        return Ok(());
                    };
                    model.owner_changed(owner.clone());
                    drop(model);
                    if let Some(current) = owner.clone() {
                        let properties = zbus::fdo::PropertiesProxy::builder(&connection)
                            .destination(current)?
                            .path(endpoint.path.as_str())?
                            .cache_properties(CacheProperties::No)
                            .build()
                            .await?;
                        let mut changes = properties.receive_properties_changed().await?;
                        let mut native = watch.receive_all_signals().await?;
                        loop {
                            enum Refresh<T> {
                                Values(T),
                                Owner(Option<Option<zbus::names::UniqueName<'static>>>),
                            }
                            let update = futures_lite::future::race(
                                async {
                                    Refresh::Values(properties.get_all(interface.clone()).await)
                                },
                                async { Refresh::Owner(owners.next().await) },
                            )
                            .await;
                            match update {
                                Refresh::Values(Ok(values)) => {
                                    if let Some(model) = weak.upgrade() {
                                        model.apply(values);
                                    }
                                }
                                Refresh::Values(Err(error)) => {
                                    if let Some(model) = weak.upgrade() {
                                        model.failure(&error);
                                    }
                                }
                                Refresh::Owner(None) => return Ok(()),
                                Refresh::Owner(Some(next)) => {
                                    owner = next.map(|s| s.to_string());
                                    break;
                                }
                            }
                            enum Event {
                                Owner(Option<Option<zbus::names::UniqueName<'static>>>),
                                Properties(bool),
                            }
                            match futures_lite::future::race(
                                async { Event::Owner(owners.next().await) },
                                futures_lite::future::race(
                                    async { Event::Properties(changes.next().await.is_some()) },
                                    async { Event::Properties(native.next().await.is_some()) },
                                ),
                            )
                            .await
                            {
                                Event::Owner(None) => return Ok(()),
                                Event::Owner(Some(next)) => {
                                    owner = next.map(|s| s.to_string());
                                    break;
                                }
                                Event::Properties(true) => {}
                                Event::Properties(false) => {
                                    owner = None;
                                    break;
                                }
                            }
                        }
                    } else {
                        let Some(next) = owners.next().await else {
                            return Ok(());
                        };
                        owner = next.map(|s| s.to_string());
                    }
                }
            }
            .await;
            if let Err(error) = result {
                if let Some(model) = weak.upgrade() {
                    model.owner_changed(None);
                    model.failure(&error);
                }
            }
        });
        self.imp().task.replace(Some(task));
    }
    pub async fn set_property(
        &self,
        name: &str,
        value: OwnedValue,
    ) -> Result<(), crate::error::ServiceError> {
        if !self.available() {
            return Err(gettext("The service is unavailable or busy.").into());
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
        let endpoint = self.imp().endpoint.borrow().clone();
        let connection = self.imp().connection.borrow().clone();
        let owner = self.imp().owner.borrow().clone();
        let result = async {
            let (Some(endpoint), Some(connection), Some(owner)) = (endpoint, connection, owner)
            else {
                return Err(zbus::Error::Failure(gettext("Service disconnected.")));
            };
            let proxy = zbus::fdo::PropertiesProxy::builder(&connection)
                .destination(owner)?
                .path(endpoint.path.as_str())?
                .cache_properties(CacheProperties::No)
                .build()
                .await?;
            proxy
                .set(
                    zbus::names::InterfaceName::try_from(endpoint.interface.as_str())?,
                    name,
                    value.into(),
                )
                .await?;
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
    pub async fn call_unit(&self, method: &str) -> Result<(), crate::error::ServiceError> {
        if !self.available() {
            return Err(gettext("The service is unavailable or busy.").into());
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
        let endpoint = self.imp().endpoint.borrow().clone();
        let connection = self.imp().connection.borrow().clone();
        let owner = self.imp().owner.borrow().clone();
        let result: zbus::Result<()> = async {
            let (Some(endpoint), Some(connection), Some(owner)) = (endpoint, connection, owner)
            else {
                return Err(zbus::Error::Failure(gettext("Service disconnected.")));
            };
            let proxy =
                zbus::Proxy::new_owned(connection, owner, endpoint.path, endpoint.interface)
                    .await?;
            proxy.call(method, &()).await
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
