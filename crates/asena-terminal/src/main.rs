//! Asena Terminal: the user's shell in VTE tabs (K31).

mod window;

use asena_app::prelude::*;
use window::TerminalWindow;

const APPLICATION_ID: &str = "org.asenalinux.Terminal";

fn main() -> glib::ExitCode {
    let app = AsenaApplication::new(APPLICATION_ID, gio::ApplicationFlags::HANDLES_COMMAND_LINE);
    app.setup_default_actions(AppMetadata::new(
        APPLICATION_ID,
        gettext("Terminal"),
        gettext("Use the command line"),
    ));
    // Every launch opens a new window; the tabs and actions keep it alive.
    app.connect_activate(|app| TerminalWindow::new(app).present());
    app.connect_command_line(|app, command| {
        let directory = command
            .cwd()
            .map(|path| path.to_string_lossy().into_owned());
        TerminalWindow::with_directory(app, directory).present();
        0
    });
    app.run()
}
