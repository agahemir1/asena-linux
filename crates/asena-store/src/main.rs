//! Asena Software: not ready in this release; see docs/apps/APP_ROADMAP.md.

use asena_app::prelude::*;

const APPLICATION_ID: &str = "org.asenalinux.Store";

fn main() -> glib::ExitCode {
    let app = AsenaApplication::new(APPLICATION_ID, gio::ApplicationFlags::empty());
    app.setup_default_actions(AppMetadata::new(
        APPLICATION_ID,
        gettext("Software"),
        gettext("Install and update applications"),
    ));
    app.connect_activate(|app| {
        if let Some(window) = app.active_window() {
            window.present();
            return;
        }
        unavailable_window(
            app,
            &gettext("Software"),
            "system-software-install-symbolic",
            &gettext("Asena Software is not ready yet. Install applications with the system package manager."),
        )
        .present();
    });
    app.run()
}
