//! Asena Notes: Markdown notes stored as plain files.

mod note;
mod window;

use asena_app::prelude::*;
use std::{cell::RefCell, rc::Rc};
use window::NotesWindow;

const APPLICATION_ID: &str = "org.asenalinux.Notes";

fn main() -> glib::ExitCode {
    let app = AsenaApplication::new(APPLICATION_ID, gio::ApplicationFlags::empty());
    app.setup_default_actions(AppMetadata::new(
        APPLICATION_ID,
        gettext("Notes"),
        gettext("Write and organize notes"),
    ));
    let window = Rc::new(RefCell::new(None::<NotesWindow>));
    app.connect_activate(move |app| {
        window
            .borrow_mut()
            .get_or_insert_with(|| NotesWindow::new(app))
            .present();
    });
    app.run()
}
