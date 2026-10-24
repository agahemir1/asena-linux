use asena_app::prelude::*;

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
        i32::MAX,
    )
    .await
}

pub fn run() {
    let app = AsenaApplication::new("org.asenalinux.Recovery", gio::ApplicationFlags::empty());
    app.connect_activate(|app| {
        if let Some(window) = app.active_window() { window.present(); return; }
        let page = adw::PreferencesPage::new();
        let choices = adw::PreferencesGroup::new();
        let mode = adw::ComboRow::builder().title(gettext("Reset this computer"))
            .model(&gtk::StringList::new(&[&gettext("Keep my files"), &gettext("Remove everything")])).build();
        choices.add(&mode);
        page.add(&choices);
        let account = adw::PreferencesGroup::builder().title(gettext("New account")).visible(false).build();
        let username = adw::EntryRow::builder().title(gettext("Username")).build();
        let password = adw::PasswordEntryRow::builder().title(gettext("Password")).build();
        let confirm_password = adw::PasswordEntryRow::builder().title(gettext("Confirm password")).build();
        account.add(&username); account.add(&password); account.add(&confirm_password);
        page.add(&account);
        let account_ref = account.downgrade();
        mode.connect_selected_notify(move |mode| { if let Some(account) = account_ref.upgrade() { account.set_visible(mode.selected() == 1); } });
        let actions = adw::PreferencesGroup::new();
        let reset = gtk::Button::with_label(&gettext("Reset"));
        reset.add_css_class("destructive-action"); reset.set_halign(gtk::Align::Center);
        let restart = gtk::Button::with_label(&gettext("Restart"));
        restart.set_visible(false); restart.set_halign(gtk::Align::Center);
        restart.connect_clicked(|_| { glib::spawn_future_local(async {
            if let Ok(bus) = gio::bus_get_future(gio::BusType::System).await {
                let _ = bus.call_future(Some("org.freedesktop.login1"), "/org/freedesktop/login1", "org.freedesktop.login1.Manager", "Reboot", Some(&(true,).to_variant()), None, gio::DBusCallFlags::NONE, -1).await;
            }
        }); });
        actions.add(&reset); actions.add(&restart); page.add(&actions);
        let toolbar = adw::ToolbarView::new();
        let header = adw::HeaderBar::new(); header.set_show_end_title_buttons(false);
        toolbar.add_top_bar(&header); toolbar.set_content(Some(&page));
        let window = AsenaWindowBuilder::new(app).title(gettext("Asena Recovery"))
            .default_size(600, 560).min_size(360, 420).content(&toolbar).build();
        let parent = window.window().clone();
        reset.set_sensitive(false);
        let mode_ref = mode.downgrade(); let reset_ref = reset.downgrade(); let parent_ref = parent.downgrade();
        glib::spawn_future_local(async move {
            let data = call("Status", ().to_variant()).await.ok()
                .and_then(|reply| reply.child_value(0).str().map(str::to_owned))
                .and_then(|text| serde_json::from_str::<serde_json::Value>(&text).ok());
            let available = data.as_ref().is_some_and(|data| data["available"] == true && data["offline"] == true);
            if let Some(button) = reset_ref.upgrade() { button.set_sensitive(available); }
            if let (Some(data), Some(mode)) = (data, mode_ref.upgrade()) {
                let resume = data["resuming"].as_str().unwrap_or_default();
                let pending = data["pending"].as_str().unwrap_or_default();
                mode.set_selected(u32::from(if resume.is_empty() { pending } else { resume } == "remove"));
                mode.set_sensitive(resume.is_empty());
            }
            if !available { if let Some(parent) = parent_ref.upgrade() { show_error_dialog(&parent, &gettext("Recovery is unavailable"), &gettext("Start Asena Recovery from the boot menu or use an Asena installation drive.")); } }
        });
        let restart_ref = restart.downgrade();
        reset.connect_clicked(move |button| {
            let removing = mode.selected() == 1;
            let login = username.text().to_string();
            let secret = zeroize::Zeroizing::new(password.text().to_string());
            if removing && (login.is_empty() || secret.len() < 8 || secret.as_str() != confirm_password.text().as_str()) {
                show_error_dialog(&parent, &gettext("Check the new account"), &gettext("Enter a username and matching passwords of at least eight characters."));
                return;
            }
            let dialog = adw::AlertDialog::new(Some(&gettext("Reset this computer?")), Some(&if removing {
                gettext("All accounts, applications and personal files on this system will be removed. This cannot be undone.")
            } else {
                gettext("The system will be reinstalled. Installed applications, including user Flatpak applications, will be removed. Personal files and accounts will stay. Keep the computer connected to power.")
            }));
            dialog.add_response("cancel", &gettext("Cancel")); dialog.add_response("reset", &gettext("Reset"));
            dialog.set_response_appearance("reset", adw::ResponseAppearance::Destructive);
            dialog.set_default_response(Some("cancel")); dialog.set_close_response("cancel");
            let button = button.downgrade(); let parent_ref = parent.downgrade();
            let password_ref = password.downgrade(); let confirm_ref = confirm_password.downgrade();
            let restart_ref = restart_ref.clone();
            dialog.connect_response(Some("reset"), move |_, _| {
                let (Some(button), Some(parent)) = (button.upgrade(), parent_ref.upgrade()) else { return; };
                button.set_sensitive(false); button.set_label(&gettext("Resetting…"));
                if let Some(entry) = password_ref.upgrade() { entry.set_text(""); }
                if let Some(entry) = confirm_ref.upgrade() { entry.set_text(""); }
                parent.connect_close_request(|_| glib::Propagation::Stop);
                let parameters = (if removing {"remove"} else {"keep"}, login.as_str(), secret.as_str()).to_variant();
                let restart_ref = restart_ref.clone();
                glib::spawn_future_local(async move {
                    match call("Reset", parameters).await {
                        Ok(_) => { button.set_visible(false); if let Some(restart) = restart_ref.upgrade() { restart.set_visible(true); } }
                        Err(error) => { tracing::warn!(%error, "Reset failed"); button.set_sensitive(true); button.set_label(&gettext("Retry")); show_error_dialog(&parent, &gettext("Reset could not be completed"), &gettext("The recovery image and saved account information are kept. Retry from Asena Recovery. Do not delete the recovery partition.")); }
                    }
                });
            });
            dialog.present(Some(&parent));
        });
        window.present();
    });
    app.run();
}
