//! Asena Photos: browse and view the pictures in your Pictures folder.

mod photo;
mod window;

use asena_app::prelude::*;
use std::{cell::RefCell, rc::Rc};
use window::PhotosWindow;

const APPLICATION_ID: &str = "org.asenalinux.Photos";

fn main() -> glib::ExitCode {
    let app = AsenaApplication::new(APPLICATION_ID, gio::ApplicationFlags::HANDLES_OPEN);
    app.setup_default_actions(AppMetadata::new(
        APPLICATION_ID,
        gettext("Photos"),
        gettext("View your photos"),
    ));
    let window = Rc::new(RefCell::new(None::<PhotosWindow>));
    let ensure = {
        let window = window.clone();
        move |app: &AsenaApplication| -> PhotosWindow {
            window
                .borrow_mut()
                .get_or_insert_with(|| PhotosWindow::new(app))
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
