//! Screenshot window: capture through the desktop portal, then copy or save.
use adw::prelude::*;
use asena_app::prelude::*;
use asena_core::screenshot;
use std::{cell::RefCell, path::PathBuf, rc::Rc, time::Duration};

struct State {
    window: AsenaWindow,
    stack: gtk::Stack,
    picture: gtk::Picture,
    delay: adw::SpinRow,
    select: adw::SwitchRow,
    capture: gtk::Button,
    last: RefCell<Option<PathBuf>>,
}

#[derive(Clone)]
pub struct CaptureWindow {
    state: Rc<State>,
}

impl CaptureWindow {
    pub fn new(app: &adw::Application) -> Self {
        let options = adw::PreferencesGroup::new();
        let delay = adw::SpinRow::with_range(0.0, 10.0, 1.0);
        delay.set_title(&gettext("Delay in seconds"));
        options.add(&delay);
        let select = adw::SwitchRow::builder()
            .title(gettext("Let me choose what to capture"))
            .build();
        options.add(&select);
        let capture = gtk::Button::with_label(&gettext("Take screenshot"));
        capture.add_css_class("pill");
        capture.add_css_class("suggested-action");
        capture.set_halign(gtk::Align::Center);
        capture.set_action_name(Some("win.capture"));

        let picture = gtk::Picture::new();
        picture.set_content_fit(gtk::ContentFit::Contain);
        picture.set_can_shrink(true);
        picture.set_vexpand(true);
        picture.add_css_class("card");
        let result_actions = gtk::Box::new(gtk::Orientation::Horizontal, 8);
        result_actions.set_halign(gtk::Align::Center);
        for (label, action) in [
            (gettext("Copy"), "win.copy"),
            (gettext("Show in Files"), "win.show"),
            (gettext("Save as…"), "win.save-as"),
        ] {
            let button = gtk::Button::with_label(&label);
            button.add_css_class("pill");
            button.set_action_name(Some(action));
            result_actions.append(&button);
        }
        let result = gtk::Box::new(gtk::Orientation::Vertical, 12);
        result.append(&picture);
        result.append(&result_actions);
        let start = adw::StatusPage::builder()
            .icon_name("camera-photo-symbolic")
            .title(gettext("Screenshot"))
            .build();
        let stack = gtk::Stack::new();
        stack.add_named(&start, Some("start"));
        stack.add_named(&result, Some("result"));
        stack.set_vexpand(true);

        let content = gtk::Box::new(gtk::Orientation::Vertical, 18);
        content.set_margin_top(18);
        content.set_margin_bottom(18);
        content.set_margin_start(18);
        content.set_margin_end(18);
        content.append(&stack);
        content.append(&options);
        content.append(&capture);

        let header = adw::HeaderBar::new();
        let menu = gio::Menu::new();
        menu.append(
            Some(&gettext("Open screenshots folder")),
            Some("win.folder"),
        );
        menu.append(Some(&gettext("About Screenshot")), Some("app.about"));
        let menu_button = gtk::MenuButton::new();
        menu_button.set_icon_name("open-menu-symbolic");
        menu_button.set_tooltip_text(Some(&gettext("Main menu")));
        menu_button.set_menu_model(Some(&menu));
        header.pack_end(&menu_button);
        let toolbar = adw::ToolbarView::new();
        toolbar.add_top_bar(&header);
        toolbar.set_content(Some(
            &adw::Clamp::builder()
                .maximum_size(720)
                .child(&content)
                .build(),
        ));
        let window = AsenaWindowBuilder::new(app)
            .title(gettext("Screenshot"))
            .default_size(560, 560)
            .min_size(360, 420)
            .content(&toolbar)
            .build();
        let state = Rc::new(State {
            window,
            stack,
            picture,
            delay,
            select,
            capture,
            last: RefCell::default(),
        });
        let this = Self { state };
        this.setup_actions();
        this
    }

    pub fn present(&self) {
        self.state.window.present();
    }

    fn setup_actions(&self) {
        let window = &self.state.window;
        let add = |name: &str, accels: &[&str], callback: Box<dyn Fn(&Self)>| {
            let action = gio::SimpleAction::new(name, None);
            let this = self.clone();
            action.connect_activate(move |_, _| callback(&this));
            window.add_action(&action);
            if let Some(app) = window.application() {
                app.set_accels_for_action(&format!("win.{name}"), accels);
            }
        };
        add(
            "capture",
            &["<Control>Return", "Print"],
            Box::new(|this| this.capture()),
        );
        add(
            "copy",
            &["<Control>c"],
            Box::new(|this| {
                if let Some(texture) = this
                    .state
                    .picture
                    .paintable()
                    .and_downcast::<gtk::gdk::Texture>()
                {
                    this.state.window.clipboard().set_texture(&texture);
                    this.state
                        .window
                        .show_toast(&gettext("Copied to the clipboard"));
                }
            }),
        );
        add(
            "show",
            &[],
            Box::new(|this| {
                if let Some(path) = this.state.last.borrow().clone() {
                    gtk::FileLauncher::new(Some(&gio::File::for_path(path)))
                        .open_containing_folder(
                            Some(this.state.window.window()),
                            gio::Cancellable::NONE,
                            |_| {},
                        );
                }
            }),
        );
        add(
            "folder",
            &[],
            Box::new(|this| {
                let directory = screenshot::directory();
                let _ = std::fs::create_dir_all(&directory);
                gtk::FileLauncher::new(Some(&gio::File::for_path(directory))).launch(
                    Some(this.state.window.window()),
                    gio::Cancellable::NONE,
                    |_| {},
                );
            }),
        );
        add(
            "save-as",
            &["<Control>s"],
            Box::new(|this| {
                let Some(source) = this.state.last.borrow().clone() else {
                    return;
                };
                let dialog = gtk::FileDialog::new();
                dialog.set_initial_name(source.file_name().and_then(|name| name.to_str()));
                let parent = this.state.window.window().clone();
                let window = this.state.window.clone();
                dialog.save(Some(&parent), gio::Cancellable::NONE, move |result| {
                    let Ok(target) = result else {
                        return;
                    };
                    let window = window.clone();
                    let source = gio::File::for_path(&source);
                    glib::spawn_future_local(async move {
                        let copied = gio::spawn_blocking(move || {
                            source.copy(
                                &target,
                                gio::FileCopyFlags::OVERWRITE,
                                gio::Cancellable::NONE,
                                None,
                            )
                        })
                        .await;
                        match copied {
                            Ok(Ok(())) => window.show_toast(&gettext("Screenshot saved")),
                            Ok(Err(error)) => show_error_dialog(
                                window.window(),
                                &gettext("Could not save the screenshot"),
                                error.message(),
                            ),
                            Err(_) => {}
                        }
                    });
                });
            }),
        );
        self.sync_actions();
    }

    fn sync_actions(&self) {
        let has_result = self.state.last.borrow().is_some();
        for name in ["copy", "show", "save-as"] {
            if let Some(action) = self
                .state
                .window
                .lookup_action(name)
                .and_downcast::<gio::SimpleAction>()
            {
                action.set_enabled(has_result);
            }
        }
    }

    fn capture(&self) {
        let delay = self.state.delay.value() as u64;
        let interactive = self.state.select.is_active();
        self.state.capture.set_sensitive(false);
        // Leave the screen before capturing so this window is not in the image.
        self.state.window.set_visible(false);
        let this = self.clone();
        glib::spawn_future_local(async move {
            glib::timeout_future(Duration::from_millis(400 + delay * 1000)).await;
            let result = async {
                let connection = zbus::Connection::session().await?;
                let uri = screenshot::capture(&connection, interactive).await?;
                let stamp = screenshot::stamp();
                gio::spawn_blocking(move || screenshot::store(&uri, &stamp))
                    .await
                    .map_err(|_| {
                        screenshot::Error::Failed(gettext("The screenshot could not be taken."))
                    })?
            }
            .await;
            this.state.window.present();
            this.state.capture.set_sensitive(true);
            match result {
                Ok(path) => {
                    let source = path.clone();
                    match gio::spawn_blocking(move || gtk::gdk::Texture::from_filename(source))
                        .await
                    {
                        Ok(Ok(texture)) => this.state.picture.set_paintable(Some(&texture)),
                        Ok(Err(error)) => {
                            tracing::warn!(%error, "Could not preview the screenshot")
                        }
                        Err(_) => {}
                    }
                    this.state.last.replace(Some(path));
                    this.state.stack.set_visible_child_name("result");
                    this.state.window.show_toast(&gettext("Screenshot saved"));
                }
                Err(screenshot::Error::Cancelled(_)) => {}
                Err(error) => show_error_dialog(
                    this.state.window.window(),
                    &gettext("Could not take the screenshot"),
                    &error.to_string(),
                ),
            }
            this.sync_actions();
        });
    }
}
