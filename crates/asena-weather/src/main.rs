//! Asena Weather: forecasts for saved places from Open-Meteo (K31).

mod forecast;
mod places;
mod window;

use asena_app::prelude::*;
use std::{cell::RefCell, rc::Rc};
use window::WeatherWindow;

const APPLICATION_ID: &str = "org.asenalinux.Weather";

fn main() -> glib::ExitCode {
    let app = AsenaApplication::new(APPLICATION_ID, gio::ApplicationFlags::empty());
    app.setup_default_actions(AppMetadata::new(
        APPLICATION_ID,
        gettext("Weather"),
        gettext("Show the weather forecast"),
    ));
    let window = Rc::new(RefCell::new(None::<WeatherWindow>));
    app.connect_activate(move |app| {
        window
            .borrow_mut()
            .get_or_insert_with(|| WeatherWindow::new(app))
            .present();
    });
    app.run()
}
