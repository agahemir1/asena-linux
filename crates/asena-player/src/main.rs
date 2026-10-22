//! Asena Media Player: music and videos from your folders.

mod library;
mod window;

use asena_app::prelude::*;
use std::{cell::RefCell, rc::Rc};
use window::PlayerWindow;

const APPLICATION_ID: &str = "org.asenalinux.Player";

fn main() -> glib::ExitCode {
    let app = AsenaApplication::new(APPLICATION_ID, gio::ApplicationFlags::HANDLES_OPEN);
    app.setup_default_actions(AppMetadata::new(
        APPLICATION_ID,
        gettext("Media Player"),
        gettext("Play music and videos"),
    ));
    let window = Rc::new(RefCell::new(None::<PlayerWindow>));
    let ensure = {
        let window = window.clone();
        move |app: &AsenaApplication| -> PlayerWindow {
            window
                .borrow_mut()
                .get_or_insert_with(|| PlayerWindow::new(app))
                .clone()
        }
    };
    let activate = ensure.clone();
    app.connect_activate(move |app| activate(app).present());
    app.connect_open(move |app, files| {
        let window = ensure(app);
        window.play_files(files);
        window.present();
    });
    app.run()
}
