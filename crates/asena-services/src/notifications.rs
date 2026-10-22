//! Async freedesktop server with a GTK-independent main-context model.
use asena_core::{
    i18n::gettext,
    notification::{self, History, Notification},
};
use glib::{
    prelude::*,
    subclass::{prelude::*, Signal},
};
use std::{
    cell::{Cell, RefCell},
    collections::{BTreeMap, HashMap},
    sync::OnceLock,
};
use zbus::zvariant::OwnedValue;

const PATH: &str = "/org/freedesktop/Notifications";
const INTERFACE: &str = "org.freedesktop.Notifications";
enum Incoming {
    Notify {
        item: Box<Notification>,
        replaces: u32,
        reply: async_channel::Sender<zbus::fdo::Result<u32>>,
    },
    Close {
        id: u32,
        reply: async_channel::Sender<zbus::fdo::Result<()>>,
    },
}
struct Server {
    sender: async_channel::Sender<Incoming>,
}
#[zbus::interface(name = "org.freedesktop.Notifications")]
impl Server {
    fn get_capabilities(&self) -> Vec<&str> {
        vec![
            "actions",
            "body",
            "body-markup",
            "body-hyperlinks",
            "icon-static",
            "persistence",
        ]
    }
    fn get_server_information(&self) -> (&str, &str, &str, &str) {
        ("Asena", "Asena Linux", env!("CARGO_PKG_VERSION"), "1.2")
    }
    #[allow(clippy::too_many_arguments)]
    async fn notify(
        &self,
        app_name: &str,
        replaces_id: u32,
        app_icon: &str,
        summary: &str,
        body: &str,
        actions: Vec<String>,
        hints: HashMap<String, OwnedValue>,
        expire_timeout: i32,
    ) -> zbus::fdo::Result<u32> {
        if summary.len() > 16_384
            || body.len() > 65_536
            || app_name.len() > 4096
            || app_icon.len() > 4096
            || actions.len() > 64
            || actions.iter().any(|s| s.len() > 4096)
            || actions.len() % 2 != 0
        {
            return Err(zbus::fdo::Error::InvalidArgs(gettext(
                "The notification is too large or malformed.",
            )));
        }
        let boolean = |key: &str| {
            hints
                .get(key)
                .and_then(|v| bool::try_from(v).ok())
                .unwrap_or(false)
        };
        let string = |key: &str| {
            hints
                .get(key)
                .and_then(|v| <&str>::try_from(v).ok())
                .filter(|s| s.len() <= 4096)
                .map(ToOwned::to_owned)
        };
        let critical = hints.get("urgency").and_then(|v| u8::try_from(v).ok()) == Some(2);
        let image = hints
            .get("image-data")
            .or_else(|| hints.get("image_data"))
            .and_then(|v| v.try_clone().ok())
            .and_then(|v| <(i32, i32, i32, bool, i32, i32, Vec<u8>)>::try_from(v).ok())
            .and_then(|(w, h, s, a, b, c, d)| notification::image_data(w, h, s, a, b, c, &d));
        let item = Notification {
            id: 0,
            app: app_name.into(),
            icon: string("image-path").unwrap_or_else(|| app_icon.into()),
            summary: summary.into(),
            body: notification::sanitize_markup(body),
            actions: actions
                .chunks_exact(2)
                .map(|pair| (pair[0].clone(), pair[1].clone()))
                .collect(),
            critical,
            transient: boolean("transient"),
            resident: boolean("resident"),
            desktop_entry: string("desktop-entry"),
            category: string("category"),
            image,
            timeout_ms: if critical || expire_timeout == 0 {
                None
            } else {
                Some(if expire_timeout < 0 {
                    5000
                } else {
                    expire_timeout as u32
                })
            },
            unread: true,
        };
        let (reply, response) = async_channel::bounded(1);
        self.sender
            .send(Incoming::Notify {
                item: Box::new(item),
                replaces: replaces_id,
                reply,
            })
            .await
            .map_err(|_| zbus::fdo::Error::Failed(gettext("The shell is shutting down.")))?;
        response
            .recv()
            .await
            .map_err(|_| zbus::fdo::Error::Failed(gettext("The shell is shutting down.")))?
    }
    async fn close_notification(&self, id: u32) -> zbus::fdo::Result<()> {
        let (reply, response) = async_channel::bounded(1);
        self.sender
            .send(Incoming::Close { id, reply })
            .await
            .map_err(|_| zbus::fdo::Error::Failed(gettext("The shell is shutting down.")))?;
        response
            .recv()
            .await
            .map_err(|_| zbus::fdo::Error::Failed(gettext("The shell is shutting down.")))?
    }
    #[zbus(signal)]
    async fn notification_closed(
        context: &zbus::object_server::SignalEmitter<'_>,
        id: u32,
        reason: u32,
    ) -> zbus::Result<()>;
    #[zbus(signal)]
    async fn action_invoked(
        context: &zbus::object_server::SignalEmitter<'_>,
        id: u32,
        action_key: &str,
    ) -> zbus::Result<()>;
    #[zbus(signal)]
    async fn activation_token(
        context: &zbus::object_server::SignalEmitter<'_>,
        id: u32,
        activation_token: &str,
    ) -> zbus::Result<()>;
}
mod imp {
    use super::*;
    #[derive(Default, glib::Properties)]
    #[properties(wrapper_type = super::NotificationModel)]
    pub struct NotificationModel {
        pub history: RefCell<History>,
        pub blocked: RefCell<std::collections::BTreeSet<String>>,
        pub active: RefCell<BTreeMap<u32, Notification>>,
        pub connection: RefCell<Option<zbus::Connection>>,
        #[property(get)]
        pub available: Cell<bool>,
        #[property(get)]
        pub unread: Cell<u32>,
    }
    #[glib::object_subclass]
    impl ObjectSubclass for NotificationModel {
        const NAME: &'static str = "AsenaNotificationModel";
        type Type = super::NotificationModel;
    }
    #[glib::derived_properties]
    impl ObjectImpl for NotificationModel {
        fn signals() -> &'static [Signal] {
            static SIGNALS: OnceLock<Vec<Signal>> = OnceLock::new();
            SIGNALS.get_or_init(|| {
                vec![
                    Signal::builder("changed").build(),
                    Signal::builder("received")
                        .param_types([u32::static_type()])
                        .build(),
                ]
            })
        }
    }
}
glib::wrapper! { pub struct NotificationModel(ObjectSubclass<imp::NotificationModel>); }
impl Default for NotificationModel {
    fn default() -> Self {
        glib::Object::new()
    }
}
impl NotificationModel {
    pub fn set_blocked(&self, applications: impl IntoIterator<Item = String>) {
        self.imp()
            .blocked
            .replace(applications.into_iter().collect());
    }
    pub fn start(&self) {
        let weak = self.downgrade();
        glib::spawn_future_local(async move {
            let (sender, receiver) = async_channel::bounded(64);
            let connection = match async {
                let connection = zbus::connection::Builder::session()?
                    .serve_at(PATH, Server { sender })?
                    .build()
                    .await?;
                asena_core::dbus::claim_name(&connection, INTERFACE).await?;
                Ok::<_, zbus::Error>(connection)
            }
            .await
            {
                Ok(connection) => connection,
                Err(error) => {
                    tracing::warn!(%error, "Notification server unavailable; another server may own the name");
                    return;
                }
            };
            let Some(model) = weak.upgrade() else {
                return;
            };
            model.imp().connection.replace(Some(connection));
            model.imp().available.set(true);
            model.notify_available();
            drop(model);
            while let Ok(command) = receiver.recv().await {
                let Some(model) = weak.upgrade() else {
                    break;
                };
                match command {
                    Incoming::Notify {
                        mut item,
                        replaces,
                        reply,
                    } => {
                        let id = if model.imp().active.borrow().contains_key(&replaces) {
                            Some(replaces)
                        } else {
                            model.imp().history.borrow_mut().allocate()
                        };
                        let Some(id) = id else {
                            let _ = reply
                                .send(Err(zbus::fdo::Error::Failed(gettext(
                                    "No notification identifiers remain.",
                                ))))
                                .await;
                            continue;
                        };
                        item.id = id;
                        let app_key =
                            notification::application_key(item.desktop_entry.as_deref(), &item.app);
                        if model.imp().blocked.borrow().contains(&app_key) {
                            let _ = reply.send(Ok(id)).await;
                            if let Some(connection) = model.imp().connection.borrow().clone() {
                                glib::spawn_future_local(async move {
                                    let _ = connection
                                        .emit_signal(
                                            None::<&str>,
                                            PATH,
                                            INTERFACE,
                                            "NotificationClosed",
                                            &(id, 4_u32),
                                        )
                                        .await;
                                });
                            }
                            continue;
                        }
                        if model.imp().active.borrow().len() >= 100
                            && !model.imp().active.borrow().contains_key(&id)
                        {
                            let oldest = model.imp().active.borrow().keys().next().copied();
                            if let Some(oldest) = oldest {
                                model.close(oldest, 4);
                            }
                        }
                        model.imp().history.borrow_mut().insert((*item).clone());
                        model.imp().active.borrow_mut().insert(id, *item);
                        model.changed();
                        model.emit_by_name::<()>("received", &[&id]);
                        let _ = reply.send(Ok(id)).await;
                    }
                    Incoming::Close { id, reply } => {
                        let exists = model.imp().active.borrow().contains_key(&id);
                        if exists {
                            model.close(id, 3);
                        }
                        let _ = reply
                            .send(if exists {
                                Ok(())
                            } else {
                                Err(zbus::fdo::Error::InvalidArgs(gettext(
                                    "The notification no longer exists.",
                                )))
                            })
                            .await;
                    }
                }
            }
        });
    }
    fn changed(&self) {
        let unread = self
            .imp()
            .history
            .borrow()
            .items
            .iter()
            .filter(|n| n.unread)
            .count() as u32;
        if self.imp().unread.replace(unread) != unread {
            self.notify_unread();
        }
        self.emit_by_name::<()>("changed", &[]);
    }
    pub fn items(&self) -> Vec<Notification> {
        self.imp().history.borrow().items.iter().cloned().collect()
    }
    pub fn active(&self, id: u32) -> Option<Notification> {
        self.imp().active.borrow().get(&id).cloned()
    }
    pub fn mark_read(&self) {
        for item in &mut self.imp().history.borrow_mut().items {
            item.unread = false;
        }
        self.changed();
    }
    pub fn clear(&self) {
        let ids: Vec<_> = self.imp().active.borrow().keys().copied().collect();
        for id in ids {
            self.close(id, 2);
        }
        self.imp().history.borrow_mut().items.clear();
        self.changed();
    }
    pub fn close(&self, id: u32, reason: u32) {
        let existed = self.imp().active.borrow_mut().remove(&id).is_some();
        if reason != 1 {
            self.imp().history.borrow_mut().remove(id);
        }
        if existed {
            if let Some(connection) = self.imp().connection.borrow().clone() {
                glib::spawn_future_local(async move {
                    if let Err(error) = connection
                        .emit_signal(
                            None::<&str>,
                            PATH,
                            INTERFACE,
                            "NotificationClosed",
                            &(id, reason),
                        )
                        .await
                    {
                        tracing::warn!(%error,"Could not emit notification closure");
                    }
                });
            }
        }
        self.changed();
    }
    pub fn invoke(&self, id: u32, key: &str, token: Option<String>) {
        let Some(item) = self.active(id) else {
            return;
        };
        if !item.actions.iter().any(|(action, _)| action == key) {
            return;
        }
        let Some(connection) = self.imp().connection.borrow().clone() else {
            return;
        };
        let model = self.clone();
        let key = key.to_owned();
        glib::spawn_future_local(async move {
            if let Some(token) = token {
                let _ = connection
                    .emit_signal(
                        None::<&str>,
                        PATH,
                        INTERFACE,
                        "ActivationToken",
                        &(id, token),
                    )
                    .await;
            }
            if let Err(error) = connection
                .emit_signal(None::<&str>, PATH, INTERFACE, "ActionInvoked", &(id, key))
                .await
            {
                tracing::warn!(%error,"Could not emit notification action");
            }
            if !item.resident {
                model.close(id, 2);
            }
        });
    }
    pub fn connect_changed(&self, callback: impl Fn(&Self) + 'static) -> glib::SignalHandlerId {
        self.connect_local("changed", false, move |values| {
            if let Ok(model) = values[0].get::<Self>() {
                callback(&model);
            }
            None
        })
    }
    pub fn connect_received(
        &self,
        callback: impl Fn(&Self, u32) + 'static,
    ) -> glib::SignalHandlerId {
        self.connect_local("received", false, move |values| {
            if let (Ok(model), Ok(id)) = (values[0].get::<Self>(), values[1].get::<u32>()) {
                callback(&model, id);
            }
            None
        })
    }
}
