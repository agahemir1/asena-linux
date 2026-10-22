use asena_core::{apps::DesktopEntry, i18n::gettext};
use gtk::prelude::*;

/// Session language variables for launched programs, so a language chosen in
/// Settings applies to applications opened afterwards.
pub fn language_environment() -> Vec<(&'static str, String)> {
    let language = asena_core::settings::load("org.asenalinux.session")
        .map(|settings| settings.string("language").to_string())
        .unwrap_or_default();
    asena_core::i18n::language_environment(&language, std::env::var_os("LC_ALL").is_some())
}

/// Launch context carrying the session language.
pub fn context() -> gio::AppLaunchContext {
    let context = gtk::gdk::Display::default()
        .map(|display| {
            display
                .app_launch_context()
                .upcast::<gio::AppLaunchContext>()
        })
        .unwrap_or_default();
    for (name, value) in language_environment() {
        context.setenv(name, value);
    }
    context
}

/// Runs a command line such as `asena-settings users` in the session language.
pub fn command(commandline: &str) {
    if let Err(error) =
        gio::AppInfo::create_from_commandline(commandline, None, gio::AppInfoCreateFlags::NONE)
            .and_then(|info| info.launch(&[], Some(&context())))
    {
        tracing::warn!(%error, commandline, "Launch failed");
    }
}

pub fn launch(entry: DesktopEntry, parent: &gtk::Widget) {
    let Some(display) = gtk::gdk::Display::default() else {
        return;
    };
    let context = display.app_launch_context();
    // Creating this metadata object only parses a command line in memory.
    // File-backed DesktopAppInfo construction and launch stay on the worker.
    let metadata = gio::AppInfo::create_from_commandline(
        &entry.commandline,
        Some(&entry.name),
        gio::AppInfoCreateFlags::SUPPORTS_STARTUP_NOTIFICATION,
    );
    let token = metadata
        .ok()
        .and_then(|info| context.startup_notify_id(Some(&info), &[]));
    let mut environment: Vec<String> = context
        .environment()
        .iter()
        .map(|item| item.to_string_lossy().into_owned())
        .collect();
    environment.extend(
        language_environment()
            .into_iter()
            .map(|(name, value)| format!("{name}={value}")),
    );
    let worker_token = token.clone();
    let weak = parent.downgrade();
    glib::spawn_future_local(async move {
        let result = gio::spawn_blocking(move || -> Result<(), String> {
            let app = gio::DesktopAppInfo::from_filename(&entry.filename)
                .ok_or_else(|| gettext("The application is no longer installed."))?;
            let context = gio::AppLaunchContext::new();
            for variable in environment {
                if let Some((name, value)) = variable.split_once('=') {
                    context.setenv(name, value);
                }
            }
            if let Some(token) = worker_token {
                context.setenv("XDG_ACTIVATION_TOKEN", &token);
                context.setenv("DESKTOP_STARTUP_ID", &token);
            }
            app.launch(&[], Some(&context))
                .map_err(|error| error.to_string())
        })
        .await;
        let error = match result {
            Ok(Ok(())) => return,
            Ok(Err(error)) => error,
            Err(error) => format!("{error:?}"),
        };
        if let Some(token) = token {
            context.launch_failed(&token);
        }
        tracing::warn!(%error, "Application launch failed");
        if let Some(parent) = weak.upgrade() {
            let dialog = adw::AlertDialog::builder()
                .heading(gettext("Could not open the application"))
                .body(&error)
                .build();
            dialog.add_response("close", &gettext("Close"));
            use adw::prelude::*;
            dialog.present(Some(&parent));
        }
    });
}
