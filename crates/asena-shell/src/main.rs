use std::{cell::RefCell, rc::Rc};

use adw::prelude::*;
use asena_core::{i18n::gettext, layout::PanelLayout};
use asena_services::{app_catalog::AppCatalog, toplevel_model::ToplevelModel};
use gtk::gdk;
use gtk4_layer_shell::{Edge, KeyboardMode, Layer, LayerShell};

mod background;
mod bus;
mod clock;
mod desktop;
mod launch;
mod media;
mod menu;
mod notifications;
mod osd;
mod outputs;
mod power;
mod quick_settings;
mod search;
mod status;
mod surface;
mod taskbar;
mod taskbar_strip;
mod tray;

struct Components {
    services: asena_services::system::SystemServices,
    windows: ToplevelModel,
    apps: AppCatalog,
    settings: gio::Settings,
    desktop: gio::Settings,
    notification_settings: gio::Settings,
    notifications: asena_services::notifications::NotificationModel,
    commands: async_channel::Sender<bus::Command>,
}

struct Panel {
    monitor: gdk::Monitor,
    window: gtk::ApplicationWindow,
    background: gtk::ApplicationWindow,
}

fn sync_panels(
    app: &adw::Application,
    monitors: &gio::ListModel,
    panels: &RefCell<Vec<Panel>>,
    components: &Components,
) {
    let current: Vec<gdk::Monitor> = (0..monitors.n_items())
        .filter_map(|index| monitors.item(index)?.downcast::<gdk::Monitor>().ok())
        .collect();
    let mut panels = panels.borrow_mut();
    let all = components.settings.boolean("panel-on-all-monitors");
    let connector = components.settings.string("panel-monitor");
    let chosen = current
        .iter()
        .find(|monitor| monitor.connector().as_deref() == Some(connector.as_str()))
        .or_else(|| current.first())
        .cloned();
    panels.retain(|panel| {
        if current.contains(&panel.monitor) {
            true
        } else {
            panel.window.close();
            panel.background.close();
            false
        }
    });
    // Desktop icons live on the panel monitor; other monitors keep the menu only.
    let primary = chosen.clone();
    for monitor in current {
        if let Some(panel) = panels.iter().find(|panel| panel.monitor == monitor) {
            panel
                .window
                .set_visible(all || chosen.as_ref() == Some(&monitor));
            continue;
        }
        let window = create_panel(
            app,
            &monitor,
            PanelLayout::from_settings(&components.settings),
            components,
        );
        let background = background::create(
            app,
            &monitor,
            &components.desktop,
            primary.as_ref() == Some(&monitor),
        );
        background.present();
        if all || chosen.as_ref() == Some(&monitor) {
            window.present();
        }
        panels.push(Panel {
            monitor,
            window,
            background,
        });
    }
}

fn create_panel(
    app: &adw::Application,
    monitor: &gdk::Monitor,
    layout: PanelLayout,
    components: &Components,
) -> gtk::ApplicationWindow {
    let window = gtk::ApplicationWindow::builder()
        .application(app)
        .title(gettext("Asena"))
        .decorated(false)
        .build();
    // Layer shell must be initialized before realizing the GTK window.
    window.init_layer_shell();
    window.set_namespace(Some("asena-panel"));
    window.set_layer(Layer::Top);
    window.set_keyboard_mode(KeyboardMode::None);
    window.set_monitor(Some(monitor));
    for (edge, anchor) in [Edge::Top, Edge::Bottom, Edge::Left, Edge::Right]
        .into_iter()
        .zip(layout.anchors())
    {
        window.set_anchor(edge, anchor);
    }
    window.set_exclusive_zone(layout.thickness);
    let orientation = if layout.is_horizontal() {
        window.set_default_size(-1, layout.thickness);
        gtk::Orientation::Horizontal
    } else {
        window.set_default_size(layout.thickness, -1);
        gtk::Orientation::Vertical
    };
    let content = gtk::Box::new(orientation, 0);
    content.set_hexpand(true);
    content.set_vexpand(true);
    content.set_valign(gtk::Align::Fill);
    let start = command_button(
        "asena-start-symbolic",
        &gettext("Applications"),
        bus::Action::Menu,
        &components.commands,
    );
    start.add_css_class("asena-start");
    content.append(&start);
    content.append(&taskbar::create(
        &components.windows,
        &components.apps,
        &components.settings,
        layout,
    ));
    content.append(&tray::create(
        &components.services.tray,
        &window,
        monitor,
        layout,
    ));
    let commands = components.commands.clone();
    let status = status::create(
        &components.services,
        &components.notification_settings,
        orientation,
        move || {
            let (reply, _) = async_channel::bounded(1);
            let _ = commands.try_send(bus::Command {
                action: bus::Action::QuickSettings,
                reply,
            });
        },
    );
    content.append(&status);
    let clock = clock::create(&components.settings);
    let commands = components.commands.clone();
    clock.connect_clicked(move |_| {
        let (reply, _) = async_channel::bounded(1);
        let _ = commands.try_send(bus::Command {
            action: bus::Action::Notifications,
            reply,
        });
    });
    content.append(&clock);
    let unread = gtk::Label::new(None);
    unread.add_css_class("asena-unread");
    unread.add_css_class("numeric");
    components
        .notifications
        .bind_property("unread", &unread, "label")
        .transform_to(|_, value: u32| {
            Some(if value == 0 {
                String::new()
            } else {
                value.to_string()
            })
        })
        .sync_create()
        .build();
    unread
        .bind_property("label", &unread, "visible")
        .transform_to(|_, label: String| Some(!label.is_empty()))
        .sync_create()
        .build();
    content.append(&unread);
    window.set_child(Some(&content));
    window.add_css_class("asena-panel");
    window.add_css_class(match layout.edge {
        asena_core::layout::PanelEdge::Bottom => "edge-bottom",
        asena_core::layout::PanelEdge::Top => "edge-top",
        asena_core::layout::PanelEdge::Left => "edge-left",
        asena_core::layout::PanelEdge::Right => "edge-right",
    });
    window
}

fn command_button(
    icon: &str,
    label: &str,
    action: bus::Action,
    commands: &async_channel::Sender<bus::Command>,
) -> gtk::Button {
    let button = gtk::Button::from_icon_name(icon);
    button.add_css_class("flat");
    button.add_css_class("asena-panel-button");
    button.set_tooltip_text(Some(label));
    button.update_property(&[gtk::accessible::Property::Label(label)]);
    let commands = commands.clone();
    button.connect_clicked(move |_| {
        let (reply, _) = async_channel::bounded(1);
        let _ = commands.try_send(bus::Command {
            action: action.clone(),
            reply,
        });
    });
    button
}

fn main() -> anyhow::Result<()> {
    asena_core::logging::init("asena-shell").map_err(|e| anyhow::anyhow!("{e}"))?;
    asena_core::i18n::init().map_err(|e| anyhow::anyhow!("{e}"))?;
    // Validate schemas before GTK starts, never abort through g_settings_new().
    let settings = asena_core::settings::load(asena_core::settings::SHELL_SCHEMA)?;
    let desktop = asena_core::settings::load("org.asenalinux.desktop")?;
    let notification_settings = asena_core::settings::load("org.asenalinux.notifications")?;
    gtk::init()?;
    anyhow::ensure!(
        gtk4_layer_shell::is_supported(),
        gettext("Layer shell is unavailable. Start Asena in a supported Wayland session.")
    );
    let app = adw::Application::builder()
        .application_id(asena_core::dbus::SHELL_ID)
        // zbus owns the public name; GApplication must not acquire it first.
        .flags(gio::ApplicationFlags::NON_UNIQUE)
        .build();
    app.connect_startup(|app| {
        if let Err(error) = asena_widgets::init() {
            tracing::error!(%error, "Unable to load shell resources");
            app.quit();
        }
    });
    let panels = Rc::new(RefCell::new(Vec::<Panel>::new()));
    let active_panels = panels.clone();
    app.connect_activate(move |app| {
        for panel in active_panels.borrow().iter() {
            if panel.window.is_visible() {
                panel.window.present();
            }
        }
        tracing::debug!(
            application = app.application_id().as_deref(),
            "Shell activated"
        );
    });
    app.connect_startup(move |app| {
        let app = app.clone();
        let panels = panels.clone();
        let settings = settings.clone();
        let desktop = desktop.clone();
        let notification_settings = notification_settings.clone();
        glib::spawn_future_local(async move {
            let (commands, receiver) = async_channel::unbounded();
            let _connection = match bus::connect(commands.clone()).await {
                Ok(connection) => connection,
                Err(error) => {
                    tracing::error!(%error, "Could not own the shell bus name");
                    app.quit();
                    return;
                }
            };
            let services = asena_services::system::SystemServices::default();
            services.start();
            outputs::start();
            let components = Rc::new(Components {
                services: services.clone(),
                windows: ToplevelModel::default(),
                apps: AppCatalog::default(),
                settings,
                desktop,
                notification_settings: notification_settings.clone(),
                notifications: Default::default(),
                commands,
            });
            if let Err(error) = components.windows.start() {
                tracing::warn!(%error, "Could not start the window model");
            }
            components.apps.start();
            components.notifications.start();
            let menu = menu::Menu::new(
                &app,
                &components.apps,
                &components.settings,
                &services.user,
                &services.daemon,
            );
            let notifications = notifications::Notifications::new(
                &app,
                &components.notifications,
                &services.mpris,
                &notification_settings,
            );
            let osd = osd::Osd::new(&app, Default::default());
            let quick_settings = quick_settings::QuickSettings::new(
                &app,
                &services,
                &notification_settings,
                &components.commands,
            );
            let notices = notifications.clone();
            menu.connect_visibility(move |visible| notices.suppress_popups(visible));
            let notices = notifications.clone();
            quick_settings.connect_visibility(move |visible| notices.suppress_popups(visible));
            let Some(display) = gdk::Display::default() else {
                app.quit();
                return;
            };
            let monitors = display.monitors();
            sync_panels(&app, &monitors, &panels, &components);
            let rebuild = Rc::new(std::cell::Cell::new(false));
            let placed_notifications = notifications.clone();
            let display_monitors = monitors.clone();
            let changed_panels = panels.clone();
            let changed_components = components.clone();
            let changed_app = app.downgrade();
            components.settings.connect_changed(None, move |_, key| {
                if !matches!(
                    key,
                    "panel-edge"
                        | "panel-thickness"
                        | "icon-size"
                        | "panel-alignment"
                        | "panel-monitor"
                        | "panel-on-all-monitors"
                ) || rebuild.replace(true)
                {
                    return;
                }
                placed_notifications.sync();
                let pending = rebuild.clone();
                let panels = changed_panels.clone();
                let components = changed_components.clone();
                let monitors = display_monitors.clone();
                let app = changed_app.clone();
                glib::idle_add_local_once(move || {
                    pending.set(false);
                    if let Some(app) = app.upgrade() {
                        for panel in panels.borrow_mut().drain(..) {
                            panel.window.close();
                            panel.background.close();
                        }
                        sync_panels(&app, &monitors, &panels, &components);
                    }
                });
            });
            let weak_app = app.downgrade();
            let active_components = components.clone();
            let active_panels = panels.clone();
            let handler = monitors.connect_items_changed(move |monitors, _, _, _| {
                if let Some(app) = weak_app.upgrade() {
                    sync_panels(&app, monitors, &active_panels, &active_components);
                }
            });
            while let Ok(command) = receiver.recv().await {
                tracing::debug!("Shell command dispatched");
                let result = match command.action {
                    bus::Action::Brightness(step) => {
                        let model = services.brightness.clone();
                        let osd = osd.clone();
                        let reply = command.reply;
                        glib::spawn_future_local(async move {
                            let result = match model.set_level(model.level() + step).await {
                                Ok(value) => {
                                    osd.show("brightness", value);
                                    Ok(())
                                }
                                Err(error) => Err(error.to_string()),
                            };
                            let _ = reply.send(result).await;
                        });
                        continue;
                    }
                    bus::Action::Audio(change) => {
                        let kind = if matches!(change, asena_services::audio::Change::Microphone) {
                            "microphone"
                        } else {
                            "volume"
                        };
                        let audio = services.audio.clone();
                        let osd = osd.clone();
                        let reply = command.reply;
                        glib::spawn_future_local(async move {
                            let result = match audio.change(change).await {
                                Ok(value) => {
                                    osd.show(kind, value);
                                    Ok(())
                                }
                                Err(error) => Err(error.to_string()),
                            };
                            let _ = reply.send(result).await;
                        });
                        continue;
                    }
                    bus::Action::ReloadStyle => bus::reload_style().await,
                    bus::Action::Osd { kind, value } => {
                        osd.show(&kind, value);
                        Ok(())
                    }
                    bus::Action::Menu => {
                        quick_settings.hide();
                        notifications.hide();
                        menu.toggle();
                        Ok(())
                    }
                    bus::Action::Notifications => {
                        quick_settings.hide();
                        menu.hide();
                        notifications.toggle();
                        Ok(())
                    }
                    bus::Action::QuickSettings => {
                        menu.hide();
                        notifications.hide();
                        quick_settings.toggle();
                        Ok(())
                    }
                };
                let _ = command.reply.send(result).await;
            }
            monitors.disconnect(handler);
        });
    });
    // Survive a temporary zero-output topology while monitors are reconfigured.
    let _hold = app.hold();
    let status = app.run();
    anyhow::ensure!(
        status == glib::ExitCode::SUCCESS,
        "Shell exited with {status:?}"
    );
    Ok(())
}
