//! Asena Archive: inspect and extract archives with libarchive's bsdtar.

mod archive;
mod window;

use asena_app::prelude::*;
use std::{cell::RefCell, rc::Rc};
use window::ArchiveWindow;

const APPLICATION_ID: &str = "org.asenalinux.Archive";

fn main() -> glib::ExitCode {
    let app = AsenaApplication::new(APPLICATION_ID, gio::ApplicationFlags::HANDLES_OPEN);
    app.setup_default_actions(AppMetadata::new(
        APPLICATION_ID,
        gettext("Archive"),
        gettext("Open and extract archives"),
    ));
    let window = Rc::new(RefCell::new(None::<ArchiveWindow>));
    let ensure = {
        let window = window.clone();
        move |app: &AsenaApplication| -> ArchiveWindow {
            window
                .borrow_mut()
                .get_or_insert_with(|| ArchiveWindow::new(app))
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
