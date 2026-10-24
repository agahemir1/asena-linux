//! Asena Disks: drives, volumes and formatting through UDisks2 (K31).

mod model;
mod window;

use asena_app::prelude::*;
use std::{cell::RefCell, rc::Rc};
use window::DisksWindow;

const APPLICATION_ID: &str = "org.asenalinux.Disks";

fn main() -> glib::ExitCode {
    let app = AsenaApplication::new(APPLICATION_ID, gio::ApplicationFlags::empty());
    app.setup_default_actions(AppMetadata::new(
        APPLICATION_ID,
        gettext("Disks"),
        gettext("Manage drives and volumes"),
    ));
    let window = Rc::new(RefCell::new(None::<DisksWindow>));
    app.connect_activate(move |app| {
        window
            .borrow_mut()
            .get_or_insert_with(|| DisksWindow::new(app))
            .present();
    });
    app.run()
}
