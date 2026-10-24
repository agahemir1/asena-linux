//! Applications: what installed Flatpak applications may reach. Sandbox
//! permissions are user overrides in `~/.local/share/flatpak/overrides`, the
//! same files `flatpak override --user` writes; decisions made in portal
//! dialogs live in the portal permission store.
use crate::pages::{toast, Context as PageContext};
use adw::prelude::*;
use asena_core::i18n::gettext;
use std::path::{Path, PathBuf};

const KEYS: [&str; 4] = ["shared", "sockets", "devices", "filesystems"];

/// The `[Context]` lists of Flatpak metadata or an override file.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Sandbox {
    pub lists: [Vec<String>; 4],
}

impl Sandbox {
    pub fn parse(text: &str) -> Self {
        let mut sandbox = Self::default();
        let mut in_context = false;
        for line in text.lines() {
            let line = line.trim();
            if line.starts_with('[') {
                in_context = line == "[Context]";
                continue;
            }
            let Some((key, value)) = line.split_once('=').filter(|_| in_context) else {
                continue;
            };
            if let Some(index) = KEYS.iter().position(|known| *known == key.trim()) {
                sandbox.lists[index] = value
                    .split(';')
                    .map(str::trim)
                    .filter(|item| !item.is_empty())
                    .map(str::to_owned)
                    .collect();
            }
        }
        sandbox
    }

    /// `other` on top: `!item` removes, `item` adds.
    pub fn with(&self, other: &Self) -> Self {
        let mut result = self.clone();
        for (list, changes) in result.lists.iter_mut().zip(&other.lists) {
            for change in changes {
                match change.strip_prefix('!') {
                    Some(removed) => list.retain(|item| {
                        item != removed && !item.starts_with(&format!("{removed}:"))
                    }),
                    None => {
                        if !list.contains(change) {
                            list.push(change.clone());
                        }
                    }
                }
            }
        }
        result
    }

    pub fn has(&self, permission: Permission) -> bool {
        let (index, items) = permission.items();
        items.iter().any(|item| {
            self.lists[index]
                .iter()
                .any(|have| have == item || have.starts_with(&format!("{item}:")))
        })
    }

    /// Sets `permission` in a user override so that `base` plus the override has it or not.
    pub fn set(&mut self, base: &Self, permission: Permission, allowed: bool) {
        let (index, items) = permission.items();
        let list = &mut self.lists[index];
        list.retain(|item| {
            let plain = item.strip_prefix('!').unwrap_or(item);
            !items
                .iter()
                .any(|item| plain == *item || plain.starts_with(&format!("{item}:")))
        });
        let granted = base.has(permission);
        if allowed && !granted {
            list.push(items[0].to_owned());
        } else if !allowed && granted {
            list.extend(items.iter().map(|item| format!("!{item}")));
        }
    }

    #[cfg(test)]
    pub fn to_keyfile(&self) -> String {
        let mut text = String::from("[Context]\n");
        for (key, list) in KEYS.iter().zip(&self.lists) {
            if !list.is_empty() {
                text.push_str(&format!("{key}={};\n", list.join(";")));
            }
        }
        text
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Permission {
    Network,
    Sound,
    Devices,
    Files,
}

impl Permission {
    pub const ALL: [Self; 4] = [Self::Network, Self::Sound, Self::Devices, Self::Files];

    /// The list it lives in and its entries; the first is the one added.
    fn items(self) -> (usize, &'static [&'static str]) {
        match self {
            Self::Network => (0, &["network"]),
            Self::Sound => (1, &["pulseaudio"]),
            Self::Devices => (2, &["all"]),
            Self::Files => (3, &["home", "host"]),
        }
    }

    fn title(self) -> String {
        match self {
            Self::Network => gettext("Network"),
            Self::Sound => gettext("Sound and microphone"),
            Self::Devices => gettext("All devices, such as cameras"),
            Self::Files => gettext("Home folder or host files"),
        }
    }
}

/// Decisions from portal dialogs, as (table, id) in the permission store.
const PORTALS: [(&str, &str); 4] = [
    ("devices", "camera"),
    ("location", "location"),
    ("background", "background"),
    ("notifications", "notification"),
];

fn portal_title(id: &str) -> String {
    match id {
        "camera" => gettext("Camera"),
        "location" => gettext("Location"),
        "background" => gettext("Run in the background"),
        _ => gettext("Notifications"),
    }
}

struct Installed {
    id: String,
    metadata: PathBuf,
}

fn installed_apps() -> Vec<Installed> {
    let mut apps = Vec::new();
    let roots = [
        glib::user_data_dir().join("flatpak/app"),
        PathBuf::from("/var/lib/flatpak/app"),
    ];
    for root in roots {
        let Ok(entries) = std::fs::read_dir(&root) else {
            continue;
        };
        for entry in entries.flatten() {
            let Some(id) = entry.file_name().to_str().map(str::to_owned) else {
                continue;
            };
            let metadata = entry.path().join("current/active/metadata");
            if metadata.is_file() && !apps.iter().any(|app: &Installed| app.id == id) {
                apps.push(Installed { id, metadata });
            }
        }
    }
    apps
}

fn override_path(id: &str) -> PathBuf {
    glib::user_data_dir().join("flatpak/overrides").join(id)
}

/// The application's metadata with the system overrides applied.
fn base_sandbox(id: &str, metadata: &Path) -> Sandbox {
    let read = |path: &Path| std::fs::read_to_string(path).unwrap_or_default();
    Sandbox::parse(&read(metadata))
        .with(&Sandbox::parse(&read(Path::new(
            "/var/lib/flatpak/overrides/global",
        ))))
        .with(&Sandbox::parse(&read(
            &Path::new("/var/lib/flatpak/overrides").join(id),
        )))
        .with(&Sandbox::parse(&read(
            &glib::user_data_dir().join("flatpak/overrides/global"),
        )))
}

fn save_override(path: &Path, sandbox: &Sandbox) -> Result<(), glib::Error> {
    let file = glib::KeyFile::new();
    if path.exists() {
        file.load_from_file(path, glib::KeyFileFlags::KEEP_COMMENTS)?;
    }
    for (key, values) in KEYS.iter().zip(&sandbox.lists) {
        if values.is_empty() {
            let _ = file.remove_key("Context", key);
        } else {
            file.set_string("Context", key, &format!("{};", values.join(";")));
        }
    }
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)
            .map_err(|error| glib::Error::new(gio::IOErrorEnum::Failed, &error.to_string()))?;
    }
    gio::File::for_path(path).replace_contents(
        file.to_data().as_bytes(),
        None,
        false,
        gio::FileCreateFlags::PRIVATE,
        gio::Cancellable::NONE,
    )?;
    Ok(())
}

async fn store_call(method: &str, parameters: glib::Variant) -> Result<glib::Variant, glib::Error> {
    let bus = gio::bus_get_future(gio::BusType::Session).await?;
    bus.call_future(
        Some("org.freedesktop.impl.portal.PermissionStore"),
        "/org/freedesktop/impl/portal/PermissionStore",
        "org.freedesktop.impl.portal.PermissionStore",
        method,
        Some(&parameters),
        None,
        gio::DBusCallFlags::NONE,
        5000,
    )
    .await
}

/// The stored portal decision of `app`, if it ever was asked.
async fn portal_decision(table: &str, id: &str, app: &str) -> Option<bool> {
    let reply = store_call("Lookup", (table, id).to_variant()).await.ok()?;
    let (permissions, _): (
        std::collections::HashMap<String, Vec<String>>,
        glib::Variant,
    ) = reply.get()?;
    let values = permissions.get(app)?;
    let first = values.first()?.as_str();
    Some(!matches!(first, "no" | "NONE"))
}

fn portal_value(table: &str, allowed: bool) -> Vec<String> {
    match (table, allowed) {
        ("location", true) => vec!["EXACT".into(), "0".into()],
        ("location", false) => vec!["NONE".into(), "0".into()],
        (_, true) => vec!["yes".into()],
        (_, false) => vec!["no".into()],
    }
}

pub fn page(context: &PageContext) -> adw::PreferencesPage {
    let page = adw::PreferencesPage::builder()
        .title(gettext("Applications"))
        .icon_name("view-app-grid-symbolic")
        .build();
    let group = adw::PreferencesGroup::builder()
        .title(gettext("Flatpak applications"))
        .build();
    page.add(&group);
    let empty = adw::ActionRow::builder()
        .title(gettext("No Flatpak applications are installed"))
        .build();
    group.add(&empty);
    let window = context.window.clone();
    let list = group.downgrade();
    glib::spawn_future_local(async move {
        let apps = gio::spawn_blocking(installed_apps)
            .await
            .unwrap_or_default();
        let Some(group) = list.upgrade() else {
            return;
        };
        let mut rows: Vec<(String, adw::ActionRow)> = apps
            .into_iter()
            .map(|app| {
                let info = gio::DesktopAppInfo::new(&format!("{}.desktop", app.id));
                let name = info
                    .as_ref()
                    .map(|info| info.display_name().to_string())
                    .unwrap_or_else(|| app.id.clone());
                let row = adw::ActionRow::builder()
                    .title(glib::markup_escape_text(&name))
                    .subtitle(glib::markup_escape_text(&app.id))
                    .activatable(true)
                    .build();
                let icon = gtk::Image::new();
                icon.set_pixel_size(32);
                if let Some(gicon) = info.as_ref().and_then(|info| info.icon()) {
                    icon.set_from_gicon(&gicon);
                }
                row.add_prefix(&icon);
                row.add_suffix(&gtk::Image::from_icon_name("go-next-symbolic"));
                let window = window.clone();
                let title = name.clone();
                row.connect_activated(move |_| {
                    open_app(&window, &title, &app.id, app.metadata.clone());
                });
                (name, row)
            })
            .collect();
        rows.sort_by(|(a, _), (b, _)| {
            glib::CollationKey::from(a.as_str()).cmp(&glib::CollationKey::from(b.as_str()))
        });
        if !rows.is_empty() {
            group.remove(&empty);
        }
        for (_, row) in rows {
            group.add(&row);
        }
    });
    page
}

fn open_app(window: &gtk::Window, name: &str, id: &str, metadata: PathBuf) {
    let dialog = adw::PreferencesDialog::new();
    dialog.set_title(name);
    let page = adw::PreferencesPage::new();
    let sandbox_group = adw::PreferencesGroup::builder()
        .title(gettext("Access"))
        .build();
    let portal_group = adw::PreferencesGroup::builder()
        .title(gettext("Asked permissions"))
        .build();
    portal_group.set_visible(false);
    let reset_group = adw::PreferencesGroup::new();
    let reset = gtk::Button::with_label(&gettext("Reset permissions"));
    reset.add_css_class("pill");
    reset.set_halign(gtk::Align::Center);
    reset_group.add(&reset);
    page.add(&sandbox_group);
    page.add(&portal_group);
    page.add(&reset_group);
    dialog.add(&page);
    dialog.present(Some(window));

    let id = id.to_owned();
    glib::spawn_future_local(async move {
        let lookup = (id.clone(), metadata.clone());
        let Ok((base, user)) = gio::spawn_blocking(move || {
            let base = base_sandbox(&lookup.0, &lookup.1);
            let user = Sandbox::parse(
                &std::fs::read_to_string(override_path(&lookup.0)).unwrap_or_default(),
            );
            (base, user)
        })
        .await
        else {
            return;
        };
        let state = std::rc::Rc::new(std::cell::RefCell::new(user));
        let effective = base.with(&state.borrow());
        for permission in Permission::ALL {
            let row = adw::SwitchRow::builder()
                .title(permission.title())
                .active(effective.has(permission))
                .build();
            let state = state.clone();
            let base = base.clone();
            let id = id.clone();
            let group = sandbox_group.downgrade();
            row.connect_active_notify(move |row| {
                if let Some(group) = group.upgrade() {
                    group.set_sensitive(false);
                }
                state.borrow_mut().set(&base, permission, row.is_active());
                let sandbox = state.borrow().clone();
                let path = override_path(&id);
                let row = row.downgrade();
                let group = group.clone();
                glib::spawn_future_local(async move {
                    let result = gio::spawn_blocking(move || save_override(&path, &sandbox)).await;
                    if let (Ok(Err(error)), Some(row)) = (result, row.upgrade()) {
                        toast(&row, &error.to_string());
                    }
                    if let Some(group) = group.upgrade() {
                        group.set_sensitive(true);
                    }
                });
            });
            sandbox_group.add(&row);
        }
        let note = gtk::Label::new(Some(&gettext(
            "Changes apply the next time the application starts.",
        )));
        note.add_css_class("dim-label");
        note.add_css_class("caption");
        note.set_xalign(0.0);
        note.set_margin_top(6);
        note.set_wrap(true);
        sandbox_group.add(&note);

        for (table, key) in PORTALS {
            let Some(allowed) = portal_decision(table, key, &id).await else {
                continue;
            };
            let row = adw::SwitchRow::builder()
                .title(portal_title(key))
                .active(allowed)
                .build();
            let app = id.clone();
            row.connect_active_notify(move |row| {
                let parameters = (
                    table,
                    true,
                    key,
                    app.as_str(),
                    portal_value(table, row.is_active()),
                )
                    .to_variant();
                glib::spawn_future_local(async move {
                    if let Err(error) = store_call("SetPermission", parameters).await {
                        tracing::warn!(%error, "Could not change a portal permission");
                    }
                });
            });
            portal_group.add(&row);
            portal_group.set_visible(true);
        }

        let dialog_ref = dialog.downgrade();
        reset.connect_clicked(move |button| {
            let id = id.clone();
            let button = button.downgrade();
            let dialog_ref = dialog_ref.clone();
            glib::spawn_future_local(async move {
                let path = override_path(&id);
                let _ = gio::spawn_blocking(move || std::fs::remove_file(path)).await;
                for (table, key) in PORTALS {
                    let _ = store_call("DeletePermission", (table, key, id.as_str()).to_variant())
                        .await;
                }
                if let Some(button) = button.upgrade() {
                    toast(&button, &gettext("Permissions reset"));
                }
                if let Some(dialog) = dialog_ref.upgrade() {
                    dialog.close();
                }
            });
        });
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    const METADATA: &str = "[Application]\nname=org.example.App\n\n[Context]\nshared=network;ipc;\nsockets=x11;wayland;pulseaudio;\ndevices=dri;\nfilesystems=home;xdg-download;\n";

    #[test]
    fn changing_permissions_preserves_other_override_sections(
    ) -> Result<(), Box<dyn std::error::Error>> {
        let directory =
            std::env::temp_dir().join(format!("asena-override-{}", glib::uuid_string_random()));
        std::fs::create_dir(&directory)?;
        let path = directory.join("org.example.App");
        std::fs::write(
            &path,
            "[Context]\nfilesystems=home:ro;\n[Session Bus Policy]\norg.example.Service=talk\n",
        )?;
        let mut sandbox = Sandbox::parse(&std::fs::read_to_string(&path)?);
        sandbox.set(&Sandbox::parse(METADATA), Permission::Files, false);
        save_override(&path, &sandbox)?;
        let file = glib::KeyFile::new();
        file.load_from_file(&path, glib::KeyFileFlags::NONE)?;
        assert_eq!(
            file.string("Session Bus Policy", "org.example.Service")?,
            "talk"
        );
        let effective =
            Sandbox::parse(METADATA).with(&Sandbox::parse(&std::fs::read_to_string(&path)?));
        assert!(!effective.has(Permission::Files));
        std::fs::remove_dir_all(directory)?;
        Ok(())
    }

    #[test]
    fn metadata_lists_are_read() {
        let sandbox = Sandbox::parse(METADATA);
        assert!(sandbox.has(Permission::Network));
        assert!(sandbox.has(Permission::Sound));
        assert!(!sandbox.has(Permission::Devices));
        assert!(sandbox.has(Permission::Files));
    }

    #[test]
    fn overrides_remove_and_add() {
        let base = Sandbox::parse(METADATA);
        let user = Sandbox::parse("[Context]\nshared=!network;\ndevices=all;\n");
        let effective = base.with(&user);
        assert!(!effective.has(Permission::Network));
        assert!(effective.has(Permission::Devices));
        assert!(effective.lists[0].contains(&"ipc".to_owned()));
    }

    #[test]
    fn setting_writes_only_the_difference() {
        let base = Sandbox::parse(METADATA);
        let mut user = Sandbox::default();
        user.set(&base, Permission::Files, false);
        assert_eq!(user.lists[3], ["!home", "!host"]);
        assert!(!base.with(&user).has(Permission::Files));
        user.set(&base, Permission::Files, true);
        assert!(user.lists[3].is_empty());
        user.set(&base, Permission::Devices, true);
        assert_eq!(user.to_keyfile(), "[Context]\ndevices=all;\n");
    }

    #[test]
    fn host_access_counts_as_files() {
        let sandbox = Sandbox::parse("[Context]\nfilesystems=host:ro;\n");
        assert!(sandbox.has(Permission::Files));
    }

    #[test]
    fn portal_values_per_table() {
        assert_eq!(portal_value("location", false)[0], "NONE");
        assert_eq!(portal_value("devices", true), ["yes"]);
    }
}
