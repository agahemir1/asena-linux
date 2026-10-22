//! Standard dialogs for Asena applications.

use crate::i18n::gettext;
use adw::prelude::*;

/// Standard metadata for an Asena application's About dialog.
#[derive(Debug, Clone)]
pub struct AppMetadata {
    pub app_id: String,
    pub name: String,
    pub developer_name: String,
    pub version: String,
    pub comments: String,
    pub copyright: String,
}

impl AppMetadata {
    /// Metadata with the shared Asena developer, version and copyright lines.
    pub fn new(app_id: &str, name: String, comments: String) -> Self {
        Self {
            app_id: app_id.to_owned(),
            name,
            comments,
            ..Self::default()
        }
    }
}

impl Default for AppMetadata {
    fn default() -> Self {
        Self {
            app_id: "org.asenalinux.App".into(),
            name: gettext("Asena"),
            developer_name: gettext("Asena Linux developers"),
            version: env!("CARGO_PKG_VERSION").into(),
            comments: String::new(),
            copyright: "© 2026 Asena Linux".into(),
        }
    }
}

/// Presents the standard Asena About dialog.
pub fn show_about_dialog(parent: &impl IsA<gtk::Widget>, meta: &AppMetadata) {
    let dialog = adw::AboutDialog::new();
    dialog.set_application_name(&meta.name);
    dialog.set_application_icon(&meta.app_id);
    dialog.set_developer_name(&meta.developer_name);
    dialog.set_version(&meta.version);
    dialog.set_comments(&meta.comments);
    dialog.set_copyright(&meta.copyright);
    dialog.set_license_type(gtk::License::Gpl30);
    dialog.set_translator_credits(&gettext("translator-credits"));
    dialog.present(Some(parent));
}

/// Presents a standard error alert dialog.
pub fn show_error_dialog(parent: &impl IsA<gtk::Widget>, title: &str, message: &str) {
    let dialog = adw::AlertDialog::new(Some(title), Some(message));
    dialog.add_response("ok", &gettext("OK"));
    dialog.set_default_response(Some("ok"));
    dialog.present(Some(parent));
}

/// Presents a confirmation alert dialog with a callback upon confirmation.
pub fn confirm_dialog<F>(
    parent: &impl IsA<gtk::Widget>,
    title: &str,
    body: &str,
    confirm_label: &str,
    destructive: bool,
    on_confirm: F,
) where
    F: FnOnce() + 'static,
{
    let dialog = adw::AlertDialog::new(Some(title), Some(body));
    dialog.add_response("cancel", &gettext("Cancel"));
    dialog.add_response("confirm", confirm_label);
    dialog.set_default_response(Some("cancel"));
    if destructive {
        dialog.set_response_appearance("confirm", adw::ResponseAppearance::Destructive);
    } else {
        dialog.set_response_appearance("confirm", adw::ResponseAppearance::Suggested);
    }

    dialog.choose(parent, gio::Cancellable::NONE, move |response| {
        if response == "confirm" {
            on_confirm();
        }
    });
}
