use asena_core::apps::AppIndex;
use gio::prelude::*;
use gio::subclass::prelude::*;
use glib::subclass::Signal;
use std::{
    cell::{Cell, RefCell},
    sync::OnceLock,
};

mod imp {
    use super::*;
    #[derive(Default)]
    pub struct AppCatalog {
        pub index: RefCell<AppIndex>,
        pub generation: Cell<u64>,
        pub monitor: RefCell<Option<(gio::AppInfoMonitor, glib::SignalHandlerId)>>,
    }
    #[glib::object_subclass]
    impl ObjectSubclass for AppCatalog {
        const NAME: &'static str = "AsenaAppCatalog";
        type Type = super::AppCatalog;
    }
    impl ObjectImpl for AppCatalog {
        fn signals() -> &'static [Signal] {
            static SIGNALS: OnceLock<Vec<Signal>> = OnceLock::new();
            SIGNALS.get_or_init(|| vec![Signal::builder("changed").build()])
        }
        fn dispose(&self) {
            if let Some((monitor, handler)) = self.monitor.borrow_mut().take() {
                monitor.disconnect(handler);
            }
        }
    }
}
glib::wrapper! { pub struct AppCatalog(ObjectSubclass<imp::AppCatalog>); }
impl Default for AppCatalog {
    fn default() -> Self {
        glib::Object::new()
    }
}
impl AppCatalog {
    pub fn start(&self) {
        if self.imp().monitor.borrow().is_some() {
            return;
        }
        let monitor = gio::AppInfoMonitor::get();
        let weak = self.downgrade();
        let handler = monitor.connect_changed(move |_| {
            if let Some(catalog) = weak.upgrade() {
                catalog.reload();
            }
        });
        self.imp().monitor.replace(Some((monitor, handler)));
        self.reload();
    }
    fn reload(&self) {
        let generation = self.imp().generation.get().wrapping_add(1);
        self.imp().generation.set(generation);
        let weak = self.downgrade();
        glib::spawn_future_local(async move {
            match gio::spawn_blocking(AppIndex::load).await {
                Ok(index) => {
                    if let Some(catalog) = weak.upgrade() {
                        if catalog.imp().generation.get() == generation {
                            catalog.imp().index.replace(index);
                            catalog.emit_by_name::<()>("changed", &[]);
                        }
                    }
                }
                Err(error) => tracing::warn!(?error, "Application indexing failed"),
            }
        });
    }
    pub fn with_index<T>(&self, f: impl FnOnce(&AppIndex) -> T) -> T {
        f(&self.imp().index.borrow())
    }
    pub fn connect_changed(&self, callback: impl Fn(&Self) + 'static) -> glib::SignalHandlerId {
        self.connect_local("changed", false, move |values| {
            if let Ok(catalog) = values[0].get::<Self>() {
                callback(&catalog);
            }
            None
        })
    }
}
