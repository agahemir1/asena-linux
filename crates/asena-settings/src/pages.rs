//! Settings panels. Every row reads or changes real system or session state.
use crate::{rows, system_info};
use adw::prelude::*;
use asena_core::i18n::gettext;
use asena_services::{audio::Change, system::SystemServices};
use std::{cell::Cell, path::PathBuf, rc::Rc};

pub struct Context {
    pub window: gtk::Window,
    pub error: gtk::Label,
    pub services: SystemServices,
    pub open_panel: Rc<dyn Fn(&str)>,
}

fn page(title: &str, icon: &str) -> adw::PreferencesPage {
    adw::PreferencesPage::builder()
        .title(title)
        .icon_name(icon)
        .build()
}

pub(crate) fn group(title: &str) -> adw::PreferencesGroup {
    adw::PreferencesGroup::builder().title(title).build()
}

fn unavailable(page: &adw::PreferencesPage, message: &str) {
    let group = adw::PreferencesGroup::new();
    let row = adw::ActionRow::builder().title(message).build();
    row.add_prefix(&gtk::Image::from_icon_name("dialog-warning-symbolic"));
    group.add(&row);
    page.add(&group);
}

pub(crate) fn toast(widget: &impl IsA<gtk::Widget>, message: &str) {
    let mut current = widget.as_ref().parent();
    while let Some(widget) = current {
        if let Some(overlay) = widget.downcast_ref::<adw::ToastOverlay>() {
            overlay.add_toast(adw::Toast::new(message));
            return;
        }
        current = widget.parent();
    }
}

// Personalization ------------------------------------------------------------------

fn wallpaper_thumbnails(flow: &gtk::FlowBox, settings: &gio::Settings) {
    let flow = flow.downgrade();
    let settings = settings.clone();
    glib::spawn_future_local(async move {
        let found = gio::spawn_blocking(|| {
            let mut files: Vec<PathBuf> = std::fs::read_dir("/usr/share/backgrounds/asena")
                .map(|entries| {
                    entries
                        .flatten()
                        .map(|entry| entry.path())
                        .filter(|path| {
                            path.extension()
                                .and_then(|extension| extension.to_str())
                                .is_some_and(|extension| {
                                    matches!(
                                        extension.to_ascii_lowercase().as_str(),
                                        "jpg" | "jpeg" | "png" | "webp"
                                    )
                                })
                        })
                        .collect()
                })
                .unwrap_or_default();
            files.sort();
            files
                .into_iter()
                .filter_map(|path| {
                    let pixbuf =
                        gtk::gdk_pixbuf::Pixbuf::from_file_at_scale(&path, 224, 140, true).ok()?;
                    Some((
                        path,
                        pixbuf.read_pixel_bytes().to_vec(),
                        pixbuf.width(),
                        pixbuf.height(),
                        pixbuf.rowstride() as usize,
                        pixbuf.has_alpha(),
                    ))
                })
                .collect::<Vec<_>>()
        })
        .await
        .unwrap_or_default();
        let Some(flow) = flow.upgrade() else {
            return;
        };
        flow.set_visible(!found.is_empty());
        for (path, bytes, width, height, stride, alpha) in found {
            let texture = gtk::gdk::MemoryTexture::new(
                width,
                height,
                if alpha {
                    gtk::gdk::MemoryFormat::R8g8b8a8
                } else {
                    gtk::gdk::MemoryFormat::R8g8b8
                },
                &glib::Bytes::from_owned(bytes),
                stride,
            );
            let picture = gtk::Picture::for_paintable(&texture);
            picture.set_content_fit(gtk::ContentFit::Cover);
            picture.set_size_request(168, 105);
            let button = gtk::Button::new();
            button.set_child(Some(&picture));
            button.add_css_class("asena-wallpaper");
            button.set_overflow(gtk::Overflow::Hidden);
            let uri = gio::File::for_path(&path).uri().to_string();
            button.set_tooltip_text(path.file_stem().and_then(|stem| stem.to_str()));
            let mark = {
                let button = button.downgrade();
                let uri = uri.clone();
                move |settings: &gio::Settings| {
                    if let Some(button) = button.upgrade() {
                        if settings.string("background-uri") == uri.as_str() {
                            button.add_css_class("selected");
                        } else {
                            button.remove_css_class("selected");
                        }
                    }
                }
            };
            mark(&settings);
            settings.connect_changed(Some("background-uri"), move |settings, _| mark(settings));
            let prefs = settings.clone();
            button.connect_clicked(move |_| {
                let _ = prefs.set_string("background-uri", &uri);
            });
            flow.append(&button);
        }
    });
}

pub fn appearance(context: &Context) -> adw::PreferencesPage {
    let page = page(&gettext("Appearance"), "preferences-desktop-theme-symbolic");
    let Ok(settings) = asena_core::settings::load("org.asenalinux.desktop") else {
        unavailable(&page, &gettext("Appearance settings are unavailable"));
        return page;
    };
    let style = group(&gettext("Style"));
    rows::choice(
        &style,
        &settings,
        "color-scheme",
        &gettext("Color scheme"),
        &[
            ("default", gettext("Follow system")),
            ("prefer-light", gettext("Light")),
            ("prefer-dark", gettext("Dark")),
        ],
    );
    rows::choice(
        &style,
        &settings,
        "accent-color",
        &gettext("Accent color"),
        &[
            ("teal", gettext("Teal")),
            ("blue", gettext("Blue")),
            ("green", gettext("Green")),
            ("yellow", gettext("Yellow")),
            ("orange", gettext("Orange")),
            ("red", gettext("Red")),
            ("pink", gettext("Pink")),
            ("purple", gettext("Purple")),
            ("slate", gettext("Slate")),
        ],
    );
    rows::boolean(
        &style,
        &settings,
        "transparency",
        &gettext("Transparency effects"),
    );
    rows::number(
        &style,
        &settings,
        "text-scaling-factor",
        &gettext("Text scale"),
        (0.75, 2.0, 0.05),
        &context.error,
    );
    page.add(&style);

    let background = group(&gettext("Background"));
    let flow = gtk::FlowBox::new();
    flow.set_selection_mode(gtk::SelectionMode::None);
    flow.set_column_spacing(8);
    flow.set_row_spacing(8);
    flow.set_min_children_per_line(2);
    flow.set_max_children_per_line(4);
    flow.set_margin_bottom(12);
    flow.set_homogeneous(true);
    background.add(&flow);
    wallpaper_thumbnails(&flow, &settings);
    let choose = rows::button(&background, &gettext("Custom image"), &gettext("Choose…"));
    let window = context.window.clone();
    let prefs = settings.clone();
    choose.connect_clicked(move |_| {
        let filter = gtk::FileFilter::new();
        filter.set_name(Some(&gettext("Images")));
        filter.add_mime_type("image/*");
        let filters = gio::ListStore::new::<gtk::FileFilter>();
        filters.append(&filter);
        let dialog = gtk::FileDialog::new();
        dialog.set_filters(Some(&filters));
        let prefs = prefs.clone();
        dialog.open(Some(&window), gio::Cancellable::NONE, move |result| {
            if let Ok(file) = result {
                let _ = prefs.set_string("background-uri", &file.uri());
            }
        });
    });
    rows::choice(
        &background,
        &settings,
        "background-mode",
        &gettext("Background scaling"),
        &[
            ("zoom", gettext("Fill")),
            ("fit", gettext("Fit")),
            ("center", gettext("Center")),
            ("solid", gettext("Color only")),
        ],
    );
    page.add(&background);
    page
}

pub fn desktop(context: &Context) -> adw::PreferencesPage {
    let page = page(
        &gettext("Desktop and panel"),
        "preferences-desktop-display-symbolic",
    );
    let Ok(settings) = asena_core::settings::load("org.asenalinux.shell") else {
        unavailable(&page, &gettext("Panel settings are unavailable"));
        return page;
    };
    let panel = group(&gettext("Panel"));
    rows::choice(
        &panel,
        &settings,
        "panel-edge",
        &gettext("Panel position"),
        &[
            ("bottom", gettext("Bottom")),
            ("top", gettext("Top")),
            ("left", gettext("Left")),
            ("right", gettext("Right")),
        ],
    );
    rows::choice(
        &panel,
        &settings,
        "panel-alignment",
        &gettext("Application alignment"),
        &[
            ("start", gettext("Start")),
            ("center", gettext("Center")),
            ("end", gettext("End")),
        ],
    );
    rows::number(
        &panel,
        &settings,
        "panel-thickness",
        &gettext("Panel size"),
        (32.0, 96.0, 4.0),
        &context.error,
    );
    rows::number(
        &panel,
        &settings,
        "icon-size",
        &gettext("Icon size"),
        (16.0, 64.0, 2.0),
        &context.error,
    );
    rows::boolean(
        &panel,
        &settings,
        "panel-on-all-monitors",
        &gettext("Show on all monitors"),
    );
    page.add(&panel);
    let menu = group(&gettext("Start menu"));
    rows::choice(
        &menu,
        &settings,
        "menu-layout",
        &gettext("Menu type"),
        &[
            ("compact", gettext("Focus list")),
            ("classic", gettext("Categories")),
            ("grid", gettext("Grid")),
        ],
    );
    page.add(&menu);
    if let Ok(desktop) = asena_core::settings::load("org.asenalinux.desktop") {
        let icons = group(&gettext("Desktop"));
        rows::boolean(
            &icons,
            &desktop,
            "show-desktop-icons",
            &gettext("Desktop icons"),
        );
        page.add(&icons);
    }
    let reset = adw::PreferencesGroup::new();
    let button = rows::button(&reset, &gettext("Panel and menu layout"), &gettext("Reset"));
    let error = context.error.clone();
    button.connect_clicked(move |_| {
        for key in [
            "panel-edge",
            "menu-layout",
            "panel-thickness",
            "icon-size",
            "panel-alignment",
        ] {
            settings.reset(key);
        }
        rows::error(&error, "");
    });
    page.add(&reset);
    page
}

pub fn notifications(_context: &Context) -> adw::PreferencesPage {
    let page = page(
        &gettext("Notifications"),
        "preferences-system-notifications-symbolic",
    );
    match asena_core::settings::load("org.asenalinux.notifications") {
        Ok(settings) => {
            let group = adw::PreferencesGroup::new();
            rows::boolean(
                &group,
                &settings,
                "do-not-disturb",
                &gettext("Do not disturb"),
            );
            page.add(&group);
        }
        Err(_) => unavailable(&page, &gettext("Notification settings are unavailable")),
    }
    page
}

// Connections ----------------------------------------------------------------------

pub fn network(context: &Context) -> adw::PreferencesPage {
    let page = page(&gettext("Network"), "network-wireless-symbolic");
    let services = &context.services;
    let wifi = group(&gettext("Wi-Fi"));
    wifi.add(&asena_widgets::wireless::wifi(
        &context.window,
        &services.network_objects,
    ));
    page.add(&wifi);
    let wired = group(&gettext("Wired"));
    let status = rows::info(&wired, &gettext("Wired connection"), "");
    page.add(&wired);
    let missing = adw::PreferencesGroup::new();
    let row = adw::ActionRow::builder()
        .title(gettext("NetworkManager is not running"))
        .build();
    row.add_prefix(&gtk::Image::from_icon_name("network-offline-symbolic"));
    missing.add(&row);
    page.add(&missing);
    let objects = services.network_objects.clone();
    let update = {
        let (wifi, wired, missing, status) = (
            wifi.downgrade(),
            wired.downgrade(),
            missing.downgrade(),
            status.downgrade(),
        );
        move |manager: &asena_services::property_model::PropertyModel| {
            let (Some(wifi), Some(wired), Some(missing), Some(status)) = (
                wifi.upgrade(),
                wired.upgrade(),
                missing.upgrade(),
                status.upgrade(),
            ) else {
                return;
            };
            let available = manager.available();
            missing.set_visible(!available);
            let (state, _) = asena_services::system::network_state(manager, &objects);
            wifi.set_visible(available && state.wifi_device);
            let has_wired = objects.with_objects(|objects| {
                objects.values().any(|interfaces| {
                    interfaces.contains_key("org.freedesktop.NetworkManager.Device.Wired")
                })
            });
            wired.set_visible(available && has_wired);
            let connected = state.primary_type == "802-3-ethernet"
                && state.state >= asena_core::status::NM_STATE_CONNECTED_LOCAL;
            status.set_text(&if connected {
                gettext("Connected")
            } else {
                gettext("Not connected")
            });
        }
    };
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
    page
}

pub fn bluetooth(context: &Context) -> adw::PreferencesPage {
    let page = page(&gettext("Bluetooth"), "bluetooth-symbolic");
    let devices = adw::PreferencesGroup::new();
    devices.add(&asena_widgets::wireless::bluetooth(
        &context.services.bluetooth,
    ));
    page.add(&devices);
    let missing = adw::PreferencesGroup::new();
    let row = adw::ActionRow::builder()
        .title(gettext("No Bluetooth adapter found"))
        .build();
    row.add_prefix(&gtk::Image::from_icon_name("bluetooth-disabled-symbolic"));
    missing.add(&row);
    page.add(&missing);
    let weak = missing.downgrade();
    let update = move |model: &asena_services::object_model::ObjectModel| {
        if let Some(missing) = weak.upgrade() {
            missing.set_visible(
                !model.available() || asena_services::bluetooth::adapters(model).is_empty(),
            );
        }
    };
    update(&context.services.bluetooth);
    context.services.bluetooth.connect_changed(update);
    page
}

// Devices --------------------------------------------------------------------------

pub fn sound(context: &Context) -> adw::PreferencesPage {
    let page = page(&gettext("Sound"), "audio-volume-high-symbolic");
    let audio = context.services.audio.clone();
    let output = group(&gettext("Output"));
    let device = adw::ComboRow::builder()
        .title(gettext("Output device"))
        .build();
    output.add(&device);
    let volume_row = adw::ActionRow::builder().title(gettext("Volume")).build();
    let volume = gtk::Scale::with_range(gtk::Orientation::Horizontal, 0.0, 100.0, 1.0);
    volume.set_hexpand(true);
    volume.set_draw_value(false);
    volume.set_valign(gtk::Align::Center);
    volume.set_size_request(180, -1);
    volume.update_property(&[gtk::accessible::Property::Label(&gettext("Volume"))]);
    volume_row.add_suffix(&volume);
    output.add(&volume_row);
    let mute = adw::SwitchRow::builder().title(gettext("Mute")).build();
    output.add(&mute);
    page.add(&output);
    let input = group(&gettext("Input"));
    let microphone = rows::info(&input, &gettext("Microphone"), "");
    let mic_mute = adw::SwitchRow::builder()
        .title(gettext("Mute microphone"))
        .build();
    input.add(&mic_mute);
    page.add(&input);
    let missing = adw::PreferencesGroup::new();
    missing.add(
        &adw::ActionRow::builder()
            .title(gettext("The sound server is not available"))
            .build(),
    );
    page.add(&missing);

    let sinks: Rc<std::cell::RefCell<Vec<String>>> = Rc::default();
    let updating = Rc::new(Cell::new(false));
    let change = {
        let audio = audio.clone();
        move |change: Change| {
            let audio = audio.clone();
            glib::spawn_future_local(async move {
                if let Err(error) = audio.change(change).await {
                    tracing::debug!(%error, "Audio change was rejected");
                }
            });
        }
    };
    let change = Rc::new(change);
    {
        let (sinks, updating, change) = (sinks.clone(), updating.clone(), change.clone());
        device.connect_selected_notify(move |row| {
            if updating.get() {
                return;
            }
            if let Some(name) = sinks.borrow().get(row.selected() as usize) {
                change(Change::DefaultSink(name.clone()));
            }
        });
    }
    {
        let (updating, change) = (updating.clone(), change.clone());
        volume.connect_change_value(move |_, _, value| {
            if !updating.get() {
                change(Change::Level(value / 100.0));
            }
            glib::Propagation::Proceed
        });
    }
    {
        let (updating, change, audio) = (updating.clone(), change.clone(), audio.clone());
        mute.connect_active_notify(move |row| {
            if !updating.get() && row.is_active() != audio.muted() {
                change(Change::Mute);
            }
        });
    }
    {
        let (updating, change, audio) = (updating.clone(), change.clone(), audio.clone());
        mic_mute.connect_active_notify(move |row| {
            if !updating.get() && row.is_active() != audio.microphone_muted() {
                change(Change::Microphone);
            }
        });
    }
    let update = {
        let (output, input, missing) = (output.downgrade(), input.downgrade(), missing.downgrade());
        move |audio: &asena_services::audio::AudioModel| {
            let (Some(output), Some(input), Some(missing)) =
                (output.upgrade(), input.upgrade(), missing.upgrade())
            else {
                return;
            };
            updating.set(true);
            let snapshot = audio.snapshot();
            missing.set_visible(!audio.available());
            output.set_visible(audio.available() && audio.output_available());
            input.set_visible(audio.available() && audio.microphone_available());
            let names: Vec<String> = snapshot.sinks.keys().cloned().collect();
            let labels: Vec<&str> = snapshot
                .sinks
                .values()
                .map(|device| device.description.as_str())
                .collect();
            device.set_model(Some(&gtk::StringList::new(&labels)));
            device.set_selected(
                snapshot
                    .sink
                    .as_ref()
                    .and_then(|sink| names.iter().position(|name| name == sink))
                    .map(|index| index as u32)
                    .unwrap_or(gtk::INVALID_LIST_POSITION),
            );
            sinks.replace(names);
            volume.set_value(audio.volume() * 100.0);
            mute.set_active(audio.muted());
            mic_mute.set_active(audio.microphone_muted());
            microphone.set_text(
                &snapshot
                    .source
                    .as_ref()
                    .and_then(|source| snapshot.sources.get(source))
                    .map(|device| device.description.clone())
                    .unwrap_or_default(),
            );
            updating.set(false);
        }
    };
    update(&audio);
    audio.connect_changed(update);
    page
}

pub fn keyboard(context: &Context) -> adw::PreferencesPage {
    let page = page(&gettext("Keyboard"), "input-keyboard-symbolic");
    let Ok(settings) = asena_core::settings::load("org.asenalinux.input") else {
        unavailable(&page, &gettext("Keyboard settings are unavailable"));
        return page;
    };
    let layouts = group(&gettext("Layouts"));
    let presets: [(&[&str], String); 5] = [
        (&["tr", "us"], gettext("Turkish Q and English (US)")),
        (&["tr(f)", "us"], gettext("Turkish F and English (US)")),
        (&["tr"], gettext("Turkish Q")),
        (&["tr(f)"], gettext("Turkish F")),
        (&["us"], gettext("English (US)")),
    ];
    let labels: Vec<&str> = presets.iter().map(|(_, label)| label.as_str()).collect();
    let layout = adw::ComboRow::builder()
        .title(gettext("Keyboard layout"))
        .model(&gtk::StringList::new(&labels))
        .build();
    let current: Vec<String> = settings
        .strv("keyboard-layouts")
        .iter()
        .map(ToString::to_string)
        .collect();
    if let Some(index) = presets.iter().position(|(values, _)| {
        values
            .iter()
            .map(|value| value.to_string())
            .collect::<Vec<_>>()
            == current
    }) {
        layout.set_selected(index as u32);
    }
    let prefs = settings.clone();
    let values: Vec<Vec<String>> = presets
        .iter()
        .map(|(values, _)| values.iter().map(|value| value.to_string()).collect())
        .collect();
    layout.connect_selected_notify(move |row| {
        if let Some(values) = values.get(row.selected() as usize) {
            let refs: Vec<&str> = values.iter().map(String::as_str).collect();
            let _ = prefs.set_strv("keyboard-layouts", refs);
        }
    });
    layouts.add(&layout);
    rows::choice(
        &layouts,
        &settings,
        "keyboard-toggle",
        &gettext("Switch layout"),
        &[
            ("grp:alt_shift_toggle", "Alt + Shift".to_string()),
            ("grp:win_space_toggle", "Super + Space".to_string()),
        ],
    );
    page.add(&layouts);
    let typing = group(&gettext("Typing"));
    rows::number(
        &typing,
        &settings,
        "repeat-rate",
        &gettext("Repeat rate"),
        (10.0, 100.0, 5.0),
        &context.error,
    );
    rows::number(
        &typing,
        &settings,
        "repeat-delay",
        &gettext("Repeat delay (ms)"),
        (100.0, 1000.0, 50.0),
        &context.error,
    );
    page.add(&typing);
    page
}

pub fn mouse(context: &Context) -> adw::PreferencesPage {
    let page = page(&gettext("Mouse and touchpad"), "input-mouse-symbolic");
    let Ok(settings) = asena_core::settings::load("org.asenalinux.input") else {
        unavailable(&page, &gettext("Pointer settings are unavailable"));
        return page;
    };
    let pointer = group(&gettext("Pointer"));
    rows::number(
        &pointer,
        &settings,
        "pointer-speed",
        &gettext("Pointer speed"),
        (-1.0, 1.0, 0.1),
        &context.error,
    );
    rows::boolean(
        &pointer,
        &settings,
        "natural-scroll",
        &gettext("Natural scrolling"),
    );
    page.add(&pointer);
    let touchpad = group(&gettext("Touchpad"));
    rows::boolean(
        &touchpad,
        &settings,
        "tap-to-click",
        &gettext("Tap to click"),
    );
    page.add(&touchpad);
    page
}

pub fn power(context: &Context) -> adw::PreferencesPage {
    let page = page(&gettext("Power"), "battery-symbolic");
    let battery_group = group(&gettext("Battery"));
    let battery = rows::info(&battery_group, &gettext("Charge"), "");
    page.add(&battery_group);
    let weak_group = battery_group.downgrade();
    let update = move |model: &asena_services::property_model::PropertyModel| {
        let Some(group) = weak_group.upgrade() else {
            return;
        };
        let value = asena_services::system::battery_percentage(model);
        group.set_visible(value.is_some());
        if let Some(value) = value {
            let state = match asena_services::system::battery_state(model) {
                asena_core::status::BatteryState::Charging => gettext("Charging"),
                asena_core::status::BatteryState::Full => gettext("Fully charged"),
                _ => gettext("On battery"),
            };
            battery.set_text(&format!("{value:.0}% · {state}"));
        }
    };
    update(&context.services.battery);
    context.services.battery.connect_changed(update);
    let Ok(settings) = asena_core::settings::load("org.asenalinux.power") else {
        unavailable(&page, &gettext("Power settings are unavailable"));
        return page;
    };
    let idle = group(&gettext("When idle"));
    rows::boolean(
        &idle,
        &settings,
        "idle-enabled",
        &gettext("Automatic actions"),
    );
    for (key, title, range) in [
        (
            "dim-seconds",
            gettext("Dim the screen after (seconds)"),
            (30.0, 1800.0, 30.0),
        ),
        (
            "lock-seconds",
            gettext("Lock after (seconds)"),
            (60.0, 3600.0, 60.0),
        ),
        (
            "off-seconds",
            gettext("Turn the screen off after (seconds)"),
            (120.0, 7200.0, 60.0),
        ),
        (
            "suspend-seconds",
            gettext("Suspend after (seconds)"),
            (300.0, 14400.0, 300.0),
        ),
    ] {
        rows::number(&idle, &settings, key, &title, range, &context.error);
    }
    page.add(&idle);
    page
}

// System ---------------------------------------------------------------------------

pub fn users(context: &Context) -> adw::PreferencesPage {
    let page = page(&gettext("Users"), "system-users-symbolic");
    let account = group(&gettext("Your account"));
    let user = &context.services.user;
    let avatar = adw::Avatar::new(64, Some(&user.name()), true);
    avatar.set_margin_bottom(12);
    user.bind_property("name", &avatar, "text")
        .sync_create()
        .build();
    account.add(&avatar);
    let name = rows::info(&account, &gettext("Name"), &user.name());
    user.bind_property("name", &name, "label")
        .sync_create()
        .build();
    rows::info(
        &account,
        &gettext("User name"),
        &glib::user_name().to_string_lossy(),
    );
    rows::info(
        &account,
        &gettext("Home folder"),
        &glib::home_dir().display().to_string(),
    );
    page.add(&account);
    page
}

pub fn region(_context: &Context) -> adw::PreferencesPage {
    let page = page(
        &gettext("Region and language"),
        "preferences-desktop-locale-symbolic",
    );
    let Ok(settings) = asena_core::settings::load("org.asenalinux.session") else {
        unavailable(&page, &gettext("Language settings are unavailable"));
        return page;
    };
    let language = adw::PreferencesGroup::builder()
        .title(gettext("Language"))
        .description(gettext("Applications opened from now on use the new language. The panel and menu change the next time you log in."))
        .build();
    rows::choice(
        &language,
        &settings,
        "language",
        &gettext("Session language"),
        &[
            ("tr_TR.UTF-8", "Türkçe (Türkiye)".to_string()),
            ("en_US.UTF-8", "English (United States)".to_string()),
        ],
    );
    page.add(&language);
    page
}

pub fn datetime(context: &Context) -> adw::PreferencesPage {
    let page = page(
        &gettext("Date and time"),
        "preferences-system-time-symbolic",
    );
    let clock = group(&gettext("Date and time"));
    let zone = rows::info(
        &clock,
        &gettext("Time zone"),
        &glib::TimeZone::local().identifier(),
    );
    let automatic = adw::SwitchRow::builder()
        .title(gettext("Set the time automatically"))
        .sensitive(false)
        .build();
    clock.add(&automatic);
    page.add(&clock);
    let timedate = asena_services::property_model::PropertyModel::new(
        asena_services::property_model::Endpoint::system(
            "org.freedesktop.timedate1",
            "/org/freedesktop/timedate1",
            "org.freedesktop.timedate1",
        ),
    );
    timedate.start();
    let updating = Rc::new(Cell::new(false));
    {
        let updating = updating.clone();
        let window = context.window.clone();
        automatic.connect_active_notify(move |row| {
            if updating.get() {
                return;
            }
            let value = row.is_active();
            let window = window.clone();
            let row = row.downgrade();
            glib::spawn_future_local(async move {
                // timedated asks polkit; Asena's agent shows the prompt when needed.
                let result: zbus::Result<()> = async {
                    let connection = zbus::Connection::system().await?;
                    let proxy = zbus::Proxy::new(
                        &connection,
                        "org.freedesktop.timedate1",
                        "/org/freedesktop/timedate1",
                        "org.freedesktop.timedate1",
                    )
                    .await?;
                    proxy.call("SetNTP", &(value, true)).await
                }
                .await;
                if let Err(error) = result {
                    if let Some(row) = row.upgrade() {
                        toast(
                            &row,
                            &gettext("Could not change automatic time: {error}")
                                .replace("{error}", &error.to_string()),
                        );
                    }
                    let _ = window;
                }
            });
        });
    }
    let weak_zone = zone.downgrade();
    let weak_auto = automatic.downgrade();
    let update = move |model: &asena_services::property_model::PropertyModel| {
        if let (Some(zone), Some(automatic)) = (weak_zone.upgrade(), weak_auto.upgrade()) {
            if let Some(name) = model.string("Timezone").filter(|name| !name.is_empty()) {
                zone.set_text(&name);
            }
            updating.set(true);
            automatic.set_sensitive(model.available() && model.boolean("CanNTP") == Some(true));
            automatic.set_active(model.boolean("NTP") == Some(true));
            updating.set(false);
        }
    };
    update(&timedate);
    timedate.connect_changed(update);
    // Keep the model alive with the page.
    let keep = timedate.clone();
    page.connect_destroy(move |_| {
        let _ = &keep;
    });
    if let Ok(settings) = asena_core::settings::load("org.asenalinux.shell") {
        let format = group(&gettext("Panel clock"));
        rows::choice(
            &format,
            &settings,
            "clock-format",
            &gettext("Clock format"),
            &[("24h", gettext("24-hour")), ("12h", gettext("12-hour"))],
        );
        rows::boolean(
            &format,
            &settings,
            "clock-show-seconds",
            &gettext("Show seconds"),
        );
        page.add(&format);
    }
    page
}

/// A default application chooser for a group of content types.
fn default_app_row(group: &adw::PreferencesGroup, title: &str, types: &'static [&'static str]) {
    let Some(primary) = types.first() else {
        return;
    };
    let apps: Vec<gio::AppInfo> = gio::AppInfo::all_for_type(primary)
        .into_iter()
        .filter(gio::AppInfo::should_show)
        .collect();
    if apps.is_empty() {
        rows::info(group, title, &gettext("No application installed"));
        return;
    }
    let names: Vec<String> = apps
        .iter()
        .map(|app| app.display_name().to_string())
        .collect();
    let labels: Vec<&str> = names.iter().map(String::as_str).collect();
    let row = adw::ComboRow::builder()
        .title(title)
        .model(&gtk::StringList::new(&labels))
        .build();
    let current = gio::AppInfo::default_for_type(primary, false);
    if let Some(index) = current
        .as_ref()
        .and_then(|current| apps.iter().position(|app| app.id() == current.id()))
    {
        row.set_selected(index as u32);
    } else {
        row.set_selected(gtk::INVALID_LIST_POSITION);
    }
    row.connect_selected_notify(move |row| {
        if let Some(app) = apps.get(row.selected() as usize) {
            for content_type in types {
                if let Err(error) = app.set_as_default_for_type(content_type) {
                    tracing::warn!(%error, content_type, "Could not set default application");
                }
            }
        }
    });
    group.add(&row);
}

pub fn defaults(_context: &Context) -> adw::PreferencesPage {
    let page = page(
        &gettext("Default applications"),
        "preferences-desktop-default-applications-symbolic",
    );
    let group = adw::PreferencesGroup::new();
    default_app_row(
        &group,
        &gettext("Web browser"),
        &[
            "x-scheme-handler/https",
            "x-scheme-handler/http",
            "text/html",
        ],
    );
    default_app_row(&group, &gettext("Files"), &["inode/directory"]);
    default_app_row(&group, &gettext("Text"), &["text/plain"]);
    default_app_row(
        &group,
        &gettext("Images"),
        &["image/jpeg", "image/png", "image/webp"],
    );
    default_app_row(
        &group,
        &gettext("Music"),
        &["audio/mpeg", "audio/flac", "audio/ogg"],
    );
    default_app_row(
        &group,
        &gettext("Video"),
        &["video/mp4", "video/x-matroska", "video/webm"],
    );
    page.add(&group);
    if let Ok(settings) = asena_core::settings::load("org.asenalinux.desktop") {
        let terminal = adw::PreferencesGroup::new();
        rows::choice(
            &terminal,
            &settings,
            "terminal",
            &gettext("Terminal"),
            &[
                ("foot", "foot".to_string()),
                ("ptyxis", "Ptyxis".to_string()),
            ],
        );
        page.add(&terminal);
    }
    page
}

pub fn privacy(context: &Context) -> adw::PreferencesPage {
    let page = page(
        &gettext("Privacy and security"),
        "preferences-system-privacy-symbolic",
    );
    let history = group(&gettext("History"));
    let clear = rows::button(&history, &gettext("Recently used files"), &gettext("Clear"));
    clear.connect_clicked(|button| match gtk::RecentManager::default().purge_items() {
        Ok(count) => toast(
            button,
            &asena_core::i18n::ngettext(
                "{count} item removed",
                "{count} items removed",
                count.max(0) as u32,
            )
            .replace("{count}", &count.to_string()),
        ),
        Err(error) => toast(button, error.message()),
    });
    page.add(&history);
    let trash = group(&gettext("Trash"));
    let empty = rows::button(&trash, &gettext("Trash"), &gettext("Empty"));
    empty.add_css_class("destructive-action");
    let window = context.window.clone();
    empty.connect_clicked(move |button| {
        let dialog = adw::AlertDialog::new(
            Some(&gettext("Empty the trash?")),
            Some(&gettext(
                "All items in the trash will be deleted permanently.",
            )),
        );
        dialog.add_response("cancel", &gettext("Cancel"));
        dialog.add_response("empty", &gettext("Empty trash"));
        dialog.set_response_appearance("empty", adw::ResponseAppearance::Destructive);
        dialog.set_close_response("cancel");
        let button = button.downgrade();
        dialog.connect_response(Some("empty"), move |_, _| {
            let button = button.clone();
            glib::spawn_future_local(async move {
                let trash = gio::File::for_uri("trash:///");
                let result = async {
                    let children = trash
                        .enumerate_children_future(
                            "standard::name",
                            gio::FileQueryInfoFlags::NOFOLLOW_SYMLINKS,
                            glib::Priority::DEFAULT,
                        )
                        .await?;
                    loop {
                        let infos = children
                            .next_files_future(64, glib::Priority::DEFAULT)
                            .await?;
                        if infos.is_empty() {
                            break;
                        }
                        for info in infos {
                            trash
                                .child(info.name())
                                .delete_future(glib::Priority::DEFAULT)
                                .await?;
                        }
                    }
                    Ok::<_, glib::Error>(())
                }
                .await;
                if let Some(button) = button.upgrade() {
                    match result {
                        Ok(()) => toast(&button, &gettext("Trash emptied")),
                        Err(error) => toast(&button, error.message()),
                    }
                }
            });
        });
        dialog.present(Some(&window));
    });
    page.add(&trash);
    let lock = group(&gettext("Screen lock"));
    let row = adw::ActionRow::builder()
        .title(gettext("Lock and idle times"))
        .activatable(true)
        .build();
    row.add_suffix(&gtk::Image::from_icon_name("go-next-symbolic"));
    let open = context.open_panel.clone();
    row.connect_activated(move |_| open("power"));
    lock.add(&row);
    page.add(&lock);
    page
}

pub fn accessibility(context: &Context) -> adw::PreferencesPage {
    let page = page(
        &gettext("Accessibility"),
        "preferences-desktop-accessibility-symbolic",
    );
    let Ok(settings) = asena_core::settings::load("org.asenalinux.desktop") else {
        unavailable(&page, &gettext("Accessibility settings are unavailable"));
        return page;
    };
    let seeing = group(&gettext("Seeing"));
    rows::number(
        &seeing,
        &settings,
        "text-scaling-factor",
        &gettext("Text scale"),
        (0.75, 2.0, 0.05),
        &context.error,
    );
    page.add(&seeing);
    let motion = group(&gettext("Motion"));
    rows::boolean(
        &motion,
        &settings,
        "enable-animations",
        &gettext("Animations"),
    );
    page.add(&motion);
    page
}

pub fn about(_context: &Context) -> adw::PreferencesPage {
    let page = page(&gettext("About"), "help-about-symbolic");
    // The distribution's logo and name, both from os-release.
    let banner = gtk::Box::new(gtk::Orientation::Vertical, 12);
    banner.set_margin_bottom(12);
    let logo = gtk::Image::new();
    logo.set_pixel_size(96);
    logo.set_visible(false);
    let distribution = gtk::Label::new(None);
    distribution.add_css_class("title-2");
    distribution.set_wrap(true);
    banner.append(&logo);
    banner.append(&distribution);
    let header = adw::PreferencesGroup::new();
    header.add(&banner);
    page.add(&header);
    let device = group(&gettext("Device"));
    rows::info(&device, &gettext("Device name"), &glib::host_name());
    let system = rows::info(&device, &gettext("Operating system"), "");
    rows::info(
        &device,
        &gettext("Desktop"),
        &format!("Asena {} · Wayland", env!("CARGO_PKG_VERSION")),
    );
    let kernel = rows::info(&device, &gettext("Kernel"), "");
    page.add(&device);
    let hardware = group(&gettext("Hardware"));
    let processor = rows::info(&hardware, &gettext("Processor"), "");
    let memory = rows::info(&hardware, &gettext("Memory"), "");
    let disk = rows::info(&hardware, &gettext("Disk"), "");
    page.add(&hardware);
    glib::spawn_future_local(async move {
        let read = |path: &'static str| std::fs::read_to_string(path).unwrap_or_default();
        let (os, kernel_text, cpu, mem) = gio::spawn_blocking(move || {
            let os_release = read("/etc/os-release");
            (
                (
                    system_info::os_name(&os_release),
                    system_info::os_logo(&os_release),
                ),
                read("/proc/sys/kernel/osrelease").trim().to_owned(),
                system_info::processor(&read("/proc/cpuinfo")),
                system_info::memory_bytes(&read("/proc/meminfo")),
            )
        })
        .await
        .unwrap_or_default();
        let (os, os_logo) = os;
        let name = os.unwrap_or_default();
        system.set_text(&name);
        distribution.set_text(&name);
        distribution.set_visible(!name.is_empty());
        let theme = gtk::IconTheme::for_display(&logo.display());
        if let Some(icon) = os_logo.filter(|icon| theme.has_icon(icon)) {
            logo.set_icon_name(Some(&icon));
            logo.set_visible(true);
        }
        kernel.set_text(&kernel_text);
        processor.set_text(
            &cpu.map(|(model, count)| format!("{model} × {count}"))
                .unwrap_or_default(),
        );
        memory.set_text(
            &mem.map(|bytes| glib::format_size(bytes).to_string())
                .unwrap_or_default(),
        );
        if let Ok(info) = gio::File::for_path("/")
            .query_filesystem_info_future(
                "filesystem::size,filesystem::free",
                glib::Priority::DEFAULT,
            )
            .await
        {
            let size = info.attribute_uint64("filesystem::size");
            let free = info.attribute_uint64("filesystem::free");
            disk.set_text(
                &gettext("{free} free of {size}")
                    .replace("{free}", &glib::format_size(free))
                    .replace("{size}", &glib::format_size(size)),
            );
        }
    });
    page
}
