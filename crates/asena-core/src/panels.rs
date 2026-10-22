//! Stable settings routes shared by the application and shell search.
use crate::{i18n::gettext, text::fold};

/// Sidebar groups, in display order.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum Group {
    Personal,
    Connections,
    Devices,
    System,
}

impl Group {
    pub fn name(self) -> String {
        match self {
            Self::Personal => gettext("Personalization"),
            Self::Connections => gettext("Connections"),
            Self::Devices => gettext("Devices"),
            Self::System => gettext("System"),
        }
    }
}

#[derive(Clone)]
pub struct Panel {
    pub id: &'static str,
    pub name: String,
    pub icon: &'static str,
    pub keywords: String,
    pub group: Group,
}

impl Panel {
    pub fn matches(&self, query: &str) -> bool {
        let query = fold(query);
        query.split_whitespace().all(|part| {
            fold(&format!("{} {} {}", self.id, self.name, self.keywords)).contains(part)
        })
    }
}

fn panel(
    id: &'static str,
    name: String,
    icon: &'static str,
    keywords: String,
    group: Group,
) -> Panel {
    Panel {
        id,
        name,
        icon,
        keywords,
        group,
    }
}

/// All panels, grouped and in sidebar order.
pub fn all() -> Vec<Panel> {
    use Group::*;
    vec![
        panel(
            "appearance",
            gettext("Appearance"),
            "preferences-desktop-theme-symbolic",
            gettext("theme color wallpaper text transparency blur"),
            Personal,
        ),
        panel(
            "desktop",
            gettext("Desktop and panel"),
            "preferences-desktop-display-symbolic",
            gettext("menu position icons clock monitors"),
            Personal,
        ),
        panel(
            "notifications",
            gettext("Notifications"),
            "preferences-system-notifications-symbolic",
            gettext("do not disturb applications"),
            Personal,
        ),
        panel(
            "network",
            gettext("Network"),
            "network-wireless-symbolic",
            gettext("wireless Wi-Fi Ethernet"),
            Connections,
        ),
        panel(
            "bluetooth",
            gettext("Bluetooth"),
            "bluetooth-symbolic",
            gettext("devices pairing"),
            Connections,
        ),
        panel(
            "displays",
            gettext("Displays"),
            "video-display-symbolic",
            gettext("resolution scale rotation arrangement"),
            Devices,
        ),
        panel(
            "sound",
            gettext("Sound"),
            "audio-volume-high-symbolic",
            gettext("speaker microphone volume"),
            Devices,
        ),
        panel(
            "keyboard",
            gettext("Keyboard"),
            "input-keyboard-symbolic",
            gettext("layout language repeat"),
            Devices,
        ),
        panel(
            "mouse",
            gettext("Mouse and touchpad"),
            "input-mouse-symbolic",
            gettext("pointer speed scrolling tapping"),
            Devices,
        ),
        panel(
            "power",
            gettext("Power"),
            "battery-symbolic",
            gettext("battery idle sleep brightness"),
            Devices,
        ),
        panel(
            "users",
            gettext("Users"),
            "system-users-symbolic",
            gettext("name photo password"),
            System,
        ),
        panel(
            "region",
            gettext("Region and language"),
            "preferences-desktop-locale-symbolic",
            gettext("locale formats system language"),
            System,
        ),
        panel(
            "datetime",
            gettext("Date and time"),
            "preferences-system-time-symbolic",
            gettext("timezone automatic time format"),
            System,
        ),
        panel(
            "defaults",
            gettext("Default applications"),
            "preferences-desktop-default-applications-symbolic",
            gettext("browser mail music video photos files terminal"),
            System,
        ),
        panel(
            "privacy",
            gettext("Privacy and security"),
            "preferences-system-privacy-symbolic",
            gettext("location camera microphone history lock permissions"),
            System,
        ),
        panel(
            "accessibility",
            gettext("Accessibility"),
            "preferences-desktop-accessibility-symbolic",
            gettext("contrast zoom reader text animation visual hearing"),
            System,
        ),
        panel(
            "about",
            gettext("About"),
            "help-about-symbolic",
            gettext("system computer name version"),
            System,
        ),
    ]
}

pub fn find(id: &str) -> Option<Panel> {
    all().into_iter().find(|panel| panel.id == id)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn routes_are_unique_and_unknown_ids_are_not_commands() {
        let panels = all();
        let ids: std::collections::BTreeSet<_> = panels.iter().map(|panel| panel.id).collect();
        assert_eq!(ids.len(), panels.len());
        assert!(find("appearance").is_some());
        assert!(find("appearance;shutdown").is_none());
    }

    #[test]
    fn panels_are_ordered_by_group() {
        let groups: Vec<Group> = all().iter().map(|panel| panel.group).collect();
        let mut sorted = groups.clone();
        sorted.sort();
        assert_eq!(groups, sorted);
    }

    #[test]
    fn localized_search_uses_turkish_folding() {
        let panel = panel(
            "appearance",
            "Görünüm".into(),
            "",
            "Işık renk".into(),
            Group::Personal,
        );
        assert!(panel.matches("GÖRÜNÜM"));
        assert!(panel.matches("ışık RENK"));
        assert!(!panel.matches("ağ"));
    }
}
