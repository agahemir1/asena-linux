use gtk::prelude::*;
use gtk::subclass::prelude::*;
mod imp {
    use super::*;
    #[derive(Default, gtk::CompositeTemplate)]
    #[template(resource = "/org/asenalinux/ui/greeter.ui")]
    pub struct Content {
        #[template_child]
        pub avatar: TemplateChild<adw::Avatar>,
        #[template_child]
        pub users: TemplateChild<gtk::DropDown>,
        #[template_child]
        pub username: TemplateChild<gtk::Entry>,
        #[template_child]
        pub sessions: TemplateChild<gtk::DropDown>,
        #[template_child]
        pub prompt: TemplateChild<gtk::Label>,
        #[template_child]
        pub password: TemplateChild<gtk::PasswordEntry>,
        #[template_child]
        pub answer: TemplateChild<gtk::Entry>,
        #[template_child]
        pub keyboard: TemplateChild<gtk::Label>,
        #[template_child]
        pub caps: TemplateChild<gtk::Label>,
        #[template_child]
        pub message: TemplateChild<gtk::Label>,
        #[template_child]
        pub cancel: TemplateChild<gtk::Button>,
        #[template_child]
        pub submit: TemplateChild<gtk::Button>,
        #[template_child]
        pub power: TemplateChild<gtk::Box>,
        #[template_child]
        pub logo: TemplateChild<gtk::Image>,
    }
    #[glib::object_subclass]
    impl ObjectSubclass for Content {
        const NAME: &'static str = "AsenaGreeterContent";
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
        fn constructed(&self) {
            self.parent_constructed();
            // The logo comes from the system icon theme; without it the image stays hidden.
            let display = self.obj().display();
            if !gtk::IconTheme::for_display(&display).has_icon("asena-logo") {
                self.logo.set_visible(false);
            }
        }
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
