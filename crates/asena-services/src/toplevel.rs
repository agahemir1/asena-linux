use asena_wayland::toplevel::Snapshot;
use gio::prelude::*;
use gio::subclass::prelude::*;
use std::cell::{Cell, RefCell};

mod imp {
    use super::*;

    #[derive(Default, glib::Properties)]
    #[properties(wrapper_type = super::Toplevel)]
    pub struct Toplevel {
        #[property(get)]
        pub id: Cell<u64>,
        #[property(get)]
        pub title: RefCell<String>,
        #[property(get)]
        pub app_id: RefCell<String>,
        #[property(get)]
        pub activated: Cell<bool>,
        #[property(get)]
        pub minimized: Cell<bool>,
        #[property(get)]
        pub maximized: Cell<bool>,
        #[property(get)]
        pub fullscreen: Cell<bool>,
        #[property(get)]
        pub outputs: RefCell<glib::StrV>,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for Toplevel {
        const NAME: &'static str = "AsenaToplevel";
        type Type = super::Toplevel;
    }
    #[glib::derived_properties]
    impl ObjectImpl for Toplevel {}
}

glib::wrapper! { pub struct Toplevel(ObjectSubclass<imp::Toplevel>); }

impl Toplevel {
    pub(super) fn apply(&self, snapshot: &Snapshot) {
        let _freeze = self.freeze_notify();
        let imp = self.imp();
        imp.id.set(snapshot.id);
        imp.title.replace(snapshot.title.clone());
        imp.app_id.replace(snapshot.app_id.clone());
        imp.activated.set(snapshot.activated);
        imp.minimized.set(snapshot.minimized);
        imp.maximized.set(snapshot.maximized);
        imp.fullscreen.set(snapshot.fullscreen);
        imp.outputs.replace(
            snapshot
                .outputs
                .iter()
                .map(|name| glib::GString::from(name.as_str()))
                .collect(),
        );
        for property in [
            "id",
            "title",
            "app-id",
            "activated",
            "minimized",
            "maximized",
            "fullscreen",
            "outputs",
        ] {
            self.notify(property);
        }
    }
}
