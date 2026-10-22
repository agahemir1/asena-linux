use adw::prelude::*;
use asena_core::i18n::gettext;
use asena_services::system::SystemServices;
use std::{cell::RefCell, rc::Rc};

mod content;
mod displays;
mod pages;
mod rows;
mod system_info;

use content::Content;

const APPLICATION_ID: &str = "org.asenalinux.Settings";
const DBUS_PATH: &str = "/org/asenalinux/Settings";
const DBUS_INTERFACE: &str = "org.asenalinux.Settings";

const SETTINGS_INTERFACE_XML: &str = "<node><interface name='org.asenalinux.Settings'><method name='OpenPanel'><arg type='s' name='panel_id' direction='in'/></method></interface></node>";

struct AppState {
    window: adw::ApplicationWindow,
    content: Content,
    // Models stay alive for the window lifetime; pages only hold weak signal handlers.
    _services: SystemServices,
}

/// Selects a panel. `reveal` moves a collapsed (narrow) window to the content page.
pub fn open_panel(content: &Content, panel_id: &str, reveal: bool) -> bool {
    if let Some(panel) = asena_core::panels::find(panel_id) {
        let widgets = content.widgets();
        widgets.stack.set_visible_child_name(panel.id);
        widgets.page.set_title(&panel.name);
        if reveal {
            widgets.split.set_show_content(true);
        }

        let mut child = widgets.categories.first_child();
        while let Some(widget) = child {
            if widget.widget_name() == panel.id {
                if let Some(row) = widget.downcast_ref::<gtk::ListBoxRow>() {
                    widgets.categories.select_row(Some(row));
                }
                break;
            }
            child = widget.next_sibling();
        }
        true
    } else {
        false
    }
}

/// Builds one settings panel.
type PageBuilder = fn(&pages::Context) -> adw::PreferencesPage;

fn populate_panels(content: &Content, window: &adw::ApplicationWindow, services: &SystemServices) {
    let widgets = content.widgets();
    let weak = content.downgrade();
    let context = pages::Context {
        window: window.clone().upcast(),
        error: widgets.error.clone(),
        services: services.clone(),
        open_panel: Rc::new(move |id: &str| {
            if let Some(content) = weak.upgrade() {
                open_panel(&content, id, true);
            }
        }),
    };
    let builders: [(&str, PageBuilder); 17] = [
        ("appearance", pages::appearance),
        ("desktop", pages::desktop),
        ("notifications", pages::notifications),
        ("network", pages::network),
        ("bluetooth", pages::bluetooth),
        ("displays", displays::page),
        ("sound", pages::sound),
        ("keyboard", pages::keyboard),
        ("mouse", pages::mouse),
        ("power", pages::power),
        ("users", pages::users),
        ("region", pages::region),
        ("datetime", pages::datetime),
        ("defaults", pages::defaults),
        ("privacy", pages::privacy),
        ("accessibility", pages::accessibility),
        ("about", pages::about),
    ];
    for (id, build) in builders {
        widgets.stack.add_named(&build(&context), Some(id));
    }

    for panel in asena_core::panels::all() {
        let row = adw::ActionRow::builder()
            .title(&panel.name)
            .activatable(true)
            .build();
        row.add_prefix(&gtk::Image::from_icon_name(panel.icon));
        row.set_widget_name(panel.id);
        widgets.categories.append(&row);
    }
    // Group headers: shown above the first visible row of each group.
    widgets.categories.set_header_func(|row, before| {
        let group = |row: &gtk::ListBoxRow| {
            asena_core::panels::find(&row.widget_name()).map(|panel| panel.group)
        };
        let current = group(row);
        if current.is_some() && before.and_then(group) != current {
            let label = gtk::Label::new(current.map(|group| group.name()).as_deref());
            label.set_xalign(0.0);
            label.add_css_class("heading");
            label.add_css_class("dim-label");
            label.set_margin_start(12);
            label.set_margin_top(if before.is_some() { 12 } else { 4 });
            label.set_margin_bottom(4);
            row.set_header(Some(&label));
        } else {
            row.set_header(None::<&gtk::Widget>);
        }
    });

    let search_entry = widgets.search.clone();
    widgets.categories.set_filter_func(move |row| {
        let query = search_entry.text();
        query.is_empty()
            || asena_core::panels::find(&row.widget_name())
                .is_some_and(|panel| panel.matches(&query))
    });
    let categories_clone = widgets.categories.clone();
    widgets.search.connect_search_changed(move |_| {
        categories_clone.invalidate_filter();
        categories_clone.invalidate_headers();
    });

    let stack_clone = widgets.stack.clone();
    let page_clone = widgets.page.clone();
    let split_clone = widgets.split.clone();
    widgets.categories.connect_row_activated(move |_, row| {
        let panel_id = row.widget_name();
        if let Some(panel) = asena_core::panels::find(&panel_id) {
            stack_clone.set_visible_child_name(&panel_id);
            page_clone.set_title(&panel.name);
            split_clone.set_show_content(true);
        }
    });

    open_panel(content, "appearance", false);
}

fn setup_dbus(app: &adw::Application, state: Rc<RefCell<Option<AppState>>>) -> anyhow::Result<()> {
    let connection = match app.dbus_connection() {
        Some(conn) => conn,
        None => return Ok(()),
    };

    let node_info = gio::DBusNodeInfo::for_xml(SETTINGS_INTERFACE_XML)
        .map_err(|e| anyhow::anyhow!("DBus XML error: {e}"))?;
    let iface = node_info
        .lookup_interface(DBUS_INTERFACE)
        .ok_or_else(|| anyhow::anyhow!("Interface not found in XML"))?;

    let state_clone = state;
    connection
        .register_object(DBUS_PATH, &iface)
        .method_call(
            move |_connection, _sender, _path, _iface, method, parameters, invocation| {
                if method == "OpenPanel" {
                    if let Some(panel_id) = parameters.child_value(0).str() {
                        if let Some(ref st) = *state_clone.borrow() {
                            if open_panel(&st.content, panel_id, true) {
                                st.window.present();
                                invocation.return_value(None);
                                return;
                            }
                        }
                    }
                    invocation.return_error(gio::IOErrorEnum::InvalidArgument, "Unknown panel id");
                }
            },
        )
        .build()
        .map_err(|e| anyhow::anyhow!("Failed to register DBus object: {e}"))?;

    Ok(())
}

fn build_ui(app: &adw::Application, state: Rc<RefCell<Option<AppState>>>) {
    if state.borrow().is_some() {
        return;
    }

    let content = Content::default();
    let window = adw::ApplicationWindow::builder()
        .application(app)
        .title(gettext("Settings"))
        .default_width(860)
        .default_height(600)
        .content(&content)
        .build();

    let services = SystemServices::default();
    services.start_preferences();
    populate_panels(&content, &window, &services);

    window.present();

    *state.borrow_mut() = Some(AppState {
        window,
        content,
        _services: services,
    });
}

fn main() -> anyhow::Result<()> {
    asena_core::logging::init("asena-settings").map_err(|e| anyhow::anyhow!("{e}"))?;
    asena_core::i18n::init().map_err(|e| anyhow::anyhow!("{e}"))?;

    let app = adw::Application::builder()
        .application_id(APPLICATION_ID)
        .flags(gio::ApplicationFlags::HANDLES_COMMAND_LINE)
        .build();

    let state = Rc::new(RefCell::new(None::<AppState>));

    {
        let state = state.clone();
        app.connect_startup(move |app| {
            if let Err(error) = asena_widgets::init() {
                tracing::error!(%error, "Failed to initialize widgets resources");
                return;
            }
            build_ui(app, state.clone());
            if let Err(error) = setup_dbus(app, state.clone()) {
                tracing::warn!(%error, "Could not register Settings D-Bus service");
            }
        });
    }

    {
        let state = state.clone();
        app.connect_activate(move |app| {
            build_ui(app, state.clone());
            if let Some(ref st) = *state.borrow() {
                st.window.present();
            }
        });
    }

    {
        let state = state.clone();
        app.connect_command_line(move |app, cmdline| {
            build_ui(app, state.clone());

            let args = cmdline.arguments();
            let mut target_panel = None;
            let mut unknown_arg = None;

            for arg in args.into_iter().skip(1) {
                let s = arg.to_string_lossy();
                if s.starts_with('-') {
                    continue;
                }
                if asena_core::panels::find(&s).is_some() {
                    target_panel = Some(s.to_string());
                } else {
                    unknown_arg = Some(s.to_string());
                }
                break;
            }

            if let Some(unknown) = unknown_arg {
                eprintln!("Unknown panel '{unknown}'. Valid panels are:");
                for p in asena_core::panels::all() {
                    eprintln!("  {}", p.id);
                }
                return 1;
            }

            if let Some(ref st) = *state.borrow() {
                if let Some(panel_id) = target_panel {
                    open_panel(&st.content, &panel_id, true);
                }
                st.window.present();
            }

            0
        });
    }

    app.run();
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_dbus_and_app_constants() {
        assert_eq!(APPLICATION_ID, "org.asenalinux.Settings");
        assert_eq!(DBUS_INTERFACE, "org.asenalinux.Settings");
        assert_eq!(DBUS_PATH, "/org/asenalinux/Settings");
    }

    #[test]
    fn test_all_17_panels_have_valid_metadata() {
        let panels = asena_core::panels::all();
        assert_eq!(panels.len(), 17);
        for p in &panels {
            assert!(!p.id.is_empty());
            assert!(!p.name.is_empty());
            assert!(!p.icon.is_empty());
            assert!(asena_core::panels::find(p.id).is_some());
        }
    }

    #[test]
    fn test_panel_routes_and_search() {
        let appearance = asena_core::panels::find("appearance");
        assert!(appearance.is_some());
        if let Some(panel) = appearance {
            assert!(panel.matches("appearance"));
            assert!(panel.matches("theme"));
            assert!(panel.matches("color"));
        }

        let unknown = asena_core::panels::find("invalid_panel_id");
        assert!(unknown.is_none());
    }
}
