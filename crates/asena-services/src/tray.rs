//! StatusNotifier watcher/host. Existing watchers are used without takeover.
use crate::{
    error::ServiceError,
    property_model::{Bus, Endpoint, PropertyModel},
};
use asena_core::{dbusmenu, i18n::gettext};
use futures_lite::StreamExt;
use glib::{
    prelude::*,
    subclass::{prelude::*, Signal},
};
use std::{
    cell::{Cell, RefCell},
    collections::{BTreeMap, BTreeSet},
    sync::{Arc, Mutex, OnceLock},
};
use zbus::zvariant::Value;
const NAME: &str = "org.kde.StatusNotifierWatcher";
const PATH: &str = "/StatusNotifierWatcher";
const ITEM: &str = "org.kde.StatusNotifierItem";
#[derive(Clone)]
pub struct Item {
    pub id: String,
    pub owner: String,
    pub path: String,
    pub properties: PropertyModel,
}
#[derive(Default)]
struct Shared {
    items: BTreeSet<String>,
    hosts: BTreeSet<String>,
}
struct Watcher {
    shared: Arc<Mutex<Shared>>,
    sender: async_channel::Sender<(String, String)>,
}
async fn validate_owner(
    connection: &zbus::Connection,
    service: &str,
    sender: &str,
) -> zbus::fdo::Result<String> {
    let bus = zbus::fdo::DBusProxy::new(connection).await?;
    let name = zbus::names::BusName::try_from(service)
        .map_err(|e| zbus::fdo::Error::InvalidArgs(e.to_string()))?;
    let owner = bus.get_name_owner(name).await?;
    if owner.as_str() != sender {
        return Err(zbus::fdo::Error::AccessDenied(gettext(
            "The tray service belongs to another connection.",
        )));
    }
    Ok(owner.to_string())
}
#[zbus::interface(name = "org.kde.StatusNotifierWatcher")]
impl Watcher {
    async fn register_status_notifier_item(
        &self,
        service: &str,
        #[zbus(header)] header: zbus::message::Header<'_>,
        #[zbus(connection)] connection: &zbus::Connection,
    ) -> zbus::fdo::Result<()> {
        let sender = header
            .sender()
            .ok_or_else(|| {
                zbus::fdo::Error::AccessDenied(gettext("The tray sender could not be identified."))
            })?
            .as_str();
        let (owner, path) = if service.starts_with('/') {
            (sender.to_owned(), service.to_owned())
        } else {
            (
                validate_owner(connection, service, sender).await?,
                "/StatusNotifierItem".into(),
            )
        };
        zbus::zvariant::ObjectPath::try_from(path.as_str())
            .map_err(|e| zbus::fdo::Error::InvalidArgs(e.to_string()))?;
        let id = format!("{owner}{path}");
        let inserted = {
            let mut shared = self
                .shared
                .lock()
                .map_err(|_| zbus::fdo::Error::Failed(gettext("Tray state is unavailable.")))?;
            if shared.items.len() >= 128 && !shared.items.contains(&id) {
                return Err(zbus::fdo::Error::LimitsExceeded(gettext(
                    "Too many tray items.",
                )));
            }
            shared.items.insert(id.clone())
        };
        if inserted {
            self.sender
                .send((owner, path))
                .await
                .map_err(|_| zbus::fdo::Error::Failed(gettext("The shell is shutting down.")))?;
            connection
                .emit_signal(
                    None::<&str>,
                    PATH,
                    NAME,
                    "StatusNotifierItemRegistered",
                    &(id,),
                )
                .await?;
        }
        Ok(())
    }
    async fn register_status_notifier_host(
        &self,
        service: &str,
        #[zbus(header)] header: zbus::message::Header<'_>,
        #[zbus(connection)] connection: &zbus::Connection,
    ) -> zbus::fdo::Result<()> {
        let sender = header
            .sender()
            .ok_or_else(|| {
                zbus::fdo::Error::AccessDenied(gettext("The tray sender could not be identified."))
            })?
            .as_str();
        let owner = validate_owner(connection, service, sender).await?;
        self.shared
            .lock()
            .map_err(|_| zbus::fdo::Error::Failed(gettext("Tray state is unavailable.")))?
            .hosts
            .insert(owner);
        connection
            .emit_signal(
                None::<&str>,
                PATH,
                NAME,
                "StatusNotifierHostRegistered",
                &(),
            )
            .await?;
        Ok(())
    }
    #[zbus(property)]
    fn registered_status_notifier_items(&self) -> Vec<String> {
        self.shared
            .lock()
            .map(|shared| shared.items.iter().cloned().collect())
            .unwrap_or_default()
    }
    #[zbus(property)]
    fn is_status_notifier_host_registered(&self) -> bool {
        self.shared
            .lock()
            .map(|shared| !shared.hosts.is_empty())
            .unwrap_or(false)
    }
    #[zbus(property)]
    fn protocol_version(&self) -> i32 {
        0
    }
    #[zbus(signal)]
    async fn status_notifier_item_registered(
        emitter: &zbus::object_server::SignalEmitter<'_>,
        service: &str,
    ) -> zbus::Result<()>;
    #[zbus(signal)]
    async fn status_notifier_item_unregistered(
        emitter: &zbus::object_server::SignalEmitter<'_>,
        service: &str,
    ) -> zbus::Result<()>;
    #[zbus(signal)]
    async fn status_notifier_host_registered(
        emitter: &zbus::object_server::SignalEmitter<'_>,
    ) -> zbus::Result<()>;
}
mod imp {
    use super::*;
    #[derive(Default, glib::Properties)]
    #[properties(wrapper_type=super::TrayModel)]
    pub struct TrayModel {
        #[property(get)]
        pub available: Cell<bool>,
        pub items: RefCell<BTreeMap<String, Item>>,
        pub connection: RefCell<Option<zbus::Connection>>,
        pub task: RefCell<Option<glib::JoinHandle<()>>>,
        pub external: RefCell<Option<PropertyModel>>,
        pub generation: Cell<u64>,
        pub registered: Cell<bool>,
        pub host_name: RefCell<String>,
    }
    #[glib::object_subclass]
    impl ObjectSubclass for TrayModel {
        const NAME: &'static str = "AsenaTrayModel";
        type Type = super::TrayModel;
    }
    #[glib::derived_properties]
    impl ObjectImpl for TrayModel {
        fn signals() -> &'static [Signal] {
            static SIGNALS: OnceLock<Vec<Signal>> = OnceLock::new();
            SIGNALS.get_or_init(|| vec![Signal::builder("changed").build()])
        }
        fn dispose(&self) {
            if let Some(task) = self.task.borrow_mut().take() {
                task.abort();
            }
            self.items.borrow_mut().clear();
            self.external.borrow_mut().take();
        }
    }
}
glib::wrapper! {pub struct TrayModel(ObjectSubclass<imp::TrayModel>);}
impl Default for TrayModel {
    fn default() -> Self {
        glib::Object::new()
    }
}
impl TrayModel {
    fn changed(&self) {
        self.emit_by_name::<()>("changed", &[]);
    }
    fn attach(&self, id: String, owner: String, path: String) {
        if let Some(item) = self.imp().items.borrow().get(&id) {
            if item.owner == owner && item.path == path {
                return;
            }
        }
        if self.imp().items.borrow().len() >= 128 {
            return;
        }
        let properties = PropertyModel::new(Endpoint {
            bus: Bus::Session,
            name: owner.clone(),
            path: path.clone(),
            interface: ITEM.into(),
        });
        let weak = self.downgrade();
        properties.connect_changed(move |_| {
            if let Some(model) = weak.upgrade() {
                model.changed();
            }
        });
        properties.start();
        self.imp().items.borrow_mut().insert(
            id.clone(),
            Item {
                id,
                owner,
                path,
                properties,
            },
        );
        self.changed();
    }
    fn remove_owner(&self, owner: &str) {
        self.imp()
            .items
            .borrow_mut()
            .retain(|_, item| item.owner != owner);
        self.changed();
    }
    pub fn items(&self) -> Vec<Item> {
        self.imp()
            .items
            .borrow()
            .values()
            .filter(|item| {
                item.properties.available()
                    && item.properties.string("Status").as_deref() != Some("Passive")
            })
            .cloned()
            .collect()
    }
    pub fn connect_changed(&self, callback: impl Fn(&Self) + 'static) -> glib::SignalHandlerId {
        self.connect_local("changed", false, move |values| {
            if let Ok(model) = values[0].get::<Self>() {
                callback(&model);
            }
            None
        })
    }
    fn external_update(&self, root: &PropertyModel) {
        self.imp().available.set(root.available());
        self.notify_available();
        let generation = self.imp().generation.get().wrapping_add(1);
        self.imp().generation.set(generation);
        if !root.available() {
            self.imp().registered.set(false);
            self.imp().items.borrow_mut().clear();
            self.changed();
            return;
        }
        let wanted = root
            .with_values(|values| {
                values
                    .get("RegisteredStatusNotifierItems")
                    .and_then(|v| v.try_clone().ok())
                    .and_then(|v| Vec::<String>::try_from(v).ok())
                    .unwrap_or_default()
            })
            .into_iter()
            .take(128)
            .collect::<BTreeSet<_>>();
        self.imp()
            .items
            .borrow_mut()
            .retain(|id, _| wanted.contains(id));
        self.changed();
        let Some(connection) = self.imp().connection.borrow().clone() else {
            return;
        };
        let register = !self.imp().registered.replace(true);
        let host = self.imp().host_name.borrow().clone();
        let weak = self.downgrade();
        glib::spawn_future_local(async move {
            if register {
                let result: zbus::Result<()> = async {
                    let proxy = zbus::Proxy::new(&connection, NAME, PATH, NAME).await?;
                    proxy.call("RegisterStatusNotifierHost", &(host,)).await
                }
                .await;
                if let Err(error) = result {
                    tracing::warn!(%error,"Could not register with the existing tray watcher");
                    return;
                }
            }
            let Ok(bus) = zbus::fdo::DBusProxy::new(&connection).await else {
                return;
            };
            for id in wanted {
                let (service, path) = if let Some((service, path)) = id.split_once('/') {
                    (service.to_owned(), format!("/{path}"))
                } else {
                    (id.clone(), "/StatusNotifierItem".into())
                };
                let Ok(name) = zbus::names::BusName::try_from(service.as_str()) else {
                    continue;
                };
                let Ok(owner) = bus.get_name_owner(name).await else {
                    continue;
                };
                if zbus::zvariant::ObjectPath::try_from(path.as_str()).is_err() {
                    continue;
                }
                let Some(model) = weak.upgrade() else {
                    return;
                };
                if model.imp().generation.get() != generation {
                    return;
                }
                model.attach(id, owner.to_string(), path);
            }
        });
    }
    pub fn start(&self) {
        if self.imp().task.borrow().is_some() {
            return;
        }
        let weak = self.downgrade();
        self.imp()
            .task
            .replace(Some(glib::spawn_future_local(async move {
                let result: zbus::Result<()> = async {
                    let shared = Arc::new(Mutex::new(Shared::default()));
                    let (sender, receiver) = async_channel::bounded(128);
                    let connection = zbus::connection::Builder::session()?
                        .serve_at(
                            PATH,
                            Watcher {
                                shared: shared.clone(),
                                sender,
                            },
                        )?
                        .build()
                        .await?;
                    let bus = zbus::fdo::DBusProxy::new(&connection).await?;
                    let mut owners = bus.receive_name_owner_changed().await?;
                    let host = format!("org.asenalinux.StatusNotifierHost.p{}", std::process::id());
                    asena_core::dbus::claim_name(&connection, &host).await?;
                    let own = asena_core::dbus::claim_name(&connection, NAME)
                        .await
                        .is_ok();
                    let Some(model) = weak.upgrade() else {
                        return Ok(());
                    };
                    model.imp().connection.replace(Some(connection.clone()));
                    model.imp().host_name.replace(host);
                    model.imp().available.set(true);
                    model.notify_available();
                    if own {
                        if let Ok(mut shared) = shared.lock() {
                            if let Some(name) = connection.unique_name() {
                                shared.hosts.insert(name.to_string());
                            }
                        }
                        connection
                            .emit_signal(
                                None::<&str>,
                                PATH,
                                NAME,
                                "StatusNotifierHostRegistered",
                                &(),
                            )
                            .await?;
                    } else {
                        connection
                            .object_server()
                            .remove::<Watcher, _>(PATH)
                            .await?;
                        let root = PropertyModel::new(Endpoint {
                            bus: Bus::Session,
                            name: NAME.into(),
                            path: PATH.into(),
                            interface: NAME.into(),
                        });
                        let weak = model.downgrade();
                        root.connect_changed(move |root| {
                            if let Some(model) = weak.upgrade() {
                                model.external_update(root);
                            }
                        });
                        root.start();
                        model.imp().external.replace(Some(root));
                    }
                    drop(model);
                    loop {
                        enum Event {
                            Owner(Option<zbus::fdo::NameOwnerChanged>),
                            Item(Option<(String, String)>),
                        }
                        let event = if own {
                            futures_lite::future::race(
                                async { Event::Owner(owners.next().await) },
                                async { Event::Item(receiver.recv().await.ok()) },
                            )
                            .await
                        } else {
                            Event::Owner(owners.next().await)
                        };
                        let Some(model) = weak.upgrade() else {
                            return Ok(());
                        };
                        match event {
                            Event::Item(Some((owner, path))) => {
                                model.attach(format!("{owner}{path}"), owner, path)
                            }
                            Event::Item(None) | Event::Owner(None) => return Ok(()),
                            Event::Owner(Some(signal)) => {
                                let args = signal.args()?;
                                if args.new_owner().is_none() {
                                    let owner = args.name().as_str();
                                    model.remove_owner(owner);
                                    if own {
                                        let removed = {
                                            let mut shared = shared.lock().map_err(|_| {
                                                zbus::Error::Failure(gettext(
                                                    "Tray state is unavailable.",
                                                ))
                                            })?;
                                            shared.hosts.remove(owner);
                                            let removed = shared
                                                .items
                                                .iter()
                                                .filter(|id| {
                                                    id.strip_prefix(owner)
                                                        .is_some_and(|rest| rest.starts_with('/'))
                                                })
                                                .cloned()
                                                .collect::<Vec<_>>();
                                            for id in &removed {
                                                shared.items.remove(id);
                                            }
                                            removed
                                        };
                                        for id in removed {
                                            connection
                                                .emit_signal(
                                                    None::<&str>,
                                                    PATH,
                                                    NAME,
                                                    "StatusNotifierItemUnregistered",
                                                    &(id,),
                                                )
                                                .await?;
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
                .await;
                if let Some(model) = weak.upgrade() {
                    model.imp().available.set(false);
                    model.notify_available();
                    if let Err(error) = result {
                        tracing::warn!(%error,"Tray watcher/host stopped");
                    }
                    model.imp().items.borrow_mut().clear();
                    model.changed();
                }
            })));
    }
    fn item(&self, id: &str) -> Result<Item, ServiceError> {
        self.imp()
            .items
            .borrow()
            .get(id)
            .filter(|item| item.properties.available())
            .cloned()
            .ok_or_else(|| gettext("The tray item is no longer available.").into())
    }
    pub async fn invoke(&self, id: &str, method: &str, x: i32, y: i32) -> Result<(), ServiceError> {
        if !["Activate", "SecondaryActivate", "ContextMenu"].contains(&method) {
            return Err(gettext("Invalid tray action.").into());
        }
        let item = self.item(id)?;
        let connection = self
            .imp()
            .connection
            .borrow()
            .clone()
            .ok_or_else(|| gettext("The tray service is unavailable."))?;
        let proxy = zbus::Proxy::new_owned(connection, item.owner, item.path, ITEM.to_owned())
            .await
            .map_err(|e| ServiceError(e.to_string()))?;
        proxy
            .call::<_, _, ()>(method, &(x, y))
            .await
            .map_err(|e| ServiceError(e.to_string()))
    }
    pub fn menu_path(&self, id: &str) -> Option<String> {
        self.item(id)
            .ok()?
            .properties
            .with_values(|values| {
                values
                    .get("Menu")
                    .and_then(|value| <&zbus::zvariant::ObjectPath>::try_from(value).ok())
                    .map(ToString::to_string)
            })
            .filter(|path| path != "/")
    }
    pub async fn menu(&self, id: &str) -> Result<dbusmenu::Node, ServiceError> {
        let item = self.item(id)?;
        let path = self
            .menu_path(id)
            .ok_or_else(|| gettext("This tray item has no menu."))?;
        let connection = self
            .imp()
            .connection
            .borrow()
            .clone()
            .ok_or_else(|| gettext("The tray service is unavailable."))?;
        let proxy = zbus::Proxy::new_owned(
            connection,
            item.owner,
            path,
            "com.canonical.dbusmenu".to_owned(),
        )
        .await
        .map_err(|e| ServiceError(e.to_string()))?;
        let _: Result<bool, _> = proxy.call("AboutToShow", &(0_i32,)).await;
        let (_, layout): (u32, dbusmenu::Layout) = proxy
            .call("GetLayout", &(0_i32, 4_i32, Vec::<String>::new()))
            .await
            .map_err(|e| ServiceError(e.to_string()))?;
        Ok(dbusmenu::parse(layout))
    }
    pub async fn menu_event(
        &self,
        id: &str,
        node: i32,
        timestamp: u32,
    ) -> Result<(), ServiceError> {
        let item = self.item(id)?;
        let path = self
            .menu_path(id)
            .ok_or_else(|| gettext("This tray item has no menu."))?;
        let connection = self
            .imp()
            .connection
            .borrow()
            .clone()
            .ok_or_else(|| gettext("The tray service is unavailable."))?;
        let proxy = zbus::Proxy::new_owned(
            connection,
            item.owner,
            path,
            "com.canonical.dbusmenu".to_owned(),
        )
        .await
        .map_err(|e| ServiceError(e.to_string()))?;
        proxy
            .call::<_, _, ()>("Event", &(node, "clicked", Value::from(0_i32), timestamp))
            .await
            .map_err(|e| ServiceError(e.to_string()))
    }
    pub async fn watch_menu(
        &self,
        id: &str,
        updates: async_channel::Sender<dbusmenu::Node>,
    ) -> Result<(), ServiceError> {
        let item = self.item(id)?;
        let path = self
            .menu_path(id)
            .ok_or_else(|| gettext("This tray item has no menu."))?;
        let connection = self
            .imp()
            .connection
            .borrow()
            .clone()
            .ok_or_else(|| gettext("The tray service is unavailable."))?;
        let proxy = zbus::Proxy::new_owned(
            connection,
            item.owner,
            path,
            "com.canonical.dbusmenu".to_owned(),
        )
        .await
        .map_err(|e| ServiceError(e.to_string()))?;
        let mut signals = proxy
            .receive_all_signals()
            .await
            .map_err(|e| ServiceError(e.to_string()))?;
        let initial = self.menu(id).await?;
        if updates.send(initial).await.is_err() {
            return Ok(());
        }
        while let Some(signal) = signals.next().await {
            let header = signal.header();
            let name = header.member().map(|v| v.as_str()).unwrap_or("");
            if name == "LayoutUpdated" || name == "ItemsPropertiesUpdated" {
                let node = self.menu(id).await?;
                if updates.send(node).await.is_err() {
                    break;
                }
            }
        }
        Ok(())
    }
}
