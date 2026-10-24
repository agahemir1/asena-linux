//! Printers through CUPS. Reading, the default printer and test pages use the
//! CUPS commands as the user; adding and removing go through the polkit-guarded
//! cups-pk-helper service. Printers are added driverless (IPP Everywhere).
use crate::pages::{toast, Context};
use adw::prelude::*;
use asena_core::i18n::gettext;
use std::{collections::HashMap, process::Command};

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Printer {
    pub name: String,
    pub enabled: bool,
    pub printing: bool,
}

/// `lpstat -p` in the C locale.
pub fn parse_printers(text: &str) -> Vec<Printer> {
    text.lines()
        .filter_map(|line| {
            let rest = line.strip_prefix("printer ")?;
            let (name, state) = rest.split_once(' ')?;
            Some(Printer {
                name: name.to_owned(),
                enabled: !state.contains("disabled"),
                printing: state.contains("now printing"),
            })
        })
        .collect()
}

/// `lpstat -d`: `system default destination: Office`.
pub fn parse_default(text: &str) -> Option<String> {
    text.lines()
        .find_map(|line| line.split_once("destination: "))
        .map(|(_, name)| name.trim().to_owned())
        .filter(|name| !name.is_empty())
}

/// Jobs per printer from `lpstat -o`: `Office-12 asena 1024 Thu 23 Oct ...`.
pub fn parse_jobs(text: &str, printers: &[Printer]) -> HashMap<String, usize> {
    let mut jobs = HashMap::new();
    for line in text.lines() {
        let Some(job) = line.split_whitespace().next() else {
            continue;
        };
        if let Some(printer) = printers.iter().find(|printer| {
            job.strip_prefix(printer.name.as_str())
                .and_then(|rest| rest.strip_prefix('-'))
                .is_some_and(|number| number.chars().all(|c| c.is_ascii_digit()))
        }) {
            *jobs.entry(printer.name.clone()).or_default() += 1;
        }
    }
    jobs
}

/// An address typed by the user, as an IPP URI.
pub fn uri_for(address: &str) -> Option<String> {
    let address = address.trim();
    if address.is_empty() || address.contains(char::is_whitespace) {
        return None;
    }
    if address.starts_with("ipp://") || address.starts_with("ipps://") {
        return Some(address.to_owned());
    }
    if address.contains("://") {
        return None;
    }
    Some(format!("ipp://{address}/ipp/print"))
}

/// CUPS queue names: letters, digits, `-` and `_`, at most 127 bytes.
pub fn queue_name(text: &str) -> String {
    let name: String = text
        .chars()
        .map(|c| {
            if c.is_ascii_alphanumeric() || c == '-' || c == '_' {
                c
            } else {
                '_'
            }
        })
        .take(127)
        .collect();
    let name = name.trim_matches('_').to_owned();
    if name.is_empty() {
        "Printer".into()
    } else {
        name
    }
}

/// Driverless devices from cups-pk-helper's `DevicesGet`, as (URI, description).
pub fn parse_devices(devices: &HashMap<String, String>) -> Vec<(String, String)> {
    let mut found: Vec<(String, String)> = devices
        .iter()
        .filter_map(|(key, uri)| {
            let index = key.strip_prefix("device-uri:")?;
            if !(uri.starts_with("ipp://") || uri.starts_with("ipps://")) {
                return None;
            }
            let info = devices
                .get(&format!("device-info:{index}"))
                .or_else(|| devices.get(&format!("device-make-and-model:{index}")))
                .cloned()
                .unwrap_or_else(|| uri.clone());
            Some((uri.clone(), info))
        })
        .collect();
    found.sort();
    found.dedup_by(|a, b| a.0 == b.0);
    found
}

fn run(program: &str, arguments: &[&str]) -> Option<String> {
    let output = Command::new(program)
        .args(arguments)
        .env("LC_ALL", "C")
        .output()
        .ok()?;
    output
        .status
        .success()
        .then(|| String::from_utf8_lossy(&output.stdout).into_owned())
}

struct Snapshot {
    available: bool,
    printers: Vec<Printer>,
    default: Option<String>,
    jobs: HashMap<String, usize>,
}

fn snapshot() -> Snapshot {
    // lpstat exits with an error when there are no printers.
    let Ok(output) = Command::new("lpstat").arg("-p").env("LC_ALL", "C").output() else {
        return Snapshot {
            available: false,
            printers: Vec::new(),
            default: None,
            jobs: HashMap::new(),
        };
    };
    let printers = parse_printers(&String::from_utf8_lossy(&output.stdout));
    let default = run("lpstat", &["-d"]).and_then(|text| parse_default(&text));
    let jobs = run("lpstat", &["-o"])
        .map(|text| parse_jobs(&text, &printers))
        .unwrap_or_default();
    Snapshot {
        available: true,
        printers,
        default,
        jobs,
    }
}

async fn helper(method: &str, parameters: glib::Variant) -> Result<glib::Variant, String> {
    let bus = gio::bus_get_future(gio::BusType::System)
        .await
        .map_err(|error| error.to_string())?;
    let reply = bus
        .call_future(
            Some("org.opensuse.CupsPkHelper.Mechanism"),
            "/",
            "org.opensuse.CupsPkHelper.Mechanism",
            method,
            Some(&parameters),
            None,
            gio::DBusCallFlags::ALLOW_INTERACTIVE_AUTHORIZATION,
            120_000,
        )
        .await
        .map_err(|error| error.to_string())?;
    // Every method answers with an error text first; empty means success.
    let error = reply
        .try_child_value(0)
        .and_then(|value| value.str().map(str::to_owned))
        .unwrap_or_default();
    if error.is_empty() {
        Ok(reply)
    } else {
        Err(error)
    }
}

pub fn page(context: &Context) -> adw::PreferencesPage {
    let page = adw::PreferencesPage::builder()
        .title(gettext("Printers"))
        .icon_name("printer-symbolic")
        .build();
    let group = adw::PreferencesGroup::builder()
        .title(gettext("Printers"))
        .build();
    let add = gtk::Button::from_icon_name("list-add-symbolic");
    add.add_css_class("flat");
    add.set_tooltip_text(Some(&gettext("Add printer")));
    group.set_header_suffix(Some(&add));
    page.add(&group);
    let rows: std::rc::Rc<std::cell::RefCell<Vec<gtk::Widget>>> = Default::default();
    let refresh: std::rc::Rc<dyn Fn()> = {
        let group = group.downgrade();
        let rows = rows.clone();
        std::rc::Rc::new(move || {
            let group = group.clone();
            let rows = rows.clone();
            glib::spawn_future_local(async move {
                let Ok(snapshot) = gio::spawn_blocking(snapshot).await else {
                    return;
                };
                let Some(group) = group.upgrade() else {
                    return;
                };
                for row in rows.borrow_mut().drain(..) {
                    group.remove(&row);
                }
                let mut added = Vec::new();
                if !snapshot.available {
                    let row = adw::ActionRow::builder()
                        .title(gettext("Printing is not installed"))
                        .build();
                    group.add(&row);
                    added.push(row.upcast());
                } else if snapshot.printers.is_empty() {
                    let row = adw::ActionRow::builder()
                        .title(gettext("No printers"))
                        .build();
                    group.add(&row);
                    added.push(row.upcast());
                }
                for printer in &snapshot.printers {
                    let row = printer_row(
                        printer,
                        snapshot.default.as_deref() == Some(printer.name.as_str()),
                        snapshot.jobs.get(&printer.name).copied().unwrap_or(0),
                    );
                    group.add(&row);
                    added.push(row.upcast());
                }
                rows.replace(added);
            });
        })
    };
    refresh();
    let window = context.window.clone();
    let after_add = refresh.clone();
    add.connect_clicked(move |_| add_dialog(&window, after_add.clone()));
    // Printer state changes outside Settings too; look again when the page shows.
    let on_map = refresh.clone();
    page.connect_map(move |_| on_map());
    page
}

fn printer_row(printer: &Printer, default: bool, jobs: usize) -> adw::ActionRow {
    let mut state = if !printer.enabled {
        gettext("Stopped")
    } else if printer.printing {
        gettext("Printing")
    } else {
        gettext("Ready")
    };
    if jobs > 0 {
        state.push_str(" · ");
        state.push_str(
            &asena_core::i18n::ngettext("{count} job", "{count} jobs", jobs as u32)
                .replace("{count}", &jobs.to_string()),
        );
    }
    if default {
        state.push_str(" · ");
        state.push_str(&gettext("Default"));
    }
    let row = adw::ActionRow::builder()
        .title(glib::markup_escape_text(&printer.name))
        .subtitle(state)
        .build();
    row.add_prefix(&gtk::Image::from_icon_name("printer-symbolic"));
    let menu = gio::Menu::new();
    let name = printer.name.to_variant();
    let item = |label: &str, action: &str| {
        let item = gio::MenuItem::new(Some(label), None);
        item.set_action_and_target_value(Some(action), Some(&name));
        menu.append_item(&item);
    };
    if !default {
        item(&gettext("Use as default"), "printers.default");
    }
    item(&gettext("Print test page"), "printers.test");
    if jobs > 0 {
        item(&gettext("Cancel jobs"), "printers.cancel");
    }
    item(&gettext("Remove"), "printers.remove");
    let button = gtk::MenuButton::new();
    button.set_icon_name("view-more-symbolic");
    button.add_css_class("flat");
    button.set_valign(gtk::Align::Center);
    button.set_tooltip_text(Some(&gettext("Printer actions")));
    button.set_menu_model(Some(&menu));
    row.add_suffix(&button);
    let actions = gio::SimpleActionGroup::new();
    let add = |name: &str, work: fn(String) -> Option<String>, done: String| {
        let action = gio::SimpleAction::new(name, Some(glib::VariantTy::STRING));
        let row = row.downgrade();
        action.connect_activate(move |_, parameter| {
            let Some(printer) = parameter.and_then(|value| value.str().map(str::to_owned)) else {
                return;
            };
            let row = row.clone();
            let done = done.clone();
            glib::spawn_future_local(async move {
                let result = gio::spawn_blocking(move || work(printer))
                    .await
                    .ok()
                    .flatten();
                if let Some(row) = row.upgrade() {
                    toast(
                        &row,
                        &if result.is_some() {
                            done
                        } else {
                            gettext("The printer did not respond")
                        },
                    );
                }
            });
        });
        actions.add_action(&action);
    };
    add(
        "default",
        |printer| run("lpoptions", &["-d", &printer]),
        gettext("Default printer changed"),
    );
    add(
        "test",
        |printer| {
            run(
                "lp",
                &[
                    "-d",
                    &printer,
                    "-t",
                    "Test page",
                    "/usr/share/cups/data/testprint",
                ],
            )
        },
        gettext("Test page sent"),
    );
    add(
        "cancel",
        |printer| run("cancel", &["-a", &printer]),
        gettext("Jobs cancelled"),
    );
    let remove = gio::SimpleAction::new("remove", Some(glib::VariantTy::STRING));
    let weak = row.downgrade();
    remove.connect_activate(move |_, parameter| {
        let Some(printer) = parameter.and_then(|value| value.str().map(str::to_owned)) else {
            return;
        };
        let weak = weak.clone();
        glib::spawn_future_local(async move {
            let result = helper("PrinterDelete", (printer.as_str(),).to_variant()).await;
            if let Some(row) = weak.upgrade() {
                match result {
                    Ok(_) => {
                        toast(&row, &gettext("Printer removed"));
                        row.set_visible(false);
                    }
                    Err(error) => {
                        tracing::warn!(%error, "Could not remove the printer");
                        toast(&row, &gettext("Could not remove the printer"));
                    }
                }
            }
        });
    });
    actions.add_action(&remove);
    row.insert_action_group("printers", Some(&actions));
    row
}

fn add_dialog(window: &gtk::Window, refresh: std::rc::Rc<dyn Fn()>) {
    let dialog = adw::Dialog::new();
    dialog.set_title(&gettext("Add printer"));
    dialog.set_content_width(420);
    let page = adw::PreferencesPage::new();
    let found = adw::PreferencesGroup::builder()
        .title(gettext("Found printers"))
        .build();
    let searching = adw::ActionRow::builder()
        .title(gettext("Looking for printers"))
        .build();
    searching.add_suffix(&adw::Spinner::new());
    found.add(&searching);
    let manual = adw::PreferencesGroup::builder()
        .title(gettext("Printer address"))
        .description(gettext(
            "For example 192.168.1.20 or ipp://printer.local/ipp/print",
        ))
        .build();
    let address = adw::EntryRow::builder().title(gettext("Address")).build();
    address.set_show_apply_button(true);
    manual.add(&address);
    page.add(&found);
    page.add(&manual);
    let toolbar = adw::ToolbarView::new();
    toolbar.add_top_bar(&adw::HeaderBar::new());
    toolbar.set_content(Some(&page));
    let overlay = adw::ToastOverlay::new();
    overlay.set_child(Some(&toolbar));
    dialog.set_child(Some(&overlay));
    dialog.present(Some(window));

    let add: std::rc::Rc<dyn Fn(String, String)> = {
        let dialog = dialog.downgrade();
        let overlay = overlay.downgrade();
        std::rc::Rc::new(move |uri: String, description: String| {
            let dialog = dialog.clone();
            let overlay = overlay.clone();
            let refresh = refresh.clone();
            glib::spawn_future_local(async move {
                let host = uri
                    .split("://")
                    .nth(1)
                    .and_then(|rest| rest.split(['/', ':']).next())
                    .unwrap_or_default()
                    .to_owned();
                let name = queue_name(if description.is_empty() {
                    &host
                } else {
                    &description
                });
                let result = helper(
                    "PrinterAdd",
                    (
                        name.as_str(),
                        uri.as_str(),
                        "everywhere",
                        description.as_str(),
                        "",
                    )
                        .to_variant(),
                )
                .await;
                let Some(dialog) = dialog.upgrade() else {
                    return;
                };
                match result {
                    Ok(_) => {
                        dialog.close();
                        refresh();
                    }
                    Err(error) => {
                        tracing::warn!(%error, "Could not add the printer");
                        if let Some(overlay) = overlay.upgrade() {
                            overlay.add_toast(adw::Toast::new(&gettext(
                                "The printer could not be added. Check that it supports driverless printing.",
                            )));
                        }
                    }
                }
            });
        })
    };
    let manual_add = add.clone();
    address.connect_apply(move |entry| match uri_for(&entry.text()) {
        Some(uri) => manual_add(uri, String::new()),
        None => entry.add_css_class("error"),
    });
    let found_ref = found.downgrade();
    glib::spawn_future_local(async move {
        let include: Vec<&str> = vec!["ipp", "ipps", "dnssd"];
        let exclude: Vec<&str> = Vec::new();
        let result = helper("DevicesGet", (10i32, -1i32, include, exclude).to_variant()).await;
        let Some(found) = found_ref.upgrade() else {
            return;
        };
        found.remove(&searching);
        let devices: Vec<(String, String)> = result
            .ok()
            .and_then(|reply| reply.try_child_value(1))
            .and_then(|value| value.get::<HashMap<String, String>>())
            .map(|devices| parse_devices(&devices))
            .unwrap_or_default();
        if devices.is_empty() {
            found.add(
                &adw::ActionRow::builder()
                    .title(gettext("No printers found on the network"))
                    .build(),
            );
        }
        for (uri, description) in devices {
            let row = adw::ActionRow::builder()
                .title(glib::markup_escape_text(&description))
                .subtitle(glib::markup_escape_text(&uri))
                .activatable(true)
                .build();
            let add = add.clone();
            row.connect_activated(move |_| add(uri.clone(), description.clone()));
            found.add(&row);
        }
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn printers_and_default_are_parsed() {
        let printers = parse_printers(
            "printer Office is idle.  enabled since Thu 23 Oct 2026\nprinter Lab now printing Lab-3.  enabled since Thu\nprinter Old disabled since Wed 22 Oct 2026 -\n\treason unknown\n",
        );
        assert_eq!(printers.len(), 3);
        assert!(printers[0].enabled && !printers[0].printing);
        assert!(printers[1].printing);
        assert!(!printers[2].enabled);
        assert_eq!(
            parse_default("system default destination: Office\n").as_deref(),
            Some("Office")
        );
        assert_eq!(parse_default("no system default destination\n"), None);
        let jobs = parse_jobs(
            "Lab-3 asena 1024 Thu\nLab-4 asena 2 Thu\nOffice-x a\n",
            &printers,
        );
        assert_eq!(jobs.get("Lab"), Some(&2));
        assert_eq!(jobs.get("Office"), None);
    }

    #[test]
    fn addresses_become_ipp_uris() {
        assert_eq!(
            uri_for("192.168.1.20").as_deref(),
            Some("ipp://192.168.1.20/ipp/print")
        );
        assert_eq!(
            uri_for(" ipps://printer.local/ipp/print ").as_deref(),
            Some("ipps://printer.local/ipp/print")
        );
        assert_eq!(uri_for("socket://10.0.0.2"), None);
        assert_eq!(uri_for("a b"), None);
    }

    #[test]
    fn queue_names_are_safe() {
        assert_eq!(queue_name("HP LaserJet (Ofis)"), "HP_LaserJet__Ofis");
        assert_eq!(queue_name("///"), "Printer");
    }

    #[test]
    fn only_driverless_devices_are_offered() {
        let devices = HashMap::from([
            (
                "device-uri:0".to_owned(),
                "ipp://10.0.0.5/ipp/print".to_owned(),
            ),
            ("device-info:0".to_owned(), "Office printer".to_owned()),
            ("device-uri:1".to_owned(), "usb://HP/LaserJet".to_owned()),
        ]);
        assert_eq!(
            parse_devices(&devices),
            [(
                "ipp://10.0.0.5/ipp/print".to_owned(),
                "Office printer".to_owned()
            )]
        );
    }
}
