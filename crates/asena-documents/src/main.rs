//! Asena Documents: not ready in this release; see docs/apps/APP_ROADMAP.md.

use asena_app::prelude::*;

const APPLICATION_ID: &str = "org.asenalinux.Documents";

fn main() -> glib::ExitCode {
    let app = AsenaApplication::new(APPLICATION_ID, gio::ApplicationFlags::empty());
    app.setup_default_actions(AppMetadata::new(
        APPLICATION_ID,
        gettext("Documents"),
        gettext("View documents"),
    ));
    app.connect_activate(|app| {
        if let Some(window) = app.active_window() {
            window.present();
            return;
        }
        unavailable_window(
            app,
            &gettext("Documents"),
            "x-office-document-symbolic",
            &gettext(
                "Asena Documents is not ready yet. Open documents with another installed viewer.",
            ),
        )
        .present();
    });
    app.run()
}
