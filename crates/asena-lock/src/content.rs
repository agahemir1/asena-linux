use gtk::prelude::*;
use gtk::subclass::prelude::*;
use std::cell::RefCell;
mod imp {
    use super::*;
    #[derive(Default, gtk::CompositeTemplate)]
    #[template(resource = "/org/asenalinux/ui/lock.ui")]
    pub struct Content {
        #[template_child]
        pub clock: TemplateChild<gtk::Label>,
        #[template_child]
        pub date: TemplateChild<gtk::Label>,
        #[template_child]
        pub avatar: TemplateChild<adw::Avatar>,
        #[template_child]
        pub name: TemplateChild<gtk::Label>,
        #[template_child]
        pub authentication: TemplateChild<gtk::Box>,
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
        pub logo: TemplateChild<gtk::Image>,
        pub device: RefCell<Option<(gtk::gdk::Device, Vec<glib::SignalHandlerId>)>>,
    }
    #[glib::object_subclass]
    impl ObjectSubclass for Content {
        const NAME: &'static str = "AsenaLockContent";
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
            if let Some((device, handlers)) = self.device.borrow_mut().take() {
                for handler in handlers {
                    device.disconnect(handler);
                }
            }
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
    pub fn keyboard(&self) {
        let Some(seat) = gtk::gdk::Display::default().and_then(|display| display.default_seat())
        else {
            return;
        };
        let Some(device) = seat.keyboard() else {
            self.imp().keyboard.set_visible(false);
            self.imp().caps.set_visible(false);
            return;
        };
        if self
            .imp()
            .device
            .borrow()
            .as_ref()
            .is_some_and(|(current, _)| current == &device)
        {
            self.refresh_keyboard(&device);
            return;
        }
        if let Some((old, handlers)) = self.imp().device.borrow_mut().take() {
            for handler in handlers {
                old.disconnect(handler);
            }
        }
        let mut handlers = Vec::new();
        let weak = self.downgrade();
        handlers.push(device.connect_active_layout_index_notify(move |device| {
            if let Some(view) = weak.upgrade() {
                view.refresh_keyboard(device);
            }
        }));
        let weak = self.downgrade();
        handlers.push(device.connect_layout_names_notify(move |device| {
            if let Some(view) = weak.upgrade() {
                view.refresh_keyboard(device);
            }
        }));
        let weak = self.downgrade();
        handlers.push(device.connect_caps_lock_state_notify(move |device| {
            if let Some(view) = weak.upgrade() {
                view.refresh_keyboard(device);
            }
        }));
        self.imp().device.replace(Some((device.clone(), handlers)));
        self.refresh_keyboard(&device);
    }
    fn refresh_keyboard(&self, device: &gtk::gdk::Device) {
        let names = device.layout_names();
        let index = usize::try_from(device.active_layout_index()).ok();
        let name = index.and_then(|index| names.get(index));
        self.imp().keyboard.set_visible(name.is_some());
        self.imp()
            .keyboard
            .set_text(name.map(|name| name.as_str()).unwrap_or_default());
        self.imp()
            .caps
            .set_text(&asena_core::i18n::gettext("Caps Lock is on"));
        self.imp().caps.set_visible(device.is_caps_locked());
    }
}
