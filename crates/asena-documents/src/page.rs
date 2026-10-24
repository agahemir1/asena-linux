//! One document page: a fixed logical size and the image `pdftoppm` rendered
//! for it, which may have more pixels than the size on high density screens.
use gtk::{gdk, glib, graphene, prelude::*, subclass::prelude::*};
use std::cell::{Cell, RefCell};

mod imp {
    use super::*;

    #[derive(Default)]
    pub struct Page {
        pub texture: RefCell<Option<gdk::Texture>>,
        pub size: Cell<(i32, i32)>,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for Page {
        const NAME: &'static str = "AsenaDocumentsPage";
        type Type = super::Page;
        type ParentType = gtk::Widget;

        fn class_init(class: &mut Self::Class) {
            class.set_css_name("asena-page");
            class.set_accessible_role(gtk::AccessibleRole::Img);
        }
    }

    impl ObjectImpl for Page {}

    impl WidgetImpl for Page {
        fn measure(&self, orientation: gtk::Orientation, _for_size: i32) -> (i32, i32, i32, i32) {
            let (width, height) = self.size.get();
            let size = if orientation == gtk::Orientation::Horizontal {
                width
            } else {
                height
            };
            (size, size, -1, -1)
        }

        fn snapshot(&self, snapshot: &gtk::Snapshot) {
            if let Some(texture) = self.texture.borrow().as_ref() {
                let widget = self.obj();
                snapshot.append_texture(
                    texture,
                    &graphene::Rect::new(0.0, 0.0, widget.width() as f32, widget.height() as f32),
                );
            }
        }
    }
}

glib::wrapper! {
    pub struct Page(ObjectSubclass<imp::Page>)
        @extends gtk::Widget,
        @implements gtk::Accessible, gtk::Buildable, gtk::ConstraintTarget;
}

impl Default for Page {
    fn default() -> Self {
        glib::Object::new()
    }
}

impl Page {
    pub fn set_size(&self, width: i32, height: i32) {
        self.imp().size.set((width.max(1), height.max(1)));
        self.queue_resize();
    }

    pub fn set_texture(&self, texture: Option<gdk::Texture>) {
        self.imp().texture.replace(texture);
        self.queue_draw();
    }
}
