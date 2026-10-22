//! Asena Clocks: world clocks, stopwatch and timer.

mod clock;
mod window;

use asena_app::prelude::*;
use std::{cell::RefCell, rc::Rc};
use window::ClockWindow;

const APPLICATION_ID: &str = "org.asenalinux.Clock";

fn main() -> glib::ExitCode {
    let app = AsenaApplication::new(APPLICATION_ID, gio::ApplicationFlags::empty());
    app.setup_default_actions(AppMetadata::new(
        APPLICATION_ID,
        gettext("Clocks"),
        gettext("World clocks, stopwatch and timers"),
    ));
    let window = Rc::new(RefCell::new(None::<ClockWindow>));
    app.connect_activate(move |app| {
        window
            .borrow_mut()
            .get_or_insert_with(|| ClockWindow::new(app))
            .present();
    });
    app.run()
}
