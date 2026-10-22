//! Quick settings in the Odak list language.
//!
//! Order follows how often each control is reached for: sliders, then
//! connections as list rows (each opens its network or device list in place),
//! then compact mode toggles, and the battery and session footer. Media
//! controls live in the notification sidebar.
use crate::surface::Surface;
use asena_core::{i18n::gettext, status};
use asena_services::{
    audio::{AudioModel, Change},
    bluetooth,
    brightness::BrightnessModel,
    network,
    property_model::PropertyModel,
    system::{self, SystemServices},
};
use gtk::prelude::*;
use std::{cell::Cell, rc::Rc};

#[derive(Clone)]
pub struct QuickSettings {
    surface: Surface,
}

impl QuickSettings {
    pub fn new(
        app: &adw::Application,
        services: &SystemServices,
        notifications: &gio::Settings,
        commands: &async_channel::Sender<crate::bus::Command>,
    ) -> Self {
        let surface = Surface::new(app, "asena-quick-settings", Default::default(), true);
        let content = gtk::Box::new(gtk::Orientation::Vertical, 12);
        content.add_css_class("asena-qs");

        let sliders = gtk::Box::new(gtk::Orientation::Vertical, 6);
        sliders.add_css_class("asena-qs-sliders");
        sliders.append(&volume_row(&services.audio, commands));
        sliders.append(&brightness_row(&services.brightness, commands));
        content.append(&sliders);

        let connections = gtk::Box::new(gtk::Orientation::Vertical, 0);
        connections.add_css_class("asena-qs-list");
        connections.append(&wifi_row(&surface.window, services));
        connections.append(&wired_row(services));
        connections.append(&bluetooth_row(&services.bluetooth));
        content.append(&connections);
        hide_when_empty(&connections);

        let modes = gtk::FlowBox::new();
        modes.add_css_class("asena-qs-modes");
        modes.set_selection_mode(gtk::SelectionMode::None);
        modes.set_homogeneous(true);
        modes.set_min_children_per_line(2);
        modes.set_max_children_per_line(2);
        modes.set_column_spacing(8);
        modes.set_row_spacing(8);
        modes.append(&quiet_toggle(notifications));
        modes.append(&dark_toggle());
        modes.append(&night_light_toggle(&services.daemon));
        modes.append(&profile_toggle(&services.profiles));
        content.append(&modes);
        sync_flow_visibility(&modes);

        content.append(&footer(&surface, services));
        surface.content.append(&content);
        Self { surface }
    }

    pub fn toggle(&self) {
        self.surface.toggle(360, 0);
    }

    pub fn hide(&self) {
        self.surface.hide();
    }

    pub fn connect_visibility(&self, callback: impl Fn(bool) + 'static) {
        self.surface
            .window
            .connect_visible_notify(move |window| callback(window.is_visible()));
    }
}

/// Hides a container when none of its rows are visible.
fn hide_when_empty(container: &gtk::Box) {
    let update = {
        let weak = container.downgrade();
        move || {
            if let Some(container) = weak.upgrade() {
                let mut child = container.first_child();
                let mut any = false;
                while let Some(widget) = child {
                    any |= widget.is_visible();
                    child = widget.next_sibling();
                }
                container.set_visible(any);
            }
        }
    };
    update();
    let update = Rc::new(update);
    let mut child = container.first_child();
    while let Some(widget) = child {
        let update = update.clone();
        widget.connect_visible_notify(move |_| update());
        child = widget.next_sibling();
    }
}

/// FlowBox children wrap their widget; mirror the widget visibility.
fn sync_flow_visibility(flow: &gtk::FlowBox) {
    let mut index = 0;
    while let Some(child) = flow.child_at_index(index) {
        if let Some(widget) = child.child() {
            child.set_visible(widget.is_visible());
            let weak = child.downgrade();
            widget.connect_visible_notify(move |widget| {
                if let Some(child) = weak.upgrade() {
                    child.set_visible(widget.is_visible());
                }
            });
        }
        index += 1;
    }
}

fn send(commands: &async_channel::Sender<crate::bus::Command>, action: crate::bus::Action) {
    let (reply, _) = async_channel::bounded(1);
    let _ = commands.try_send(crate::bus::Command { action, reply });
}

fn slider(
    icon: &str,
    label: &str,
    minimum: f64,
) -> (gtk::Box, gtk::Button, gtk::Scale, gtk::Label) {
    let row = gtk::Box::new(gtk::Orientation::Horizontal, 10);
    row.add_css_class("asena-qs-slider");
    let button = gtk::Button::from_icon_name(icon);
    button.add_css_class("flat");
    button.add_css_class("circular");
    row.append(&button);
    let scale = gtk::Scale::with_range(gtk::Orientation::Horizontal, minimum, 100.0, 1.0);
    scale.set_draw_value(false);
    scale.set_hexpand(true);
    scale.update_property(&[gtk::accessible::Property::Label(label)]);
    row.append(&scale);
    let value = gtk::Label::new(None);
    value.add_css_class("asena-qs-value");
    value.add_css_class("numeric");
    value.set_width_chars(4);
    value.set_xalign(1.0);
    row.append(&value);
    (row, button, scale, value)
}

fn volume_row(
    model: &AudioModel,
    commands: &async_channel::Sender<crate::bus::Command>,
) -> gtk::Box {
    let (row, mute, scale, value) = slider("audio-volume-high-symbolic", &gettext("Volume"), 0.0);
    mute.set_tooltip_text(Some(&gettext("Mute")));
    let output = gtk::MenuButton::new();
    output.set_icon_name("pan-down-symbolic");
    output.add_css_class("flat");
    output.add_css_class("circular");
    output.set_tooltip_text(Some(&gettext("Output device")));
    row.append(&output);
    let sender = commands.clone();
    scale.connect_change_value(move |_, _, level| {
        send(
            &sender,
            crate::bus::Action::Audio(Change::Level(level / 100.0)),
        );
        glib::Propagation::Stop
    });
    let sender = commands.clone();
    mute.connect_clicked(move |_| send(&sender, crate::bus::Action::Audio(Change::Mute)));
    let weak = row.downgrade();
    let sender = commands.clone();
    let update = move |model: &AudioModel| {
        let Some(row) = weak.upgrade() else {
            return;
        };
        let ready = model.available() && model.output_available();
        row.set_visible(ready);
        let percent = (model.volume() * 100.0).clamp(0.0, 100.0);
        scale.set_value(percent);
        scale.set_sensitive(!model.busy());
        mute.set_sensitive(!model.busy());
        value.set_text(&format!("{percent:.0}%"));
        if let Some(icon) = status::volume_icon(ready, model.volume(), model.muted()) {
            mute.set_icon_name(icon);
        }
        let snapshot = model.snapshot();
        output.set_visible(snapshot.sinks.len() > 1);
        let menu = gtk::Box::new(gtk::Orientation::Vertical, 2);
        for (name, device) in &snapshot.sinks {
            let item = gtk::Button::new();
            item.add_css_class("flat");
            item.add_css_class("asena-menu-item");
            let line = gtk::Box::new(gtk::Orientation::Horizontal, 10);
            let check = gtk::Image::from_icon_name("object-select-symbolic");
            check.set_opacity(if snapshot.sink.as_deref() == Some(name.as_str()) {
                1.0
            } else {
                0.0
            });
            line.append(&check);
            let label = gtk::Label::new(Some(&device.description));
            label.set_xalign(0.0);
            label.set_ellipsize(gtk::pango::EllipsizeMode::End);
            label.set_max_width_chars(32);
            line.append(&label);
            item.set_child(Some(&line));
            let sender = sender.clone();
            let name = name.clone();
            item.connect_clicked(move |item| {
                send(
                    &sender,
                    crate::bus::Action::Audio(Change::DefaultSink(name.clone())),
                );
                if let Some(popover) = item
                    .ancestor(gtk::Popover::static_type())
                    .and_downcast::<gtk::Popover>()
                {
                    popover.popdown();
                }
            });
            menu.append(&item);
        }
        let popover = gtk::Popover::new();
        popover.set_has_arrow(false);
        popover.set_child(Some(&menu));
        output.set_popover(Some(&popover));
    };
    update(model);
    model.connect_changed(update);
    row
}

fn brightness_row(
    model: &BrightnessModel,
    commands: &async_channel::Sender<crate::bus::Command>,
) -> gtk::Box {
    let (row, icon, scale, value) =
        slider("display-brightness-symbolic", &gettext("Brightness"), 5.0);
    icon.set_can_target(false);
    icon.set_focusable(false);
    let active = model.clone();
    let sender = commands.clone();
    scale.connect_change_value(move |_, _, level| {
        let model = active.clone();
        let sender = sender.clone();
        glib::spawn_future_local(async move {
            if let Ok(applied) = model.set_level(level / 100.0).await {
                send(
                    &sender,
                    crate::bus::Action::Osd {
                        kind: "brightness".into(),
                        value: applied,
                    },
                );
            }
        });
        glib::Propagation::Stop
    });
    let weak = row.downgrade();
    let update = move |model: &BrightnessModel| {
        let Some(row) = weak.upgrade() else {
            return;
        };
        row.set_visible(model.available());
        let percent = (model.level() * 100.0).clamp(5.0, 100.0);
        scale.set_value(percent);
        scale.set_sensitive(!model.busy());
        value.set_text(&format!("{percent:.0}%"));
    };
    update(model);
    model.connect_changed(update);
    row
}

/// A connection row: icon, title, state, a switch and an optional detail list.
struct ConnectionRow {
    root: gtk::Box,
    icon: gtk::Image,
    subtitle: gtk::Label,
    switch: gtk::Switch,
    expand: gtk::ToggleButton,
}

fn connection_row(icon: &str, title: &str, detail: Option<&gtk::Widget>) -> ConnectionRow {
    let root = gtk::Box::new(gtk::Orientation::Vertical, 0);
    root.add_css_class("asena-qs-row");
    let line = gtk::Box::new(gtk::Orientation::Horizontal, 12);
    line.add_css_class("asena-qs-row-line");
    let image = gtk::Image::from_icon_name(icon);
    image.add_css_class("asena-qs-row-icon");
    line.append(&image);
    let text = gtk::Box::new(gtk::Orientation::Vertical, 1);
    text.set_hexpand(true);
    text.set_valign(gtk::Align::Center);
    let heading = gtk::Label::new(Some(title));
    heading.set_xalign(0.0);
    heading.add_css_class("asena-qs-row-title");
    text.append(&heading);
    let subtitle = gtk::Label::new(None);
    subtitle.set_xalign(0.0);
    subtitle.set_ellipsize(gtk::pango::EllipsizeMode::End);
    subtitle.add_css_class("dim-label");
    subtitle.add_css_class("caption");
    text.append(&subtitle);
    line.append(&text);
    let expand = gtk::ToggleButton::new();
    expand.set_icon_name("go-next-symbolic");
    expand.add_css_class("flat");
    expand.add_css_class("circular");
    expand.set_valign(gtk::Align::Center);
    expand.set_tooltip_text(Some(&gettext("Show details")));
    expand.set_visible(detail.is_some());
    let switch = gtk::Switch::new();
    switch.set_valign(gtk::Align::Center);
    switch.update_property(&[gtk::accessible::Property::Label(title)]);
    line.append(&switch);
    line.append(&expand);
    root.append(&line);
    if let Some(detail) = detail {
        let revealer = gtk::Revealer::new();
        revealer.set_transition_type(gtk::RevealerTransitionType::SlideDown);
        revealer.set_transition_duration(150);
        let frame = gtk::Box::new(gtk::Orientation::Vertical, 0);
        frame.add_css_class("asena-qs-detail");
        frame.append(detail);
        revealer.set_child(Some(&frame));
        root.append(&revealer);
        expand
            .bind_property("active", &revealer, "reveal-child")
            .sync_create()
            .build();
        expand.connect_active_notify(|button| {
            button.set_icon_name(if button.is_active() {
                "go-down-symbolic"
            } else {
                "go-next-symbolic"
            });
        });
    }
    ConnectionRow {
        root,
        icon: image,
        subtitle,
        switch,
        expand,
    }
}

fn wifi_enabled(model: &asena_services::object_model::ObjectModel) -> bool {
    model.with_objects(|objects| {
        objects
            .iter()
            .find_map(|(path, interfaces)| {
                (path.as_str() == network::MANAGER).then_some(interfaces)
            })
            .and_then(|interfaces| interfaces.get(network::INTERFACE))
            .map(|properties| bluetooth::boolean(properties, "WirelessEnabled"))
            .unwrap_or(false)
    })
}

fn wifi_row(window: &gtk::ApplicationWindow, services: &SystemServices) -> gtk::Box {
    let objects = services.network_objects.clone();
    let list = asena_widgets::wireless::wifi_networks(window, &objects);
    let row = connection_row(
        "network-wireless-symbolic",
        &gettext("Wi-Fi"),
        Some(list.upcast_ref()),
    );
    let guard = Rc::new(Cell::new(false));
    let updating = guard.clone();
    let active = objects.clone();
    row.switch.connect_state_set(move |_, state| {
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
                tracing::debug!(%error, "Wi-Fi state change was rejected");
            }
        });
        glib::Propagation::Stop
    });
    let widget = row.root.clone();
    let root = row.root.downgrade();
    let manager = services.network.clone();
    let update = Rc::new(move |objects: &asena_services::object_model::ObjectModel| {
        let Some(root) = root.upgrade() else {
            return;
        };
        let (state, ssid) = system::network_state(&manager, objects);
        root.set_visible(objects.available() && state.wifi_device);
        let enabled = wifi_enabled(objects);
        guard.set(true);
        row.switch.set_active(enabled);
        row.switch.set_state(enabled);
        guard.set(false);
        row.switch.set_sensitive(!objects.busy());
        row.expand.set_sensitive(enabled);
        if !enabled {
            row.expand.set_active(false);
        }
        let connected = state.primary_type == "802-11-wireless"
            && state.state >= status::NM_STATE_CONNECTED_LOCAL;
        row.icon.set_icon_name(Some(if !enabled {
            "network-wireless-disabled-symbolic"
        } else if connected {
            status::wifi_signal_icon(state.wifi_strength.unwrap_or(0))
        } else {
            "network-wireless-signal-none-symbolic"
        }));
        row.subtitle.set_text(&match (enabled, ssid) {
            (false, _) => gettext("Off"),
            (true, Some(name)) => name,
            (true, None) => gettext("Not connected"),
        });
        root.set_css_classes(if enabled {
            &["asena-qs-row", "active"]
        } else {
            &["asena-qs-row"]
        });
    });
    update(&objects);
    let refresh = update.clone();
    objects.connect_changed(move |model| refresh(model));
    let refresh = update.clone();
    services.network.connect_changed(move |_| refresh(&objects));
    widget
}

fn wired_row(services: &SystemServices) -> gtk::Box {
    let row = connection_row("network-wired-symbolic", &gettext("Wired connection"), None);
    row.switch.set_visible(false);
    let root = row.root.clone();
    let weak = row.root.downgrade();
    let objects = services.network_objects.clone();
    let update = move |manager: &PropertyModel| {
        let Some(root) = weak.upgrade() else {
            return;
        };
        let wired = objects.with_objects(|objects| {
            objects.values().any(|interfaces| {
                interfaces.contains_key("org.freedesktop.NetworkManager.Device.Wired")
            })
        });
        root.set_visible(manager.available() && wired);
        let connected = manager.string("PrimaryConnectionType").as_deref()
            == Some("802-3-ethernet")
            && manager.unsigned("State").unwrap_or(0) >= status::NM_STATE_CONNECTED_LOCAL;
        row.subtitle.set_text(&if connected {
            gettext("Connected")
        } else {
            gettext("Not connected")
        });
        row.icon.set_icon_name(Some(if connected {
            "network-wired-symbolic"
        } else {
            "network-wired-disconnected-symbolic"
        }));
    };
    // Device objects and manager properties arrive independently.
    let update = Rc::new(update);
    update(&services.network);
    let refresh = update.clone();
    services
        .network
        .connect_changed(move |manager| refresh(manager));
    let manager = services.network.clone();
    services
        .network_objects
        .connect_changed(move |_| update(&manager));
    root
}

fn bluetooth_row(model: &asena_services::object_model::ObjectModel) -> gtk::Box {
    let list = asena_widgets::wireless::bluetooth_devices(model);
    let row = connection_row(
        "bluetooth-active-symbolic",
        &gettext("Bluetooth"),
        Some(list.upcast_ref()),
    );
    let root = row.root.clone();
    let guard = Rc::new(Cell::new(false));
    let updating = guard.clone();
    let active = model.clone();
    row.switch.connect_state_set(move |_, state| {
        if updating.get() {
            return glib::Propagation::Proceed;
        }
        if let Some(adapter) = bluetooth::adapters(&active).first() {
            let model = active.clone();
            let path = adapter.path.clone();
            glib::spawn_future_local(async move {
                if let Err(error) = model
                    .set_bool(&path, "org.bluez.Adapter1", "Powered", state)
                    .await
                {
                    tracing::debug!(%error, "Bluetooth power change was rejected");
                }
            });
        }
        glib::Propagation::Stop
    });
    let weak = row.root.downgrade();
    let update = move |model: &asena_services::object_model::ObjectModel| {
        let Some(root) = weak.upgrade() else {
            return;
        };
        let adapters = bluetooth::adapters(model);
        root.set_visible(model.available() && !adapters.is_empty());
        let powered = adapters.first().is_some_and(|adapter| adapter.powered);
        guard.set(true);
        row.switch.set_active(powered);
        row.switch.set_state(powered);
        guard.set(false);
        row.switch.set_sensitive(!model.busy());
        row.expand.set_sensitive(powered);
        if !powered {
            row.expand.set_active(false);
        }
        let connected = adapters
            .first()
            .map(|adapter| {
                bluetooth::devices(model, &adapter.path)
                    .iter()
                    .filter(|device| device.connected)
                    .count()
            })
            .unwrap_or(0);
        row.subtitle.set_text(&if !powered {
            gettext("Off")
        } else if connected > 0 {
            asena_core::i18n::ngettext(
                "{count} device connected",
                "{count} devices connected",
                connected as u32,
            )
            .replace("{count}", &connected.to_string())
        } else {
            gettext("On")
        });
        row.icon.set_icon_name(Some(if !powered {
            "bluetooth-disabled-symbolic"
        } else if connected > 0 {
            "bluetooth-connected-symbolic"
        } else {
            "bluetooth-active-symbolic"
        }));
        root.set_css_classes(if powered {
            &["asena-qs-row", "active"]
        } else {
            &["asena-qs-row"]
        });
    };
    update(model);
    model.connect_changed(update);
    root
}

/// Compact mode toggle: icon and short label, accent colored when on.
fn mode(icon: &str, label: &str) -> (gtk::Button, gtk::Image, gtk::Label) {
    let button = gtk::Button::new();
    button.add_css_class("asena-qs-mode");
    let content = gtk::Box::new(gtk::Orientation::Horizontal, 8);
    let image = gtk::Image::from_icon_name(icon);
    content.append(&image);
    let text = gtk::Label::new(Some(label));
    text.set_xalign(0.0);
    text.set_ellipsize(gtk::pango::EllipsizeMode::End);
    content.append(&text);
    button.set_child(Some(&content));
    button.update_property(&[gtk::accessible::Property::Label(label)]);
    (button, image, text)
}

fn set_on(button: &gtk::Button, on: bool) {
    if on {
        button.add_css_class("active");
    } else {
        button.remove_css_class("active");
    }
    button.update_state(&[gtk::accessible::State::Pressed(if on {
        gtk::AccessibleTristate::True
    } else {
        gtk::AccessibleTristate::False
    })]);
}

fn quiet_toggle(settings: &gio::Settings) -> gtk::Button {
    let (button, _, _) = mode(
        "notifications-disabled-symbolic",
        &gettext("Do not disturb"),
    );
    let weak = button.downgrade();
    let update = move |settings: &gio::Settings| {
        if let Some(button) = weak.upgrade() {
            set_on(&button, settings.boolean("do-not-disturb"));
        }
    };
    update(settings);
    settings.connect_changed(Some("do-not-disturb"), move |settings, _| update(settings));
    let settings = settings.clone();
    button.connect_clicked(move |_| {
        let _ = settings.set_boolean("do-not-disturb", !settings.boolean("do-not-disturb"));
    });
    button
}

fn dark_toggle() -> gtk::Button {
    let (button, _, _) = mode("dark-mode-symbolic", &gettext("Dark style"));
    let weak = button.downgrade();
    let update = move || {
        if let Some(button) = weak.upgrade() {
            set_on(&button, adw::StyleManager::default().is_dark());
        }
    };
    update();
    adw::StyleManager::default().connect_dark_notify(move |_| update());
    match asena_core::settings::load("org.asenalinux.desktop") {
        Ok(settings) => {
            button.connect_clicked(move |_| {
                let dark = adw::StyleManager::default().is_dark();
                let _ = settings.set_string(
                    "color-scheme",
                    if dark { "prefer-light" } else { "prefer-dark" },
                );
            });
        }
        Err(_) => button.set_visible(false),
    }
    button
}

fn night_light_toggle(daemon: &PropertyModel) -> gtk::Button {
    let (button, _, _) = mode("night-light-symbolic", &gettext("Night light"));
    let Ok(settings) = asena_core::settings::load("org.asenalinux.night-light") else {
        button.set_visible(false);
        return button;
    };
    let weak = button.downgrade();
    let prefs = settings.clone();
    let update = move |model: &PropertyModel| {
        if let Some(button) = weak.upgrade() {
            button.set_visible(
                model.available() && model.boolean("NightLightAvailable") == Some(true),
            );
            set_on(&button, prefs.boolean("enabled"));
        }
    };
    update(daemon);
    let refresh = update.clone();
    let model = daemon.clone();
    settings.connect_changed(Some("enabled"), move |_, _| refresh(&model));
    daemon.connect_changed(update);
    button.connect_clicked(move |_| {
        let _ = settings.set_boolean("enabled", !settings.boolean("enabled"));
    });
    button
}

fn profile_toggle(model: &PropertyModel) -> gtk::Button {
    let (button, image, label) = mode("power-profile-balanced-symbolic", &gettext("Balanced"));
    let weak = button.downgrade();
    let update = move |model: &PropertyModel| {
        let Some(button) = weak.upgrade() else {
            return;
        };
        let ids = system::profile_ids(model);
        button.set_visible(model.available() && !ids.is_empty());
        button.set_sensitive(!model.busy());
        let current = model
            .string("ActiveProfile")
            .unwrap_or_else(|| "balanced".into());
        label.set_text(&system::profile_label(&current));
        image.set_icon_name(Some(match current.as_str() {
            "performance" => "power-profile-performance-symbolic",
            "power-saver" => "power-profile-power-saver-symbolic",
            _ => "power-profile-balanced-symbolic",
        }));
        set_on(&button, current != "balanced");
        button.set_tooltip_text(Some(&gettext("Power profile")));
    };
    update(model);
    model.connect_changed(update);
    let active = model.clone();
    button.connect_clicked(move |_| {
        let model = active.clone();
        let ids = system::profile_ids(&model);
        if ids.is_empty() {
            return;
        }
        let current = model
            .string("ActiveProfile")
            .unwrap_or_else(|| "balanced".into());
        let index = ids.iter().position(|id| id == &current).unwrap_or(0);
        let next = ids[(index + 1) % ids.len()].clone();
        glib::spawn_future_local(async move {
            let value = zbus::zvariant::OwnedValue::from(zbus::zvariant::Str::from(next));
            let _ = model.set_property("ActiveProfile", value).await;
        });
    });
    button
}

fn footer(surface: &Surface, services: &SystemServices) -> gtk::Box {
    let footer = gtk::Box::new(gtk::Orientation::Horizontal, 6);
    footer.add_css_class("asena-qs-footer");
    let battery = gtk::Box::new(gtk::Orientation::Horizontal, 6);
    battery.set_hexpand(true);
    battery.set_valign(gtk::Align::Center);
    let icon = gtk::Image::new();
    let text = gtk::Label::new(None);
    text.set_xalign(0.0);
    text.set_ellipsize(gtk::pango::EllipsizeMode::End);
    battery.append(&icon);
    battery.append(&text);
    footer.append(&battery);
    let weak = battery.downgrade();
    let update = move |model: &PropertyModel| {
        let Some(battery) = weak.upgrade() else {
            return;
        };
        let percentage = system::battery_percentage(model);
        let state = system::battery_state(model);
        battery.set_opacity(if percentage.is_some() { 1.0 } else { 0.0 });
        if let (Some(value), Some(name)) = (percentage, status::battery_icon(percentage, state)) {
            icon.set_icon_name(Some(&name));
            let mut line = format!("{value:.0}%");
            match state {
                status::BatteryState::Charging => {
                    line.push_str(" · ");
                    line.push_str(&gettext("Charging"));
                }
                status::BatteryState::Full => {
                    line.push_str(" · ");
                    line.push_str(&gettext("Fully charged"));
                }
                _ => {}
            }
            text.set_text(&line);
        }
    };
    update(&services.battery);
    services.battery.connect_changed(update);

    let window = surface.window.downgrade();
    let screenshot = gtk::Button::from_icon_name("camera-photo-symbolic");
    screenshot.add_css_class("asena-icon-button");
    screenshot.set_tooltip_text(Some(&gettext("Take a screenshot")));
    screenshot.connect_clicked(move |_| {
        if let Some(window) = window.upgrade() {
            window.set_visible(false);
        }
        launch("asena-screenshot");
    });
    footer.append(&screenshot);
    let window = surface.window.downgrade();
    let settings = gtk::Button::from_icon_name("preferences-system-symbolic");
    settings.add_css_class("asena-icon-button");
    settings.set_tooltip_text(Some(&gettext("Settings")));
    settings.connect_clicked(move |_| {
        if let Some(window) = window.upgrade() {
            window.set_visible(false);
        }
        launch("asena-settings");
    });
    footer.append(&settings);
    footer.append(&crate::power::lock_button(
        &surface.window,
        &services.daemon,
    ));
    let power = gtk::MenuButton::new();
    power.set_icon_name("system-shutdown-symbolic");
    power.add_css_class("asena-icon-button");
    power.set_tooltip_text(Some(&gettext("Power")));
    crate::power::attach(&power, &surface.window, &services.daemon);
    footer.append(&power);
    footer
}

fn launch(command: &str) {
    crate::launch::command(command);
}
