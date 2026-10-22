use crate::toplevel::Toplevel;
use asena_wayland::toplevel::{self, Event, Request};
use gio::prelude::*;
use gio::subclass::prelude::*;
use glib::subclass::Signal;
use std::{
    cell::{Cell, RefCell},
    sync::OnceLock,
};

mod imp {
    use super::*;
    #[derive(Default, glib::Properties)]
    #[properties(wrapper_type = super::ToplevelModel)]
    pub struct ToplevelModel {
        pub windows: RefCell<Vec<super::Toplevel>>,
        #[property(get)]
        pub available: Cell<bool>,
        pub requests: RefCell<Option<async_channel::Sender<Request>>>,
        pub task: RefCell<Option<glib::JoinHandle<()>>>,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for ToplevelModel {
        const NAME: &'static str = "AsenaToplevelModel";
        type Type = super::ToplevelModel;
        type Interfaces = (gio::ListModel,);
    }
    #[glib::derived_properties]
    impl ObjectImpl for ToplevelModel {
        fn signals() -> &'static [Signal] {
            static SIGNALS: OnceLock<Vec<Signal>> = OnceLock::new();
            SIGNALS.get_or_init(|| vec![Signal::builder("changed").build()])
        }
        fn dispose(&self) {
            if let Some(task) = self.task.borrow_mut().take() {
                task.abort();
            }
            if let Some(requests) = self.requests.borrow_mut().take() {
                requests.close();
            }
        }
    }
    impl ListModelImpl for ToplevelModel {
        fn item_type(&self) -> glib::Type {
            super::Toplevel::static_type()
        }
        fn n_items(&self) -> u32 {
            self.windows.borrow().len() as u32
        }
        fn item(&self, position: u32) -> Option<glib::Object> {
            self.windows
                .borrow()
                .get(position as usize)
                .cloned()
                .map(Cast::upcast)
        }
    }
}

glib::wrapper! { pub struct ToplevelModel(ObjectSubclass<imp::ToplevelModel>) @implements gio::ListModel; }

impl Default for ToplevelModel {
    fn default() -> Self {
        glib::Object::new()
    }
}

impl ToplevelModel {
    pub fn start(&self) -> std::io::Result<()> {
        if self.imp().task.borrow().is_some() {
            return Ok(());
        }
        let bridge = toplevel::start()?;
        self.imp().requests.replace(Some(bridge.requests.clone()));
        let weak = self.downgrade();
        let task = glib::spawn_future_local(async move {
            while let Ok(event) = bridge.events.recv().await {
                let Some(model) = weak.upgrade() else {
                    break;
                };
                model.apply(event);
            }
        });
        self.imp().task.replace(Some(task));
        Ok(())
    }

    pub fn request(&self, request: Request) {
        if let Some(sender) = self.imp().requests.borrow().as_ref() {
            let _ = sender.try_send(request);
        }
    }

    pub fn windows(&self) -> Vec<Toplevel> {
        self.imp().windows.borrow().clone()
    }

    pub fn apply(&self, event: Event) {
        let imp = self.imp();
        match event {
            Event::Available(available) => {
                imp.available.set(available);
                self.notify("available");
                if !available {
                    let count = imp.windows.borrow().len() as u32;
                    imp.windows.borrow_mut().clear();
                    if count > 0 {
                        self.items_changed(0, count, 0);
                    }
                }
            }
            Event::Updated(snapshot) => {
                let existing = imp
                    .windows
                    .borrow()
                    .iter()
                    .find(|window| window.id() == snapshot.id)
                    .cloned();
                if let Some(window) = existing {
                    window.apply(&snapshot);
                } else {
                    let window: Toplevel = glib::Object::new();
                    window.apply(&snapshot);
                    let position = imp.windows.borrow().len() as u32;
                    imp.windows.borrow_mut().push(window);
                    self.items_changed(position, 0, 1);
                }
            }
            Event::Closed(id) => {
                let position = imp
                    .windows
                    .borrow()
                    .iter()
                    .position(|window| window.id() == id);
                if let Some(position) = position {
                    imp.windows.borrow_mut().remove(position);
                    self.items_changed(position as u32, 1, 0);
                }
            }
        }
        self.emit_by_name::<()>("changed", &[]);
    }

    pub fn connect_changed(&self, callback: impl Fn(&Self) + 'static) -> glib::SignalHandlerId {
        self.connect_local("changed", false, move |values| {
            if let Ok(model) = values[0].get::<Self>() {
                callback(&model);
            }
            None
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use asena_wayland::toplevel::Snapshot;
    #[test]
    #[allow(clippy::unwrap_used)]
    fn updates_preserve_object_identity_and_closed_disconnect_clears_model() {
        let model = ToplevelModel::default();
        model.apply(Event::Available(true));
        let mut snapshot = Snapshot {
            id: 1,
            title: "First".into(),
            ..Snapshot::default()
        };
        model.apply(Event::Updated(snapshot.clone()));
        let first = model.item(0).unwrap();
        snapshot.title = "Changed".into();
        snapshot.activated = true;
        model.apply(Event::Updated(snapshot));
        assert_eq!(model.item(0).unwrap(), first);
        assert_eq!(model.windows()[0].title(), "Changed");
        assert!(model.windows()[0].activated());
        model.apply(Event::Updated(Snapshot {
            id: 2,
            ..Snapshot::default()
        }));
        model.apply(Event::Closed(1));
        assert_eq!(model.n_items(), 1);
        assert_eq!(model.windows()[0].id(), 2);
        model.apply(Event::Closed(1));
        model.apply(Event::Available(false));
        assert_eq!(model.n_items(), 0);
        assert!(!model.available());
    }
}
