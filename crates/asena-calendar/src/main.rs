//! Asena Calendar: local events stored as iCalendar.

mod ics;
mod window;

use asena_app::prelude::*;
use std::{cell::RefCell, rc::Rc};
use window::CalendarWindow;

const APPLICATION_ID: &str = "org.asenalinux.Calendar";

fn main() -> glib::ExitCode {
    let app = AsenaApplication::new(APPLICATION_ID, gio::ApplicationFlags::empty());
    app.setup_default_actions(AppMetadata::new(
        APPLICATION_ID,
        gettext("Calendar"),
        gettext("Keep track of your events"),
    ));
    let window = Rc::new(RefCell::new(None::<CalendarWindow>));
    app.connect_activate(move |app| {
        window
            .borrow_mut()
            .get_or_insert_with(|| CalendarWindow::new(app))
            .present();
    });
    app.run()
}
