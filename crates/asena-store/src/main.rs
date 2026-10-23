//! Asena Software: applications from Debian, the Asena archive and Flathub
//! (K21, Phase 6F). Package changes go through the polkit-guarded
//! `org.asenalinux.Packages` service; Flatpak applications install for the
//! user through libflatpak.

mod catalog;
mod flatpak;
mod packages;
mod screenshots;
mod window;

use asena_app::prelude::*;
use std::{cell::RefCell, rc::Rc};
use window::StoreWindow;

const APPLICATION_ID: &str = "org.asenalinux.Store";

fn main() -> glib::ExitCode {
    let app = AsenaApplication::new(APPLICATION_ID, gio::ApplicationFlags::empty());
    app.setup_default_actions(AppMetadata::new(
        APPLICATION_ID,
        gettext("Software"),
        gettext("Install and update applications"),
    ));
    app.add_main_option(
        "updates",
        glib::Char::from(b'u'),
        glib::OptionFlags::NONE,
        glib::OptionArg::None,
        &gettext("Show the available updates"),
        None,
    );
    let window = Rc::new(RefCell::new(None::<StoreWindow>));
    let show_updates = gio::SimpleAction::new("show-updates", None);
    let target = window.clone();
    let application = app.clone();
    show_updates.connect_activate(move |_, _| {
        target
            .borrow_mut()
            .get_or_insert_with(|| StoreWindow::new(&application))
            .show_updates();
        if let Some(window) = target.borrow().as_ref() {
            window.present();
        }
    });
    app.add_action(&show_updates);
    // `--updates` opens the updates page, also in an instance that already runs.
    app.connect_handle_local_options(|app, options| {
        if !options.contains("updates") {
            return -1;
        }
        if app.register(gio::Cancellable::NONE).is_err() {
            return 1;
        }
        app.activate_action("show-updates", None);
        0
    });
    app.connect_activate(move |app| {
        window
            .borrow_mut()
            .get_or_insert_with(|| StoreWindow::new(app))
            .present();
    });
    app.run()
}
