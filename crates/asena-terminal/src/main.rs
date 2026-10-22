//! Asena Terminal: not ready in this release; see docs/apps/APP_ROADMAP.md.

use asena_app::prelude::*;

const APPLICATION_ID: &str = "org.asenalinux.Terminal";

fn main() -> glib::ExitCode {
    let app = AsenaApplication::new(APPLICATION_ID, gio::ApplicationFlags::empty());
    app.setup_default_actions(AppMetadata::new(
        APPLICATION_ID,
        gettext("Terminal"),
        gettext("Use the command line"),
    ));
    app.connect_activate(|app| {
        if let Some(window) = app.active_window() {
            window.present();
            return;
        }
        unavailable_window(
            app,
            &gettext("Terminal"),
            "utilities-terminal-symbolic",
            &gettext("Asena Terminal is not ready yet. Use the terminal chosen in Settings, Default applications."),
        )
        .present();
    });
    app.run()
}
