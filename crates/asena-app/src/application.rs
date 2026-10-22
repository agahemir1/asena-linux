//! Standard application lifecycle wrapper for Asena applications.

use crate::{
    dialogs::{show_about_dialog, AppMetadata},
    i18n::init_i18n,
};
use adw::prelude::*;
use std::ops::Deref;

/// Standard application wrapper managing lifecycle, theming, actions, and internationalization.
#[derive(Clone)]
pub struct AsenaApplication {
    app: adw::Application,
}

impl Deref for AsenaApplication {
    type Target = adw::Application;

    fn deref(&self) -> &Self::Target {
        &self.app
    }
}

impl AsenaApplication {
    /// Creates a new Asena application instance.
    ///
    /// Shared styles, icons and preferences need an initialized display, so they
    /// are installed from the `startup` signal instead of here.
    pub fn new(app_id: &str, flags: gio::ApplicationFlags) -> Self {
        if let Err(error) = asena_core::logging::init(app_id) {
            eprintln!("{error}");
        }
        if let Err(error) = init_i18n() {
            tracing::warn!(%error, "Translations are unavailable");
        }

        let app = adw::Application::builder()
            .application_id(app_id)
            .flags(flags)
            .build();
        app.connect_startup(|_| {
            if let Err(error) = asena_widgets::init() {
                tracing::error!(%error, "Could not load the shared Asena resources");
            }
        });

        Self { app }
    }

    /// Sets up standard actions (`app.quit`, `app.about`) and shortcuts.
    pub fn setup_default_actions(&self, meta: AppMetadata) {
        let app = self.app.clone();
        let quit_action = gio::SimpleAction::new("quit", None);
        quit_action.connect_activate(move |_, _| {
            app.quit();
        });
        self.app.add_action(&quit_action);
        self.app.set_accels_for_action("app.quit", &["<Control>q"]);

        let app_for_about = self.app.clone();
        let about_action = gio::SimpleAction::new("about", None);
        about_action.connect_activate(move |_, _| {
            if let Some(window) = app_for_about.active_window() {
                show_about_dialog(&window, &meta);
            }
        });
        self.app.add_action(&about_action);
    }

    /// Connects the activation handler.
    pub fn connect_activate<F>(&self, callback: F)
    where
        F: Fn(&Self) + 'static,
    {
        let this = self.clone();
        self.app.connect_activate(move |_| {
            callback(&this);
        });
    }

    /// Connects the handler for files passed by the launcher or file manager.
    ///
    /// Applications created with `HANDLES_OPEN` must connect this; GLib would
    /// otherwise drop the request without showing a window.
    pub fn connect_open<F>(&self, callback: F)
    where
        F: Fn(&Self, &[gio::File]) + 'static,
    {
        let this = self.clone();
        self.app.connect_open(move |_, files, _| {
            callback(&this, files);
        });
    }

    /// Runs the application main loop.
    pub fn run(&self) -> glib::ExitCode {
        self.app.run()
    }
}
