use asena_app::prelude::*;

mod calc;
mod window;

const APP_ID: &str = "org.asenalinux.Calculator";

fn main() -> glib::ExitCode {
    let app = AsenaApplication::new(APP_ID, gio::ApplicationFlags::FLAGS_NONE);

    let meta = AppMetadata::new(
        APP_ID,
        gettext("Calculator"),
        gettext("Calculate and convert units"),
    );
    app.setup_default_actions(meta);

    app.connect_activate(|app| {
        let win = window::CalculatorWindow::new(app);
        win.present();
    });

    app.run()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_application_constants() {
        assert_eq!(APP_ID, "org.asenalinux.Calculator");
    }
}
