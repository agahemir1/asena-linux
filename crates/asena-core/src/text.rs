use unicode_normalization::{char::is_combining_mark, UnicodeNormalization};

/// Accent-insensitive search key, not a locale-aware display sorting key.
pub fn fold(text: &str) -> String {
    text.chars()
        .map(|c| match c {
            'İ' | 'I' | 'ı' => 'i',
            other => other,
        })
        .flat_map(char::to_lowercase)
        .nfd()
        .filter(|c| !is_combining_mark(*c))
        .collect()
}

#[cfg(test)]
mod tests {
    use super::fold;

    #[test]
    fn turkish_letters_and_cases() {
        for (input, expected) in [
            ("İSTANBUL", "istanbul"),
            ("Işık", "isik"),
            ("ÇAĞRI", "cagri"),
            ("İIıi", "iiii"),
            ("ÇçĞğÖöŞşÜü", "ccggoossuu"),
        ] {
            assert_eq!(fold(input), expected);
        }
        assert_eq!(fold("Ayarlar"), fold("AYARLAR"));
    }

    #[test]
    fn canonical_equivalence_and_accents() {
        assert_eq!(fold("I\u{307}S\u{327}IK"), "isik");
        assert_eq!(fold("École Ångström café"), "ecole angstrom cafe");
        assert_eq!(fold("ş"), fold("s\u{327}"));
    }

    #[test]
    fn preserves_spacing_symbols_and_other_scripts() {
        assert_eq!(fold(""), "");
        assert_eq!(fold("  A\tB\n42 + 🐺 中文"), "  a\tb\n42 + 🐺 中文");
        assert_eq!(fold("Москва"), "москва");
        for input in ["İstanbul", "Çağrı", "cafe\u{301}", "中文"] {
            assert_eq!(fold(&fold(input)), fold(input));
        }
    }
}
