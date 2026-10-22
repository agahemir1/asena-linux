//! Asena Weather: not ready in this release; see docs/apps/APP_ROADMAP.md.

use asena_app::prelude::*;

const APPLICATION_ID: &str = "org.asenalinux.Weather";

fn main() -> glib::ExitCode {
    let app = AsenaApplication::new(APPLICATION_ID, gio::ApplicationFlags::empty());
    app.setup_default_actions(AppMetadata::new(
        APPLICATION_ID,
        gettext("Weather"),
        gettext("Show the weather forecast"),
    ));
    app.connect_activate(|app| {
        if let Some(window) = app.active_window() {
            window.present();
            return;
        }
        unavailable_window(
            app,
            &gettext("Weather"),
            "weather-few-clouds-symbolic",
            &gettext("Asena Weather is not ready yet."),
        )
        .present();
    });
    app.run()
}
