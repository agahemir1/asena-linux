use crate::property_model::{Bus, Endpoint, PropertyModel};
use futures_lite::StreamExt;
use glib::{prelude::*, subclass::prelude::*};
use std::cell::{Cell, RefCell};
mod imp {
    use super::*;
    #[derive(Default, glib::Properties)]
    #[properties(wrapper_type=super::UserModel)]
    pub struct UserModel {
        #[property(get)]
        pub available: Cell<bool>,
        #[property(get)]
        pub name: RefCell<String>,
        #[property(get)]
        pub icon_file: RefCell<String>,
        pub username: RefCell<String>,
        pub current: RefCell<Option<PropertyModel>>,
        pub task: RefCell<Option<glib::JoinHandle<()>>>,
    }
    #[glib::object_subclass]
    impl ObjectSubclass for UserModel {
        const NAME: &'static str = "AsenaUserModel";
        type Type = super::UserModel;
    }
    #[glib::derived_properties]
    impl ObjectImpl for UserModel {
        fn dispose(&self) {
            if let Some(task) = self.task.borrow_mut().take() {
                task.abort();
            }
            self.current.borrow_mut().take();
        }
    }
}
glib::wrapper! {pub struct UserModel(ObjectSubclass<imp::UserModel>);}
impl Default for UserModel {
    fn default() -> Self {
        let model: Self = glib::Object::new();
        let name = glib::user_name().to_string_lossy().into_owned();
        model.imp().username.replace(name.clone());
        model.imp().name.replace(name);
        model
    }
}
impl UserModel {
    fn fallback(&self) {
        self.imp().available.set(false);
        self.notify_available();
        self.imp()
            .name
            .replace(self.imp().username.borrow().clone());
        self.notify_name();
        self.imp().icon_file.borrow_mut().clear();
        self.notify_icon_file();
    }
    fn refresh(&self, properties: &PropertyModel) {
        if self.imp().current.borrow().as_ref() != Some(properties) {
            return;
        }
        if !properties.available() {
            self.fallback();
            return;
        }
        self.imp().available.set(true);
        self.notify_available();
        self.imp().name.replace(
            properties
                .string("RealName")
                .filter(|name| !name.trim().is_empty())
                .unwrap_or_else(|| self.imp().username.borrow().clone()),
        );
        self.notify_name();
        self.imp()
            .icon_file
            .replace(properties.string("IconFile").unwrap_or_default());
        self.notify_icon_file();
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
                    let connection = zbus::Connection::system().await?;
                    let proxy = zbus::Proxy::new(
                        &connection,
                        "org.freedesktop.Accounts",
                        "/org/freedesktop/Accounts",
                        "org.freedesktop.Accounts",
                    )
                    .await?;
                    let mut owners = proxy.receive_owner_changed().await?;
                    loop {
                        let Some(model) = weak.upgrade() else {
                            return Ok(());
                        };
                        let username = model.imp().username.borrow().clone();
                        drop(model);
                        let path: Result<zbus::zvariant::OwnedObjectPath, _> =
                            proxy.call("FindUserByName", &(username,)).await;
                        if let Some(model) = weak.upgrade() {
                            model.imp().current.borrow_mut().take();
                            match path {
                                Ok(path) => {
                                    let properties = PropertyModel::new(Endpoint {
                                        bus: Bus::System,
                                        name: "org.freedesktop.Accounts".into(),
                                        path: path.to_string(),
                                        interface: "org.freedesktop.Accounts.User".into(),
                                    });
                                    let weak = model.downgrade();
                                    properties.connect_changed(move |properties| {
                                        if let Some(model) = weak.upgrade() {
                                            model.refresh(properties);
                                        }
                                    });
                                    model.imp().current.replace(Some(properties.clone()));
                                    properties.start();
                                }
                                Err(_) => model.fallback(),
                            }
                        }
                        if owners.next().await.is_none() {
                            return Ok(());
                        }
                    }
                }
                .await;
                if let Some(model) = weak.upgrade() {
                    model.imp().current.borrow_mut().take();
                    model.fallback();
                    if let Err(error) = result {
                        tracing::debug!(%error,"AccountsService is unavailable");
                    }
                }
            })));
    }
}
