use gtk::{prelude::*, subclass::prelude::*};
use std::cell::OnceCell;

mod imp {
    use super::*;
    #[derive(Default)]
    pub struct TaskbarStrip {
        pub content: OnceCell<gtk::Box>,
        pub more: OnceCell<gtk::MenuButton>,
    }
    #[glib::object_subclass]
    impl ObjectSubclass for TaskbarStrip {
        const NAME: &'static str = "AsenaTaskbarStrip";
        type Type = super::TaskbarStrip;
        type ParentType = gtk::Box;
    }
    impl ObjectImpl for TaskbarStrip {}
    impl WidgetImpl for TaskbarStrip {
        fn size_allocate(&self, width: i32, height: i32, baseline: i32) {
            self.parent_size_allocate(width, height, baseline);
            if let (Some(content), Some(more)) = (self.content.get(), self.more.get()) {
                let axis = content.orientation();
                let (_, natural, _, _) = content.measure(axis, -1);
                more.set_visible(
                    natural
                        > if axis == gtk::Orientation::Horizontal {
                            width
                        } else {
                            height
                        },
                );
            }
        }
    }
    impl BoxImpl for TaskbarStrip {}
}
glib::wrapper! {
    pub struct TaskbarStrip(ObjectSubclass<imp::TaskbarStrip>)
        @extends gtk::Widget, gtk::Box,
        @implements gtk::Accessible, gtk::Buildable, gtk::ConstraintTarget, gtk::Orientable;
}
impl TaskbarStrip {
    pub fn new(content: &gtk::Box, scroll: &gtk::ScrolledWindow, more: &gtk::MenuButton) -> Self {
        let strip: Self = glib::Object::new();
        strip.set_orientation(content.orientation());
        strip.set_hexpand(content.orientation() == gtk::Orientation::Horizontal);
        strip.set_vexpand(content.orientation() == gtk::Orientation::Vertical);
        strip.append(scroll);
        strip.append(more);
        let _ = strip.imp().content.set(content.clone());
        let _ = strip.imp().more.set(more.clone());
        strip
    }
}
