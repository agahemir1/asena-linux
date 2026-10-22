//! Start menu structure shared by the compact, classic and grid layouts.
//!
//! Sorting for display uses the locale aware collation supplied by the caller;
//! this module only groups already sorted entries and maps categories.
use crate::{apps::DesktopEntry, i18n::gettext};

/// Menu layouts stored in `org.asenalinux.shell menu-layout`.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Layout {
    #[default]
    Compact,
    Classic,
    Grid,
}

impl Layout {
    pub fn from_key(key: &str) -> Self {
        match key {
            "classic" => Self::Classic,
            "grid" => Self::Grid,
            _ => Self::Compact,
        }
    }
}

/// Application groups of the classic menu, in display order.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Category {
    Internet,
    Office,
    Graphics,
    Multimedia,
    Development,
    Education,
    Games,
    Utilities,
    System,
    Other,
}

impl Category {
    pub const ALL: [Category; 10] = [
        Category::Internet,
        Category::Office,
        Category::Graphics,
        Category::Multimedia,
        Category::Development,
        Category::Education,
        Category::Games,
        Category::Utilities,
        Category::System,
        Category::Other,
    ];

    pub fn name(self) -> String {
        match self {
            Self::Internet => gettext("Internet"),
            Self::Office => gettext("Office"),
            Self::Graphics => gettext("Graphics"),
            Self::Multimedia => gettext("Sound and video"),
            Self::Development => gettext("Development"),
            Self::Education => gettext("Education"),
            Self::Games => gettext("Games"),
            Self::Utilities => gettext("Accessories"),
            Self::System => gettext("System"),
            Self::Other => gettext("Other"),
        }
    }

    pub fn icon(self) -> &'static str {
        match self {
            Self::Internet => "applications-internet-symbolic",
            Self::Office => "applications-office-symbolic",
            Self::Graphics => "applications-graphics-symbolic",
            Self::Multimedia => "applications-multimedia-symbolic",
            Self::Development => "applications-development-symbolic",
            Self::Education => "applications-science-symbolic",
            Self::Games => "applications-games-symbolic",
            Self::Utilities => "applications-utilities-symbolic",
            Self::System => "applications-system-symbolic",
            Self::Other => "applications-other-symbolic",
        }
    }
}

/// Maps desktop file categories (freedesktop main categories first) to a group.
pub fn category(categories: &[String]) -> Category {
    let has = |name: &str| categories.iter().any(|item| item == name);
    if has("Settings") || has("System") || has("Monitor") || has("PackageManager") {
        Category::System
    } else if has("Network") || has("WebBrowser") || has("Email") || has("Chat") {
        Category::Internet
    } else if has("Office") || has("WordProcessor") || has("Spreadsheet") || has("Calendar") {
        Category::Office
    } else if has("Graphics") || has("Photography") || has("Viewer") {
        Category::Graphics
    } else if has("AudioVideo") || has("Audio") || has("Video") || has("Player") {
        Category::Multimedia
    } else if has("Development") || has("IDE") {
        Category::Development
    } else if has("Education") || has("Science") {
        Category::Education
    } else if has("Game") {
        Category::Games
    } else if has("Utility") || has("Accessories") || has("TextEditor") || has("FileTools") {
        Category::Utilities
    } else {
        Category::Other
    }
}

/// Section letter for the alphabetical list. Turkish locales keep dotted and
/// dotless I apart; digits and symbols share the `#` section.
pub fn index_letter(name: &str, turkish: bool) -> String {
    let Some(first) = name.trim_start().chars().next() else {
        return "#".into();
    };
    if !first.is_alphabetic() {
        return "#".into();
    }
    match (turkish, first) {
        (true, 'i') => "İ".into(),
        (true, 'ı') => "I".into(),
        _ => first.to_uppercase().collect(),
    }
}

/// Section letters offered by the letter picker, in alphabet order.
pub fn alphabet(turkish: bool) -> Vec<String> {
    let letters = if turkish {
        "A B C Ç D E F G Ğ H I İ J K L M N O Ö P R S Ş T U Ü V Y Z"
    } else {
        "A B C D E F G H I J K L M N O P Q R S T U V W X Y Z"
    };
    letters
        .split(' ')
        .map(ToOwned::to_owned)
        .chain(std::iter::once("#".to_owned()))
        .collect()
}

/// True when user interface messages are Turkish.
pub fn turkish_locale() -> bool {
    ["LANGUAGE", "LC_ALL", "LC_MESSAGES", "LANG"]
        .iter()
        .filter_map(|name| std::env::var(name).ok())
        .find(|value| !value.is_empty())
        .is_some_and(|value| value.starts_with("tr"))
}

/// Groups entries that are already in display order into letter sections.
pub fn sections(entries: &[DesktopEntry], turkish: bool) -> Vec<(String, Vec<&DesktopEntry>)> {
    let mut result: Vec<(String, Vec<&DesktopEntry>)> = Vec::new();
    for entry in entries {
        let letter = index_letter(&entry.name, turkish);
        match result.last_mut() {
            Some((current, items)) if *current == letter => items.push(entry),
            _ => result.push((letter, vec![entry])),
        }
    }
    result
}

/// Orders pinned desktop IDs as stored and drops IDs that are not installed.
pub fn pinned<'a>(entries: &'a [DesktopEntry], pins: &[String]) -> Vec<&'a DesktopEntry> {
    pins.iter()
        .filter_map(|pin| {
            entries
                .iter()
                .find(|entry| &entry.id == pin && entry.visible)
        })
        .collect()
}

/// Adds or removes a desktop ID, keeping the order of the remaining pins.
pub fn toggle_pin(pins: &[String], id: &str) -> Vec<String> {
    if pins.iter().any(|pin| pin == id) {
        pins.iter().filter(|pin| *pin != id).cloned().collect()
    } else {
        let mut next = pins.to_vec();
        next.push(id.to_owned());
        next
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    fn entry(id: &str, name: &str, categories: &[&str]) -> DesktopEntry {
        DesktopEntry {
            id: id.into(),
            name: name.into(),
            icon: None,
            filename: PathBuf::from(id),
            startup_wm_class: None,
            generic_name: None,
            keywords: vec![],
            categories: categories.iter().map(|item| (*item).to_owned()).collect(),
            visible: true,
            commandline: String::new(),
        }
    }

    #[test]
    fn layouts_fall_back_to_compact() {
        assert_eq!(Layout::from_key("classic"), Layout::Classic);
        assert_eq!(Layout::from_key("grid"), Layout::Grid);
        assert_eq!(Layout::from_key("unknown"), Layout::Compact);
    }

    #[test]
    fn categories_prefer_system_and_specific_groups() {
        let list = |items: &[&str]| {
            items
                .iter()
                .map(|item| (*item).to_owned())
                .collect::<Vec<_>>()
        };
        assert_eq!(
            category(&list(&["Network", "WebBrowser"])),
            Category::Internet
        );
        assert_eq!(
            category(&list(&["Settings", "DesktopSettings"])),
            Category::System
        );
        assert_eq!(
            category(&list(&["GTK", "Utility", "TextEditor"])),
            Category::Utilities
        );
        assert_eq!(
            category(&list(&["AudioVideo", "Player"])),
            Category::Multimedia
        );
        assert_eq!(category(&list(&["Graphics", "Viewer"])), Category::Graphics);
        assert_eq!(category(&[]), Category::Other);
    }

    #[test]
    fn turkish_letters_keep_dotted_and_dotless_i() {
        assert_eq!(index_letter("ışık", true), "I");
        assert_eq!(index_letter("izmir", true), "İ");
        assert_eq!(index_letter("izmir", false), "I");
        assert_eq!(index_letter("Çizim", true), "Ç");
        assert_eq!(index_letter("7-Zip", true), "#");
        assert_eq!(index_letter("", true), "#");
    }

    #[test]
    fn alphabets_cover_section_letters() {
        let turkish = alphabet(true);
        assert_eq!(turkish.len(), 30);
        assert!(turkish.contains(&"İ".to_owned()) && turkish.contains(&"I".to_owned()));
        assert!(!turkish.contains(&"Q".to_owned()));
        assert_eq!(alphabet(false).len(), 27);
        assert_eq!(alphabet(false).last().map(String::as_str), Some("#"));
    }

    #[test]
    fn sections_follow_existing_order() {
        let entries = vec![
            entry("a", "Arşiv", &[]),
            entry("b", "Ayarlar", &[]),
            entry("c", "Çizim", &[]),
            entry("d", "İnternet", &[]),
        ];
        let groups = sections(&entries, true);
        let letters: Vec<_> = groups
            .iter()
            .map(|(letter, items)| (letter.as_str(), items.len()))
            .collect();
        assert_eq!(letters, vec![("A", 2), ("Ç", 1), ("İ", 1)]);
    }

    #[test]
    fn pins_keep_order_and_skip_missing() {
        let entries = vec![entry("a.desktop", "A", &[]), entry("b.desktop", "B", &[])];
        let pins = vec![
            "b.desktop".to_owned(),
            "missing.desktop".to_owned(),
            "a.desktop".to_owned(),
        ];
        let ids: Vec<_> = pinned(&entries, &pins)
            .iter()
            .map(|entry| entry.id.as_str())
            .collect();
        assert_eq!(ids, vec!["b.desktop", "a.desktop"]);
        assert_eq!(
            toggle_pin(&pins, "missing.desktop"),
            vec!["b.desktop", "a.desktop"]
        );
        assert_eq!(toggle_pin(&["a".into()], "b"), vec!["a", "b"]);
    }
}
