//! Session actions shared by the start menu and quick settings.
use adw::prelude::*;
use asena_core::i18n::gettext;
use gtk4_layer_shell::{KeyboardMode, LayerShell};
use std::{
    cell::{Cell, RefCell},
    rc::Rc,
    time::Duration,
};

fn lock_available(daemon: &asena_services::property_model::PropertyModel) -> bool {
    daemon.available() && daemon.boolean("LockAvailable") == Some(true)
}

fn hide(window: &gtk::ApplicationWindow) {
    window.set_keyboard_mode(KeyboardMode::None);
    window.set_visible(false);
}

fn show_error(window: &gtk::ApplicationWindow, heading: &str, body: &str) {
    let dialog = adw::AlertDialog::new(Some(heading), Some(body));
    dialog.add_response("close", &gettext("Close"));
    dialog.present(Some(window));
}

/// Locks through the Asena daemon so the verified locker owns the session.
pub fn lock_session(window: &gtk::ApplicationWindow) {
    let window = window.downgrade();
    glib::spawn_future_local(async move {
        let result: zbus::Result<()> = async {
            let scope = asena_core::session::current_scope().map_err(|_| {
                zbus::Error::Failure(gettext("The current session could not be identified."))
            })?;
            let connection = zbus::Connection::session().await?;
            asena_core::dbus::DaemonProxy::new(&connection)
                .await?
                .lock(&scope)
                .await
        }
        .await;
        if let Some(window) = window.upgrade() {
            match result {
                Ok(()) => hide(&window),
                Err(error) => show_error(
                    &window,
                    &gettext("The session could not be locked."),
                    &error.to_string(),
                ),
            }
        }
    });
}

/// Icon button that locks the session; insensitive until the locker is verified.
pub fn lock_button(
    window: &gtk::ApplicationWindow,
    daemon: &asena_services::property_model::PropertyModel,
) -> gtk::Button {
    let button = gtk::Button::from_icon_name("system-lock-screen-symbolic");
    button.add_css_class("asena-icon-button");
    let label = gettext("Lock");
    button.update_property(&[gtk::accessible::Property::Label(&label)]);
    let weak = button.downgrade();
    let update = move |model: &asena_services::property_model::PropertyModel| {
        if let Some(button) = weak.upgrade() {
            let available = lock_available(model);
            button.set_sensitive(available);
            button.set_tooltip_text(Some(&if available {
                gettext("Lock")
            } else {
                gettext("A verified session locker is not available.")
            }));
        }
    };
    update(daemon);
    daemon.connect_changed(update);
    let window = window.downgrade();
    button.connect_clicked(move |_| {
        if let Some(window) = window.upgrade() {
            lock_session(&window);
        }
    });
    button
}

async fn login1_capability(method: &str) -> bool {
    let Ok(connection) = zbus::Connection::system().await else {
        return false;
    };
    let Ok(proxy) = zbus::Proxy::new(
        &connection,
        "org.freedesktop.login1",
        "/org/freedesktop/login1",
        "org.freedesktop.login1.Manager",
    )
    .await
    else {
        return false;
    };
    proxy
        .call::<_, _, String>(method, &())
        .await
        .is_ok_and(|value| value != "no" && value != "na")
}

fn action_row(icon: &str, label: &str) -> gtk::Button {
    let button = gtk::Button::new();
    button.add_css_class("flat");
    button.add_css_class("asena-menu-item");
    let content = gtk::Box::new(gtk::Orientation::Horizontal, 12);
    content.append(&gtk::Image::from_icon_name(icon));
    let text = gtk::Label::new(Some(label));
    text.set_xalign(0.0);
    content.append(&text);
    button.set_child(Some(&content));
    button
}

/// Power menu: suspend, log out, restart and power off.
pub fn attach(
    button: &gtk::MenuButton,
    parent: &gtk::ApplicationWindow,
    daemon: &asena_services::property_model::PropertyModel,
) {
    let popover = gtk::Popover::new();
    popover.set_has_arrow(false);
    popover.add_css_class("asena-popover");
    let content = gtk::Box::new(gtk::Orientation::Vertical, 2);
    let dev = std::env::var_os("ASENA_DEV").is_some();
    let actions = [
        (
            "system-suspend-symbolic",
            gettext("Suspend"),
            "Suspend",
            "CanSuspend",
        ),
        ("system-log-out-symbolic", gettext("Log out"), "logout", ""),
        (
            "system-reboot-symbolic",
            gettext("Restart"),
            "Reboot",
            "CanReboot",
        ),
        (
            "system-shutdown-symbolic",
            gettext("Power off"),
            "PowerOff",
            "CanPowerOff",
        ),
    ];
    for (icon, label, method, capability) in actions {
        let action = action_row(icon, &label);
        action.set_sensitive(!dev);
        if dev {
            action.set_tooltip_text(Some(&gettext(
                "Power actions are unavailable in a nested development session.",
            )));
        }
        if !capability.is_empty() {
            action.set_visible(false);
            let weak = action.downgrade();
            glib::spawn_future_local(async move {
                let supported = login1_capability(capability).await;
                if let Some(action) = weak.upgrade() {
                    action.set_visible(supported);
                }
            });
        }
        if method == "Suspend" && !dev {
            // Sleeping without a verified locker would resume to an open desktop.
            let weak = action.downgrade();
            let update = move |model: &asena_services::property_model::PropertyModel| {
                if let Some(action) = weak.upgrade() {
                    let available = lock_available(model);
                    action.set_sensitive(available);
                    let tooltip = (!available).then(|| {
                        gettext("A verified session locker is required before suspending.")
                    });
                    action.set_tooltip_text(tooltip.as_deref());
                }
            };
            update(daemon);
            daemon.connect_changed(update);
        }
        let parent = parent.downgrade();
        let popup = popover.downgrade();
        action.connect_clicked(move |_| {
            if let Some(popup) = popup.upgrade() {
                popup.popdown();
            }
            if let Some(parent) = parent.upgrade() {
                if method == "Suspend" {
                    run(&parent, method);
                } else {
                    confirm(&parent, &label, method);
                }
            }
        });
        content.append(&action);
    }
    popover.set_child(Some(&content));
    button.set_popover(Some(&popover));
}

fn confirm(parent: &gtk::ApplicationWindow, label: &str, method: &'static str) {
    let dialog = adw::AlertDialog::new(Some(label), None);
    dialog.add_response("cancel", &gettext("Cancel"));
    dialog.add_response("apply", label);
    dialog.set_response_appearance("apply", adw::ResponseAppearance::Destructive);
    dialog.set_close_response("cancel");
    let timer = Rc::new(RefCell::new(None::<glib::SourceId>));
    if method != "logout" {
        let remaining = Rc::new(Cell::new(60_u32));
        dialog.set_body(&gettext("Continuing in {seconds} seconds.").replace("{seconds}", "60"));
        let weak = dialog.downgrade();
        let pending = timer.clone();
        timer.replace(Some(glib::timeout_add_local(
            Duration::from_secs(1),
            move || {
                let Some(dialog) = weak.upgrade() else {
                    pending.borrow_mut().take();
                    return glib::ControlFlow::Break;
                };
                let value = remaining.get().saturating_sub(1);
                remaining.set(value);
                dialog.set_body(
                    &gettext("Continuing in {seconds} seconds.")
                        .replace("{seconds}", &value.to_string()),
                );
                if value == 0 {
                    pending.borrow_mut().take();
                    dialog.emit_by_name::<()>("response", &[&"apply"]);
                    dialog.force_close();
                    glib::ControlFlow::Break
                } else {
                    glib::ControlFlow::Continue
                }
            },
        )));
    }
    let weak = parent.downgrade();
    dialog.connect_response(None, move |_, response| {
        if let Some(timer) = timer.borrow_mut().take() {
            timer.remove();
        }
        if response == "apply" {
            if let Some(parent) = weak.upgrade() {
                run(&parent, method);
            }
        }
    });
    dialog.present(Some(parent));
}

fn run(parent: &gtk::ApplicationWindow, method: &'static str) {
    let weak = parent.downgrade();
    glib::spawn_future_local(async move {
        let result: Result<(), String> = async {
            let connection = zbus::Connection::system()
                .await
                .map_err(|e| e.to_string())?;
            let proxy = zbus::Proxy::new(
                &connection,
                "org.freedesktop.login1",
                "/org/freedesktop/login1",
                "org.freedesktop.login1.Manager",
            )
            .await
            .map_err(|e| e.to_string())?;
            if method == "logout" {
                let session = std::env::var("XDG_SESSION_ID")
                    .map_err(|_| gettext("The current session could not be identified."))?;
                proxy
                    .call("TerminateSession", &(session,))
                    .await
                    .map_err(|e| e.to_string())
            } else {
                proxy
                    .call(method, &(true,))
                    .await
                    .map_err(|e| e.to_string())
            }
        }
        .await;
        if let Some(parent) = weak.upgrade() {
            match result {
                Ok(()) => hide(&parent),
                Err(error) => show_error(
                    &parent,
                    &gettext("The operation could not be completed"),
                    &error,
                ),
            }
        }
    });
}
