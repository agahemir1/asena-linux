use adw::prelude::*;
use asena_core::i18n::gettext;
use asena_services::{
    bluetooth,
    network::{self, AccessPoint, Security},
    object_model::ObjectModel,
};
use std::{
    cell::{Cell, RefCell},
    collections::BTreeMap,
    rc::Rc,
};

/// Wi-Fi switch and network list for the Settings application.
pub fn wifi<W: IsA<gtk::Window>>(parent: &W, model: &ObjectModel) -> gtk::Box {
    wifi_view(parent.as_ref(), model, true)
}

/// Network list only, for surfaces that show their own Wi-Fi switch.
pub fn wifi_networks<W: IsA<gtk::Window>>(parent: &W, model: &ObjectModel) -> gtk::Box {
    wifi_view(parent.as_ref(), model, false)
}

fn wifi_view(parent: &gtk::Window, model: &ObjectModel, full: bool) -> gtk::Box {
    let content = gtk::Box::new(gtk::Orientation::Vertical, 8);
    content.add_css_class("asena-wireless");
    let header = gtk::Box::new(gtk::Orientation::Horizontal, 8);
    let label = gtk::Label::new(Some(&gettext("Wi-Fi")));
    label.set_xalign(0.0);
    label.set_hexpand(true);
    header.append(&label);
    let toggle = gtk::Switch::new();
    toggle.set_valign(gtk::Align::Center);
    header.append(&toggle);
    let detail = gtk::Expander::new(Some(&gettext("Networks")));
    let body = gtk::Box::new(gtk::Orientation::Vertical, 4);
    let tools = gtk::Box::new(gtk::Orientation::Horizontal, 8);
    let caption = gtk::Label::new(Some(&gettext("Available networks")));
    caption.set_xalign(0.0);
    caption.set_hexpand(true);
    caption.add_css_class("caption-heading");
    caption.add_css_class("dim-label");
    tools.append(&caption);
    let refresh = gtk::Button::from_icon_name("view-refresh-symbolic");
    refresh.add_css_class("flat");
    refresh.add_css_class("circular");
    refresh.set_tooltip_text(Some(&gettext("Refresh")));
    tools.append(&refresh);
    body.append(&tools);
    let rows = gtk::Box::new(gtk::Orientation::Vertical, 2);
    body.append(&rows);
    if full {
        content.append(&header);
        detail.set_child(Some(&body));
        content.append(&detail);
    } else {
        content.append(&body);
        let active = model.clone();
        content.connect_map(move |_| request_scan(&active));
    }
    let error = gtk::Label::new(None);
    error.set_wrap(true);
    error.set_xalign(0.0);
    content.append(&error);
    let guard = Rc::new(Cell::new(false));
    let updating = guard.clone();
    let active = model.clone();
    toggle.connect_state_set(move |_, state| {
        if updating.get() {
            return glib::Propagation::Proceed;
        }
        let model = active.clone();
        glib::spawn_future_local(async move {
            if let Err(error) = model
                .set_bool(
                    network::MANAGER,
                    network::INTERFACE,
                    "WirelessEnabled",
                    state,
                )
                .await
            {
                tracing::debug!(%error,"Wi-Fi state change was rejected");
            }
        });
        glib::Propagation::Stop
    });
    let active = model.clone();
    detail.connect_expanded_notify(move |detail| {
        if detail.is_expanded() {
            request_scan(&active);
        }
    });
    let active = model.clone();
    refresh.connect_clicked(move |_| request_scan(&active));
    type Row = (
        gtk::Button,
        gtk::Label,
        gtk::Image,
        Rc<RefCell<AccessPoint>>,
    );
    let entries = Rc::new(RefCell::new(BTreeMap::<String, Row>::new()));
    let weak = content.downgrade();
    let parent = parent.downgrade();
    let update = move |model: &ObjectModel| {
        let Some(content) = weak.upgrade() else {
            return;
        };
        let device = network::wifi(model);
        content.set_visible(model.available() && device.is_some());
        let enabled = model.with_objects(|objects| {
            objects
                .iter()
                .find_map(|(path, interfaces)| {
                    (path.as_str() == network::MANAGER).then_some(interfaces)
                })
                .and_then(|interfaces| interfaces.get(network::INTERFACE))
                .map(|properties| bluetooth::boolean(properties, "WirelessEnabled"))
                .unwrap_or(false)
        });
        guard.set(true);
        toggle.set_active(enabled);
        toggle.set_state(enabled);
        guard.set(false);
        toggle.set_sensitive(!model.busy());
        refresh.set_sensitive(!model.busy() && enabled);
        detail.set_sensitive(enabled);
        body.set_sensitive(enabled);
        error.set_visible(!model.last_error().is_empty());
        error.set_text(&model.last_error());
        let points = device
            .as_deref()
            .map(|device| network::access_points(model, device))
            .unwrap_or_default();
        entries.borrow_mut().retain(|id, (row, _, _, _)| {
            if points.iter().any(|point| &point.id == id) {
                true
            } else {
                rows.remove(row);
                false
            }
        });
        let mut previous = None::<gtk::Button>;
        for point in points {
            if !entries.borrow().contains_key(&point.id) {
                let row = gtk::Button::new();
                row.add_css_class("flat");
                row.add_css_class("asena-wireless-row");
                let line = gtk::Box::new(gtk::Orientation::Horizontal, 10);
                let name = gtk::Label::new(None);
                name.set_xalign(0.0);
                name.set_ellipsize(gtk::pango::EllipsizeMode::End);
                name.set_hexpand(true);
                let icon = gtk::Image::new();
                line.append(&icon);
                line.append(&name);
                let state = gtk::Image::new();
                state.add_css_class("dim-label");
                line.append(&state);
                row.set_child(Some(&line));
                let selected = Rc::new(RefCell::new(point.clone()));
                let current = selected.clone();
                let active = model.clone();
                let parent = parent.clone();
                row.connect_clicked(move |_| {
                    if let Some(parent) = parent.upgrade() {
                        connect_network(&parent, &active, current.borrow().clone());
                    }
                });
                rows.append(&row);
                entries
                    .borrow_mut()
                    .insert(point.id.clone(), (row, name, icon, selected));
            }
            if let Some((row, name, icon, selected)) = entries.borrow().get(&point.id) {
                name.set_text(&point.name);
                icon.set_icon_name(Some(if point.connecting {
                    "network-wireless-acquiring-symbolic"
                } else {
                    asena_core::status::wifi_signal_icon(point.strength)
                }));
                if let Some(state) = row
                    .child()
                    .and_then(|line| line.last_child())
                    .and_downcast::<gtk::Image>()
                {
                    state.set_visible(point.active || point.security != Security::Open);
                    state.set_icon_name(Some(if point.active {
                        "object-select-symbolic"
                    } else {
                        "system-lock-screen-symbolic"
                    }));
                }
                row.set_sensitive(
                    !model.busy() && !point.active && point.security != Security::Unsupported,
                );
                row.set_tooltip_text(Some(&if point.security == Security::Unsupported {
                    gettext("This network requires authentication that is not supported yet.")
                } else {
                    gettext("Signal {strength}%").replace("{strength}", &point.strength.to_string())
                }));
                selected.replace(point);
                rows.reorder_child_after(row, previous.as_ref());
                previous = Some(row.clone());
            }
        }
    };
    update(model);
    model.connect_changed(update);
    content
}
fn request_scan(model: &ObjectModel) {
    if let Some(device) = network::wifi(model) {
        let model = model.clone();
        glib::spawn_future_local(async move {
            if let Err(error) = network::scan(&model, &device).await {
                tracing::debug!(%error,"Wi-Fi scan was rejected");
            }
        });
    }
}
fn connect_network(parent: &gtk::Window, model: &ObjectModel, point: AccessPoint) {
    if point.security == Security::Open {
        let weak = parent.downgrade();
        let model = model.clone();
        glib::spawn_future_local(async move {
            if let Err(error) = network::connect(&model, &point, String::new()).await {
                if let Some(parent) = weak.upgrade() {
                    show_error(&parent, &error.to_string());
                }
            }
        });
        return;
    }
    let dialog = adw::AlertDialog::new(Some(&point.name), None);
    let password = gtk::PasswordEntry::new();
    password.set_show_peek_icon(true);
    password.set_placeholder_text(Some(&gettext("Password")));
    dialog.set_extra_child(Some(&password));
    dialog.add_response("cancel", &gettext("Cancel"));
    dialog.add_response("connect", &gettext("Connect"));
    dialog.set_close_response("cancel");
    dialog.set_response_enabled("connect", false);
    let weak = dialog.downgrade();
    let security = point.security;
    password.connect_changed(move |entry| {
        if let Some(dialog) = weak.upgrade() {
            dialog.set_response_enabled(
                "connect",
                network::valid_password(security, entry.text().as_str()),
            );
        }
    });
    let model = model.clone();
    let weak_parent = parent.downgrade();
    dialog.connect_response(None, move |_, response| {
        let secret = password.text().to_string();
        password.set_text("");
        if response != "connect" {
            return;
        }
        let model = model.clone();
        let point = point.clone();
        let parent = weak_parent.clone();
        glib::spawn_future_local(async move {
            if let Err(error) = network::connect(&model, &point, secret).await {
                if let Some(parent) = parent.upgrade() {
                    show_error(&parent, &error.to_string());
                }
            }
        });
    });
    dialog.present(Some(parent));
}
fn show_error(parent: &gtk::Window, error: &str) {
    let dialog = adw::AlertDialog::new(
        Some(&gettext("The connection could not be completed")),
        Some(error),
    );
    dialog.add_response("close", &gettext("Close"));
    dialog.present(Some(parent));
}

/// Bluetooth switch and paired devices for the Settings application.
pub fn bluetooth(model: &ObjectModel) -> gtk::Box {
    bluetooth_view(model, true)
}

/// Paired devices only, for surfaces that show their own Bluetooth switch.
pub fn bluetooth_devices(model: &ObjectModel) -> gtk::Box {
    bluetooth_view(model, false)
}

fn bluetooth_view(model: &ObjectModel, full: bool) -> gtk::Box {
    let content = gtk::Box::new(gtk::Orientation::Vertical, 8);
    content.add_css_class("asena-wireless");
    let header = gtk::Box::new(gtk::Orientation::Horizontal, 8);
    let label = gtk::Label::new(Some(&gettext("Bluetooth")));
    label.set_xalign(0.0);
    label.set_hexpand(true);
    header.append(&label);
    let toggle = gtk::Switch::new();
    toggle.set_valign(gtk::Align::Center);
    header.append(&toggle);
    if full {
        content.append(&header);
    }
    let rows = gtk::Box::new(gtk::Orientation::Vertical, 2);
    content.append(&rows);
    let empty = gtk::Label::new(Some(&gettext("No paired devices")));
    empty.set_xalign(0.0);
    empty.add_css_class("dim-label");
    content.append(&empty);
    let error = gtk::Label::new(None);
    error.set_wrap(true);
    content.append(&error);
    let adapter = Rc::new(RefCell::new(None::<String>));
    let updating = Rc::new(Cell::new(false));
    let guard = updating.clone();
    let selected = adapter.clone();
    let active = model.clone();
    toggle.connect_state_set(move |_, state| {
        if guard.get() {
            return glib::Propagation::Proceed;
        }
        if let Some(path) = selected.borrow().clone() {
            let model = active.clone();
            glib::spawn_future_local(async move {
                if let Err(error) = model
                    .set_bool(&path, "org.bluez.Adapter1", "Powered", state)
                    .await
                {
                    tracing::debug!(%error,"Bluetooth power change was rejected");
                }
            });
        }
        glib::Propagation::Stop
    });
    let weak = content.downgrade();
    let update = move |model: &ObjectModel| {
        let Some(content) = weak.upgrade() else {
            return;
        };
        let adapters = bluetooth::adapters(model);
        let current = adapter.borrow().clone();
        let selected = current
            .and_then(|path| adapters.iter().find(|item| item.path == path))
            .or_else(|| adapters.first());
        content.set_visible(model.available() && selected.is_some());
        adapter.replace(selected.map(|item| item.path.clone()));
        updating.set(true);
        toggle.set_active(selected.is_some_and(|item| item.powered));
        toggle.set_state(selected.is_some_and(|item| item.powered));
        updating.set(false);
        toggle.set_sensitive(!model.busy());
        while let Some(row) = rows.first_child() {
            rows.remove(&row);
        }
        let mut count = 0;
        if let Some(selected) = selected {
            label.set_tooltip_text(Some(&selected.name));
            for device in bluetooth::devices(model, &selected.path) {
                count += 1;
                let button = gtk::Button::new();
                button.add_css_class("flat");
                button.add_css_class("asena-wireless-row");
                let line = gtk::Box::new(gtk::Orientation::Horizontal, 10);
                line.append(&gtk::Image::from_icon_name(if device.connected {
                    "bluetooth-connected-symbolic"
                } else {
                    "bluetooth-active-symbolic"
                }));
                let name = gtk::Label::new(Some(&device.name));
                name.set_xalign(0.0);
                name.set_hexpand(true);
                name.set_ellipsize(gtk::pango::EllipsizeMode::End);
                line.append(&name);
                if device.connected {
                    let state = gtk::Label::new(Some(&gettext("Connected")));
                    state.add_css_class("dim-label");
                    line.append(&state);
                }
                button.set_child(Some(&line));
                button.set_sensitive(!model.busy() && selected.powered);
                button.set_tooltip_text(Some(&if device.connected {
                    gettext("Disconnect")
                } else {
                    gettext("Connect")
                }));
                let active = model.clone();
                button.connect_clicked(move |_| {
                    let model = active.clone();
                    let path = device.path.clone();
                    let method = if device.connected {
                        "Disconnect"
                    } else {
                        "Connect"
                    };
                    glib::spawn_future_local(async move {
                        if let Err(error) =
                            model.call_unit(&path, "org.bluez.Device1", method).await
                        {
                            tracing::debug!(%error,"Bluetooth connection change was rejected");
                        }
                    });
                });
                rows.append(&button);
            }
        }
        empty.set_visible(count == 0);
        error.set_visible(!model.last_error().is_empty());
        error.set_text(&model.last_error());
    };
    update(model);
    model.connect_changed(update);
    content
}
