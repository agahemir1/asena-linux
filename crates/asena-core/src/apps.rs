//! Desktop metadata snapshots. Load on a worker; resolve without filesystem I/O.
use std::{collections::BTreeMap, path::PathBuf};

use gio::prelude::*;

use crate::text::fold;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DesktopEntry {
    pub id: String,
    pub name: String,
    pub icon: Option<String>,
    pub filename: PathBuf,
    pub startup_wm_class: Option<String>,
    pub generic_name: Option<String>,
    pub keywords: Vec<String>,
    /// freedesktop.org menu categories in desktop file order.
    pub categories: Vec<String>,
    pub visible: bool,
    pub commandline: String,
}

impl DesktopEntry {
    pub fn from_app(app: &gio::DesktopAppInfo) -> Option<Self> {
        Some(Self {
            id: app.id()?.to_string(),
            name: app.display_name().to_string(),
            icon: app.icon().and_then(|icon| icon.to_string()).map(Into::into),
            filename: app.filename()?,
            startup_wm_class: app.startup_wm_class().map(Into::into),
            generic_name: app.generic_name().map(Into::into),
            keywords: app.keywords().iter().map(ToString::to_string).collect(),
            categories: app
                .categories()
                .map(|value| {
                    value
                        .split(';')
                        .filter(|item| !item.is_empty())
                        .map(ToOwned::to_owned)
                        .collect()
                })
                .unwrap_or_default(),
            visible: app.should_show(),
            commandline: app
                .commandline()
                .map(|line| line.to_string_lossy().into_owned())
                .unwrap_or_default(),
        })
    }
}

#[derive(Clone, Debug, Default)]
pub struct AppIndex {
    entries: Vec<DesktopEntry>,
}

impl AppIndex {
    pub fn new(entries: impl IntoIterator<Item = DesktopEntry>) -> Self {
        // GIO provides XDG precedence. Keep its first entry for each desktop ID.
        let mut unique = BTreeMap::new();
        for entry in entries {
            unique.entry(entry.id.clone()).or_insert(entry);
        }
        Self {
            entries: unique.into_values().collect(),
        }
    }

    /// GIO handles XDG directories, desktop overrides and Flatpak exports in
    /// XDG_DATA_DIRS. AppInfoMonitor notifies callers when its cache changes.
    pub fn load() -> Self {
        Self::new(gio::AppInfo::all().into_iter().filter_map(|app| {
            app.downcast::<gio::DesktopAppInfo>()
                .ok()
                .and_then(|app| DesktopEntry::from_app(&app))
        }))
    }

    pub fn entries(&self) -> &[DesktopEntry] {
        &self.entries
    }

    pub fn resolve(&self, app_id: &str) -> Option<&DesktopEntry> {
        if app_id.is_empty() {
            return None;
        }
        let id = without_suffix(app_id);
        self.entries
            .iter()
            .find(|entry| without_suffix(&entry.id) == id)
            .or_else(|| {
                self.entries
                    .iter()
                    .find(|entry| entry.startup_wm_class.as_deref() == Some(app_id))
            })
            .or_else(|| {
                let key = fold(id);
                self.entries
                    .iter()
                    .find(|entry| fold(without_suffix(&entry.id)) == key)
                    .or_else(|| {
                        self.entries.iter().find(|entry| {
                            entry
                                .startup_wm_class
                                .as_deref()
                                .is_some_and(|class| fold(class) == key)
                        })
                    })
            })
            .or_else(|| {
                let key = fold(id.rsplit('.').next().unwrap_or(id));
                self.entries.iter().find(|entry| {
                    let id = without_suffix(&entry.id);
                    fold(id.rsplit('.').next().unwrap_or(id)) == key
                })
            })
    }
}

fn without_suffix(id: &str) -> &str {
    id.strip_suffix(".desktop").unwrap_or(id)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn entry(id: &str, class: Option<&str>) -> DesktopEntry {
        DesktopEntry {
            id: id.into(),
            name: id.into(),
            icon: None,
            filename: PathBuf::from(id),
            startup_wm_class: class.map(Into::into),
            generic_name: None,
            keywords: vec![],
            categories: vec![],
            visible: true,
            commandline: String::new(),
        }
    }

    #[test]
    fn exact_id_precedes_class_and_suffix_heuristics() {
        let index = AppIndex::new([
            entry("org.gnome.Nautilus.desktop", None),
            entry("other.desktop", Some("org.gnome.Nautilus")),
            entry("Nautilus.desktop", None),
        ]);
        assert_eq!(
            index
                .resolve("org.gnome.Nautilus")
                .map(|entry| entry.id.as_str()),
            Some("org.gnome.Nautilus.desktop")
        );
        assert_eq!(
            index
                .resolve("org.gnome.Nautilus.desktop")
                .map(|entry| entry.id.as_str()),
            Some("org.gnome.Nautilus.desktop")
        );
    }

    #[test]
    fn handles_wm_class_turkish_case_flatpak_and_unknown_ids() {
        let index = AppIndex::new([
            entry("firefox-esr.desktop", Some("firefox")),
            entry("org.example.Işık.desktop", None),
            entry("org.flatpak.Editor.desktop", None),
        ]);
        assert_eq!(
            index.resolve("FIREFOX").map(|entry| entry.id.as_str()),
            Some("firefox-esr.desktop")
        );
        assert_eq!(
            index.resolve("ISIK").map(|entry| entry.id.as_str()),
            Some("org.example.Işık.desktop")
        );
        assert_eq!(
            index
                .resolve("org.flatpak.Editor")
                .map(|entry| entry.id.as_str()),
            Some("org.flatpak.Editor.desktop")
        );
        assert!(index.resolve("").is_none());
        assert!(index.resolve("missing").is_none());
    }

    #[test]
    fn preserves_first_xdg_override_and_no_display_metadata() {
        let mut hidden = entry("same.desktop", None);
        hidden.name = "User override".into();
        hidden.visible = false;
        let index = AppIndex::new([hidden, entry("same.desktop", None)]);
        assert_eq!(index.entries().len(), 1);
        assert_eq!(
            index
                .resolve("same")
                .map(|entry| (&*entry.name, entry.visible)),
            Some(("User override", false))
        );
    }

    #[test]
    #[allow(clippy::unwrap_used)]
    fn reads_real_desktop_fixture() {
        let app = gio::DesktopAppInfo::from_filename(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/tests/fixtures/org.example.Editor.desktop"
        ))
        .unwrap();
        let entry = DesktopEntry::from_app(&app).unwrap();
        assert_eq!(entry.startup_wm_class.as_deref(), Some("ExampleEditor"));
        assert_eq!(entry.icon.as_deref(), Some("org.example.Editor"));
        assert!(entry.keywords.iter().any(|word| word == "Text"));
    }
}
