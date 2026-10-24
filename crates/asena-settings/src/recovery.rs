//! Recovery: resetting the user's Asena settings (K30).
use crate::pages::{toast, Context};
use adw::prelude::*;
use asena_core::i18n::gettext;

pub fn page(context: &Context) -> adw::PreferencesPage {
    let page = adw::PreferencesPage::builder()
        .title(gettext("Recovery"))
        .icon_name("view-refresh-symbolic")
        .build();
    let settings = adw::PreferencesGroup::new();
    let button = crate::rows::button(&settings, &gettext("Asena settings"), &gettext("Reset"));
    button.add_css_class("destructive-action");
    let window = context.window.clone();
    button.connect_clicked(move |button| {
        let dialog = adw::AlertDialog::new(
            Some(&gettext("Reset Asena settings?")),
            Some(&gettext(
                "Appearance, panel, menu, notification, power, mouse and touchpad settings return to their defaults. The session language changes at the next login. Keyboard layouts, files and applications stay as they are.",
            )),
        );
        dialog.add_response("cancel", &gettext("Cancel"));
        dialog.add_response("reset", &gettext("Reset"));
        dialog.set_response_appearance("reset", adw::ResponseAppearance::Destructive);
        dialog.set_default_response(Some("cancel"));
        dialog.set_close_response("cancel");
        let button = button.downgrade();
        dialog.connect_response(Some("reset"), move |_, _| {
            let Some(button) = button.upgrade() else {
                return;
            };
            let reset = gio::SettingsSchemaSource::default()
                .map(|source| asena_core::settings::reset_all(&source, None))
                .unwrap_or_default();
            if reset.is_empty() {
                toast(&button, &gettext("Asena settings are unavailable"));
            } else {
                toast(&button, &gettext("Asena settings reset"));
            }
        });
        dialog.present(Some(&window));
    });
    page.add(&settings);
    let diagnostics = adw::PreferencesGroup::new();
    let report = crate::rows::button(&diagnostics, &gettext("System report"), &gettext("Save"));
    let window = context.window.clone();
    report.connect_clicked(move |_| {
        let dialog = gtk::FileDialog::builder()
            .title(gettext("Save system report"))
            .initial_name("Asena-system-report.txt")
            .build();
        let parent = window.clone();
        dialog.save(Some(&window), gio::Cancellable::NONE, move |result| {
            let Some(file) = result.ok() else {
                return;
            };
            let parent = parent.clone();
            glib::spawn_future_local(async move {
                let Ok(report) = gio::spawn_blocking(system_report).await else {
                    return;
                };
                if file
                    .replace_contents_future(
                        report.into_bytes(),
                        None,
                        false,
                        gio::FileCreateFlags::PRIVATE,
                    )
                    .await
                    .is_err()
                {
                    toast(&parent, &gettext("Could not save the system report"));
                } else {
                    toast(&parent, &gettext("System report saved"));
                }
            });
        });
    });
    page.add(&diagnostics);
    let system = adw::PreferencesGroup::builder()
        .title(gettext("Reset this computer"))
        .build();
    let mode = adw::ComboRow::builder()
        .title(gettext("Reset option"))
        .model(&gtk::StringList::new(&[
            &gettext("Keep my files"),
            &gettext("Remove everything"),
        ]))
        .build();
    system.add(&mode);
    let prepare = crate::rows::button(
        &system,
        &gettext("Restart into recovery"),
        &gettext("Prepare"),
    );
    prepare.add_css_class("destructive-action");
    prepare.set_sensitive(false);
    let cancel_row = adw::ActionRow::builder()
        .title(gettext("Pending reset"))
        .build();
    let cancel = gtk::Button::with_label(&gettext("Cancel reset"));
    cancel.set_valign(gtk::Align::Center);
    cancel_row.add_suffix(&cancel);
    cancel_row.set_activatable_widget(Some(&cancel));
    system.add(&cancel_row);
    cancel
        .bind_property("visible", &cancel_row, "visible")
        .sync_create()
        .build();
    cancel.set_visible(false);
    let status = adw::ActionRow::builder()
        .title(gettext("Checking recovery…"))
        .build();
    system.add(&status);
    page.add(&system);
    let status_ref = status.downgrade();
    let prepare_ref = prepare.downgrade();
    let cancel_ref = cancel.downgrade();
    glib::spawn_future_local(async move {
        let result = call("Status", ().to_variant()).await;
        let data = result
            .ok()
            .and_then(|reply| reply.child_value(0).str().map(str::to_owned))
            .and_then(|text| serde_json::from_str::<serde_json::Value>(&text).ok());
        let available = data
            .as_ref()
            .and_then(|data| data["available"].as_bool())
            .unwrap_or(false);
        let pending = data
            .as_ref()
            .and_then(|data| data["pending"].as_str())
            .is_some_and(|mode| !mode.is_empty());
        if let Some(status) = status_ref.upgrade() {
            status.set_title(&if available {
                gettext("The final confirmation appears in recovery. Applications will be removed.")
            } else {
                gettext("Recovery is unavailable. Start from an Asena installation drive.")
            });
        }
        if let Some(button) = prepare_ref.upgrade() {
            button.set_sensitive(available && !pending);
        }
        if let Some(button) = cancel_ref.upgrade() {
            button.set_visible(pending);
        }
    });
    let window = context.window.clone();
    let cancel_ref = cancel.downgrade();
    prepare.connect_clicked(move |button| {
        let chosen = if mode.selected() == 0 { "keep" } else { "remove" };
        let parent = window.clone(); let button = button.downgrade(); let cancel = cancel_ref.clone();
        glib::spawn_future_local(async move {
            let Some(button) = button.upgrade() else { return; };
            button.set_sensitive(false);
            match call("Schedule", (chosen,).to_variant()).await {
                Ok(_) => {
                    if let Some(cancel) = cancel.upgrade() { cancel.set_visible(true); }
                    let dialog = adw::AlertDialog::new(Some(&gettext("Recovery is ready")), Some(&gettext("Restart to continue. No files are removed until you confirm in recovery.")));
                    dialog.add_response("later", &gettext("Later")); dialog.add_response("restart", &gettext("Restart"));
                    dialog.set_default_response(Some("later")); dialog.set_close_response("later");
                    dialog.connect_response(Some("restart"), |_, _| { glib::spawn_future_local(async {
                        if let Ok(bus) = gio::bus_get_future(gio::BusType::System).await { let _ = bus.call_future(Some("org.freedesktop.login1"), "/org/freedesktop/login1", "org.freedesktop.login1.Manager", "Reboot", Some(&(true,).to_variant()), None, gio::DBusCallFlags::NONE, -1).await; }
                    }); });
                    dialog.present(Some(&parent));
                }
                Err(error) => { tracing::warn!(%error, "Could not prepare recovery"); button.set_sensitive(true); toast(&button, &gettext("Recovery could not be prepared")); }
            }
        });
    });
    let prepare_ref = prepare.downgrade();
    cancel.connect_clicked(move |button| {
        let button = button.downgrade();
        let prepare = prepare_ref.clone();
        glib::spawn_future_local(async move {
            if let Some(button) = button.upgrade() {
                match call("Cancel", ().to_variant()).await {
                    Ok(_) => {
                        button.set_visible(false);
                        if let Some(prepare) = prepare.upgrade() {
                            prepare.set_sensitive(true);
                        }
                    }
                    Err(_) => toast(&button, &gettext("The reset could not be cancelled")),
                }
            }
        });
    });
    page
}

async fn call(method: &str, parameters: glib::Variant) -> Result<glib::Variant, glib::Error> {
    let bus = gio::bus_get_future(gio::BusType::System).await?;
    bus.call_future(
        Some("org.asenalinux.Recovery"),
        "/org/asenalinux/Recovery",
        "org.asenalinux.Recovery",
        method,
        Some(&parameters),
        None,
        gio::DBusCallFlags::ALLOW_INTERACTIVE_AUTHORIZATION,
        600_000,
    )
    .await
}

fn system_report() -> String {
    let mut report = String::from("Asena Linux system report\n\n");
    for (program, arguments) in [
        ("uname", vec!["-srmo"]),
        ("cat", vec!["/etc/os-release"]),
        ("free", vec!["-h"]),
        ("lsblk", vec!["-o", "NAME,SIZE,FSTYPE,MOUNTPOINTS"]),
        ("systemctl", vec!["--failed", "--no-pager"]),
    ] {
        if let Ok(output) = std::process::Command::new(program)
            .args(arguments)
            .env("LC_ALL", "C")
            .output()
        {
            report.push_str(program);
            report.push('\n');
            report.push_str(&String::from_utf8_lossy(&output.stdout));
            report.push('\n');
        }
    }
    report
}
