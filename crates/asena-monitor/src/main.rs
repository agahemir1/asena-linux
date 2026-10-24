//! Asena System Monitor: processes and resource use (K31).

mod graph;
mod process_object;
mod procfs;
mod window;

use asena_app::prelude::*;
use std::{cell::RefCell, rc::Rc};
use window::MonitorWindow;

const APPLICATION_ID: &str = "org.asenalinux.SystemMonitor";

fn main() -> glib::ExitCode {
    let app = AsenaApplication::new(APPLICATION_ID, gio::ApplicationFlags::empty());
    app.setup_default_actions(AppMetadata::new(
        APPLICATION_ID,
        gettext("System Monitor"),
        gettext("Processes and resource use"),
    ));
    let window = Rc::new(RefCell::new(None::<MonitorWindow>));
    app.connect_activate(move |app| {
        window
            .borrow_mut()
            .get_or_insert_with(|| MonitorWindow::new(app))
            .present();
    });
    app.run()
}
