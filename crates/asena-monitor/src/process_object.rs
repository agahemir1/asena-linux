//! One row of the process list; a refresh updates these in place so the
//! list keeps its selection and scroll position.
use gtk::{glib, prelude::*, subclass::prelude::*};
use std::cell::{Cell, RefCell};

mod imp {
    use super::*;

    #[derive(Default, glib::Properties)]
    #[properties(wrapper_type = super::ProcessObject)]
    pub struct ProcessObject {
        #[property(get, construct_only)]
        pid: Cell<u32>,
        #[property(get, set)]
        name: RefCell<String>,
        #[property(get, set)]
        command: RefCell<String>,
        #[property(get, set)]
        user: RefCell<String>,
        #[property(get, set)]
        own: Cell<bool>,
        /// Percent of all processors since the previous refresh.
        #[property(get, set)]
        cpu: Cell<f64>,
        /// Resident memory in bytes.
        #[property(get, set)]
        memory: Cell<u64>,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for ProcessObject {
        const NAME: &'static str = "AsenaMonitorProcess";
        type Type = super::ProcessObject;
    }

    #[glib::derived_properties]
    impl ObjectImpl for ProcessObject {}
}

glib::wrapper! {
    pub struct ProcessObject(ObjectSubclass<imp::ProcessObject>);
}

impl ProcessObject {
    pub fn new(pid: u32) -> Self {
        glib::Object::builder().property("pid", pid).build()
    }
}
