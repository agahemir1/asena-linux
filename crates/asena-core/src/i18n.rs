use std::path::PathBuf;

pub use gettextrs::{gettext, ngettext};
pub const DOMAIN: &str = "asena";

/// Call once, before GTK initialization or starting worker threads.
pub fn init() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    gettextrs::setlocale(gettextrs::LocaleCategory::LcAll, "");
    let locale_dir = std::env::var_os("ASENA_LOCALE_DIR")
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("/usr/share/locale"));
    gettextrs::bindtextdomain(DOMAIN, locale_dir)?;
    gettextrs::bind_textdomain_codeset(DOMAIN, "UTF-8")?;
    gettextrs::textdomain(DOMAIN)?;
    Ok(())
}

/// Environment for processes started in a chosen session language, for
/// example `tr_TR.UTF-8`. Empty when the value is not a locale name.
///
/// `LANGUAGE` selects message catalogs even when `LC_ALL` is set by the
/// session; `LC_ALL` is only replaced when the caller already had one.
pub fn language_environment(language: &str, has_lc_all: bool) -> Vec<(&'static str, String)> {
    let valid = !language.is_empty()
        && language.len() <= 32
        && language
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'_' | b'.' | b'-' | b'@'));
    if !valid {
        return Vec::new();
    }
    let base = language.split(['.', '@']).next().unwrap_or(language);
    let short = base.split('_').next().unwrap_or(base);
    let fallback = if base == short {
        short.to_owned()
    } else {
        format!("{base}:{short}")
    };
    let mut environment = vec![
        ("LANG", language.to_owned()),
        ("LANGUAGE", fallback),
        ("LC_MESSAGES", language.to_owned()),
    ];
    if has_lc_all {
        environment.push(("LC_ALL", language.to_owned()));
    }
    environment
}

#[cfg(test)]
mod tests {
    use super::language_environment;

    #[test]
    fn language_environment_covers_catalog_lookup() {
        assert_eq!(
            language_environment("tr_TR.UTF-8", false),
            vec![
                ("LANG", "tr_TR.UTF-8".to_owned()),
                ("LANGUAGE", "tr_TR:tr".to_owned()),
                ("LC_MESSAGES", "tr_TR.UTF-8".to_owned()),
            ]
        );
        assert_eq!(language_environment("en_US.UTF-8", true).len(), 4);
        assert!(language_environment("", false).is_empty());
        assert!(language_environment("tr\nPATH=/x", false).is_empty());
    }
}
