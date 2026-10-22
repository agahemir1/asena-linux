use gtk::subclass::prelude::*;
mod imp {
    use super::*;
    #[derive(Default, gtk::CompositeTemplate)]
    #[template(resource = "/org/asenalinux/ui/settings.ui")]
    pub struct Content {
        #[template_child]
        pub split: TemplateChild<adw::NavigationSplitView>,
        #[template_child]
        pub sidebar: TemplateChild<adw::NavigationPage>,
        #[template_child]
        pub page: TemplateChild<adw::NavigationPage>,
        #[template_child]
        pub categories: TemplateChild<gtk::ListBox>,
        #[template_child]
        pub search: TemplateChild<gtk::SearchEntry>,
        #[template_child]
        pub stack: TemplateChild<gtk::Stack>,
        #[template_child]
        pub error: TemplateChild<gtk::Label>,
    }
    #[glib::object_subclass]
    impl ObjectSubclass for Content {
        const NAME: &'static str = "AsenaSettingsContent";
        type Type = super::Content;
        type ParentType = gtk::Box;
        fn class_init(class: &mut Self::Class) {
            class.bind_template();
        }
        fn instance_init(object: &glib::subclass::InitializingObject<Self>) {
            object.init_template();
        }
    }
    impl ObjectImpl for Content {
        fn dispose(&self) {
            self.dispose_template();
        }
    }
    impl WidgetImpl for Content {}
    impl BoxImpl for Content {}
}
glib::wrapper! {pub struct Content(ObjectSubclass<imp::Content>) @extends gtk::Widget,gtk::Box,@implements gtk::Accessible,gtk::Buildable,gtk::ConstraintTarget,gtk::Orientable;}
impl Default for Content {
    fn default() -> Self {
        glib::Object::new()
    }
}
impl Content {
    pub fn widgets(&self) -> &imp::Content {
        self.imp()
    }
}
