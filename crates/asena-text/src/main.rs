//! Asena Text Editor: tabbed plain text editing.

mod document;
mod window;

use asena_app::prelude::*;
use std::{cell::RefCell, rc::Rc};
use window::TextWindow;

const APPLICATION_ID: &str = "org.asenalinux.Text";

fn main() -> glib::ExitCode {
    let app = AsenaApplication::new(APPLICATION_ID, gio::ApplicationFlags::HANDLES_OPEN);
    app.setup_default_actions(AppMetadata::new(
        APPLICATION_ID,
        gettext("Text Editor"),
        gettext("Edit text files"),
    ));
    let window = Rc::new(RefCell::new(None::<TextWindow>));
    let ensure = {
        let window = window.clone();
        move |app: &AsenaApplication| -> TextWindow {
            window
                .borrow_mut()
                .get_or_insert_with(|| TextWindow::new(app))
                .clone()
        }
    };
    let activate = ensure.clone();
    app.connect_activate(move |app| activate(app).present());
    app.connect_open(move |app, files| {
        let window = ensure(app);
        window.open_files(files);
        window.present();
    });
    app.run()
}
