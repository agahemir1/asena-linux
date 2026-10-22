//! Accent palette and the icon theme drawn for each accent.

/// libadwaita 1.7 accent colors by `org.asenalinux.desktop accent-color` name.
/// libadwaita maps a portal color to the nearest named accent, so these values
/// round-trip exactly.
pub const ACCENTS: [(&str, u32); 9] = [
    ("blue", 0x3584e4),
    ("teal", 0x2190a4),
    ("green", 0x3a944a),
    ("yellow", 0xc88800),
    ("orange", 0xed5b00),
    ("red", 0xe62d42),
    ("pink", 0xd56199),
    ("purple", 0x9141ac),
    ("slate", 0x6f8396),
];

/// The base icon theme; its folders and accent-colored app icons are teal.
pub const ICON_THEME: &str = "Asena";
const BASE_ACCENT: &str = "teal";

pub fn accent_rgb(name: &str) -> Option<u32> {
    ACCENTS
        .iter()
        .find(|(known, _)| *known == name)
        .map(|(_, rgb)| *rgb)
}

/// Folders and accent-colored app icons come from `Asena-<accent>`, which
/// inherits the base theme. Teal and unknown accents use the base theme.
pub fn icon_theme(accent: &str) -> String {
    if accent != BASE_ACCENT && accent_rgb(accent).is_some() {
        format!("{ICON_THEME}-{accent}")
    } else {
        ICON_THEME.to_owned()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_accent_but_the_base_has_its_own_icon_theme() {
        assert_eq!(icon_theme("teal"), "Asena");
        assert_eq!(icon_theme("purple"), "Asena-purple");
        assert_eq!(icon_theme("slate"), "Asena-slate");
        assert_eq!(icon_theme("magenta"), "Asena");
        assert_eq!(icon_theme("../x"), "Asena");
        let themes: std::collections::BTreeSet<String> =
            ACCENTS.iter().map(|(name, _)| icon_theme(name)).collect();
        assert_eq!(themes.len(), ACCENTS.len());
    }

    #[test]
    fn accent_colors_are_known_by_name() {
        assert_eq!(accent_rgb("purple"), Some(0x9141ac));
        assert_eq!(accent_rgb("Purple"), None);
    }
}
