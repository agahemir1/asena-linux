//! Asena Camera: photos and videos from the camera through GStreamer (K31).

mod capture;
mod window;

use asena_app::prelude::*;
use std::{cell::RefCell, rc::Rc};
use window::CameraWindow;

const APPLICATION_ID: &str = "org.asenalinux.Camera";

fn main() -> glib::ExitCode {
    let app = AsenaApplication::new(APPLICATION_ID, gio::ApplicationFlags::empty());
    app.setup_default_actions(AppMetadata::new(
        APPLICATION_ID,
        gettext("Camera"),
        gettext("Take photos and videos"),
    ));
    let ready = match gst::init() {
        Ok(()) => true,
        Err(error) => {
            tracing::error!(%error, "GStreamer is unavailable");
            false
        }
    };
    let window = Rc::new(RefCell::new(None::<CameraWindow>));
    app.connect_activate(move |app| {
        if !ready {
            unavailable_window(
                app,
                &gettext("Camera"),
                "camera-disabled-symbolic",
                &gettext("The camera could not be started"),
            )
            .present();
            return;
        }
        window
            .borrow_mut()
            .get_or_insert_with(|| CameraWindow::new(app))
            .present();
    });
    app.run()
}
