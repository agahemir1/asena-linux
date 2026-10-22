//! Asena Files: browse folders, open files and manage the trash.

mod places;
mod window;

use asena_app::prelude::*;
use std::{cell::RefCell, rc::Rc};
use window::FilesWindow;

const APPLICATION_ID: &str = "org.asenalinux.Files";

fn main() -> glib::ExitCode {
    let app = AsenaApplication::new(APPLICATION_ID, gio::ApplicationFlags::HANDLES_OPEN);
    app.setup_default_actions(AppMetadata::new(
        APPLICATION_ID,
        gettext("Files"),
        gettext("Browse files and folders"),
    ));
    let window = Rc::new(RefCell::new(None::<FilesWindow>));
    let ensure = {
        let window = window.clone();
        move |app: &AsenaApplication| -> FilesWindow {
            window
                .borrow_mut()
                .get_or_insert_with(|| FilesWindow::new(app))
                .clone()
        }
    };
    let activate = ensure.clone();
    app.connect_activate(move |app| activate(app).present());
    app.connect_open(move |app, files| {
        let window = ensure(app);
        if let Some(file) = files.first() {
            window.open(file);
        }
        window.present();
    });
    app.run()
}
