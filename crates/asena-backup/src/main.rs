//! Asena Backup: copies of the home folder on an external drive (K31).
//! `asena-backup --scheduled` is the daily run started by the user timer.

mod backup;
mod window;

use asena_app::prelude::*;
use std::{cell::RefCell, path::PathBuf, rc::Rc};
use window::BackupWindow;

const APPLICATION_ID: &str = "org.asenalinux.Backup";
/// The timer fires hourly; a backup runs once the last one is this old.
const DAILY_SECONDS: i64 = 20 * 3600;

/// Runs without a window and reports a failure as a notification.
fn scheduled() -> glib::ExitCode {
    let Ok(settings) = asena_core::settings::load("org.asenalinux.backup") else {
        return glib::ExitCode::FAILURE;
    };
    let destination = settings.string("destination");
    let now = glib::DateTime::now_utc()
        .map(|now| now.to_unix())
        .unwrap_or_default();
    if !settings.boolean("automatic")
        || destination.is_empty()
        || now - settings.int64("last-backup") < DAILY_SECONDS
    {
        return glib::ExitCode::SUCCESS;
    }
    let destination = PathBuf::from(destination.as_str());
    // Not connected: the next hourly run tries again.
    if !destination.is_dir() || window::same_disk(&destination) {
        return glib::ExitCode::SUCCESS;
    }
    let history = gio::File::for_path(&destination)
        .query_filesystem_info("filesystem::type", gio::Cancellable::NONE)
        .ok()
        .and_then(|info| info.attribute_string("filesystem::type"))
        .is_some_and(|kind| backup::keeps_history(&kind));
    match backup::run(
        &glib::home_dir(),
        &window::folder(&destination),
        &window::stamp(),
        history,
        |_| {},
        |_| {},
    ) {
        backup::Outcome::Done => {
            let _ = settings.set_int64("last-backup", now);
            gio::Settings::sync();
            glib::ExitCode::SUCCESS
        }
        backup::Outcome::Stopped => glib::ExitCode::SUCCESS,
        backup::Outcome::Failed(error) => {
            tracing::warn!(%error, "Scheduled backup failed");
            notify(
                &gettext("Backup failed"),
                &gettext("Check that the backup drive has free space."),
            );
            glib::ExitCode::FAILURE
        }
    }
}

fn notify(summary: &str, body: &str) {
    let Ok(bus) = gio::bus_get_sync(gio::BusType::Session, gio::Cancellable::NONE) else {
        return;
    };
    let hints = glib::VariantDict::new(None);
    hints.insert("desktop-entry", "org.asenalinux.Backup");
    let parameters = (
        gettext("Backup"),
        0u32,
        APPLICATION_ID,
        summary,
        body,
        Vec::<String>::new(),
        hints.end(),
        -1i32,
    )
        .to_variant();
    let _ = bus.call_sync(
        Some("org.freedesktop.Notifications"),
        "/org/freedesktop/Notifications",
        "org.freedesktop.Notifications",
        "Notify",
        Some(&parameters),
        None,
        gio::DBusCallFlags::NONE,
        5000,
        gio::Cancellable::NONE,
    );
}

fn main() -> glib::ExitCode {
    if std::env::args().any(|argument| argument == "--scheduled") {
        if let Err(error) = asena_core::logging::init(APPLICATION_ID) {
            eprintln!("{error}");
        }
        return scheduled();
    }
    let app = AsenaApplication::new(APPLICATION_ID, gio::ApplicationFlags::empty());
    app.setup_default_actions(AppMetadata::new(
        APPLICATION_ID,
        gettext("Backup"),
        gettext("Back up personal files to another drive"),
    ));
    let window = Rc::new(RefCell::new(None::<BackupWindow>));
    app.connect_activate(move |app| {
        let mut slot = window.borrow_mut();
        if slot.is_none() {
            *slot = BackupWindow::new(app);
        }
        match slot.as_ref() {
            Some(window) => window.present(),
            None => unavailable_window(
                app,
                &gettext("Backup"),
                "drive-removable-media-symbolic",
                &gettext("Backup settings are unavailable"),
            )
            .present(),
        }
    });
    app.run()
}
