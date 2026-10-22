//! Asena Screenshot: capture the screen through the desktop portal.

mod window;

use asena_app::prelude::*;
use std::{cell::RefCell, rc::Rc};
use window::CaptureWindow;

const APPLICATION_ID: &str = "org.asenalinux.Capture";

fn main() -> glib::ExitCode {
    let app = AsenaApplication::new(APPLICATION_ID, gio::ApplicationFlags::empty());
    app.setup_default_actions(AppMetadata::new(
        APPLICATION_ID,
        gettext("Screenshot"),
        gettext("Capture the screen"),
    ));
    let window = Rc::new(RefCell::new(None::<CaptureWindow>));
    app.connect_activate(move |app| {
        window
            .borrow_mut()
            .get_or_insert_with(|| CaptureWindow::new(app))
            .present();
    });
    app.run()
}
