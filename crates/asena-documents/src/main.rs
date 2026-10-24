//! Asena Documents: a PDF viewer on the poppler command line tools (K31).

mod page;
mod poppler;
mod window;

use asena_app::prelude::*;
use std::{cell::RefCell, rc::Rc};
use window::DocumentsWindow;

const APPLICATION_ID: &str = "org.asenalinux.Documents";

fn main() -> glib::ExitCode {
    let app = AsenaApplication::new(APPLICATION_ID, gio::ApplicationFlags::HANDLES_OPEN);
    app.setup_default_actions(AppMetadata::new(
        APPLICATION_ID,
        gettext("Documents"),
        gettext("View documents"),
    ));
    let window = Rc::new(RefCell::new(None::<DocumentsWindow>));
    let shared = window.clone();
    app.connect_activate(move |app| {
        shared
            .borrow_mut()
            .get_or_insert_with(|| DocumentsWindow::new(app))
            .present();
    });
    app.connect_open(move |app, files| {
        // One window per document, like other document viewers.
        for path in files.iter().filter_map(|file| file.path()) {
            let opened = DocumentsWindow::new(app);
            opened.open(path);
            opened.present();
            window.borrow_mut().get_or_insert(opened);
        }
    });
    app.run()
}
