//! All compositor-specific configuration stays inside this module.
//!
//! The daemon renders the Wayfire configuration from the Asena template and
//! the input, transparency and animation preferences. Wayfire watches the file
//! and applies a replaced copy by itself, so no reload request is needed.
use gio::prelude::*;
use std::{
    cell::RefCell,
    io::Write,
    path::{Path, PathBuf},
    rc::Rc,
    time::Duration,
};
const TEMPLATE: &str = include_str!("../../../data/wayfire/wayfire.ini");
const FILE_NAME: &str = "wayfire.ini";

/// Input preferences that Wayfire applies.
#[derive(Clone, Debug, PartialEq)]
pub struct Input {
    pub repeat_rate: i32,
    pub repeat_delay: i32,
    pub pointer_speed: f64,
    pub natural_scroll: bool,
    pub tap_to_click: bool,
    pub layouts: Vec<String>,
    pub toggle: String,
}

impl Default for Input {
    fn default() -> Self {
        Self {
            repeat_rate: 25,
            repeat_delay: 600,
            pointer_speed: 0.0,
            natural_scroll: false,
            tap_to_click: true,
            layouts: vec!["tr".into(), "us".into()],
            toggle: "grp:alt_shift_toggle".into(),
        }
    }
}

impl Input {
    pub fn from_settings(settings: &gio::Settings) -> Self {
        Self {
            repeat_rate: settings.int("repeat-rate"),
            repeat_delay: settings.int("repeat-delay"),
            pointer_speed: settings.double("pointer-speed"),
            natural_scroll: settings.boolean("natural-scroll"),
            tap_to_click: settings.boolean("tap-to-click"),
            layouts: settings
                .strv("keyboard-layouts")
                .iter()
                .map(ToString::to_string)
                .collect(),
            toggle: settings.string("keyboard-toggle").to_string(),
        }
    }
}

/// Desktop preferences that decide which Wayfire effect plugins run.
#[derive(Clone, Debug, PartialEq)]
pub struct Effects {
    pub transparency: bool,
    pub animations: bool,
}

impl Default for Effects {
    fn default() -> Self {
        Self {
            transparency: true,
            animations: true,
        }
    }
}

impl Effects {
    pub fn from_settings(settings: &gio::Settings) -> Self {
        Self {
            transparency: settings.boolean("transparency"),
            animations: settings.boolean("enable-animations"),
        }
    }
}

fn boolean(value: bool) -> &'static str {
    if value {
        "true"
    } else {
        "false"
    }
}

/// XKB layout names are short ASCII identifiers; anything else is dropped.
fn safe_token(value: &str) -> bool {
    !value.is_empty()
        && value.len() <= 32
        && value.bytes().all(|byte| {
            byte.is_ascii_alphanumeric() || matches!(byte, b'_' | b'-' | b':' | b'(' | b')')
        })
}

fn section_name(line: &str) -> Option<&str> {
    line.trim()
        .strip_prefix('[')
        .and_then(|rest| rest.strip_suffix(']'))
}

fn option_name(line: &str) -> Option<&str> {
    let line = line.trim_start();
    if line.starts_with('#') {
        return None;
    }
    line.split_once('=').map(|(name, _)| name.trim())
}

/// Replaces `key = value` in `[section]`, or adds it at the end of the
/// section. Values with line breaks are refused so a value can never start a
/// new option or section.
pub fn set_option(text: &str, section: &str, key: &str, value: &str) -> String {
    if value.contains(['\n', '\r']) {
        return text.to_owned();
    }
    let assignment = format!("{key} = {value}");
    let mut lines: Vec<String> = Vec::new();
    let mut current: Option<String> = None;
    let mut done = false;
    for line in text.lines() {
        if let Some(name) = section_name(line) {
            if current.as_deref() == Some(section) && !done {
                // Keep the blank separator lines after the new option.
                let at = lines
                    .iter()
                    .rposition(|line| !line.trim().is_empty())
                    .map_or(lines.len(), |index| index + 1);
                lines.insert(at, assignment.clone());
                done = true;
            }
            current = Some(name.to_owned());
        } else if current.as_deref() == Some(section) && option_name(line) == Some(key) {
            if done {
                // A duplicate would override the replaced value.
                continue;
            }
            lines.push(assignment.clone());
            done = true;
            continue;
        }
        lines.push(line.to_owned());
    }
    if !done {
        if current.as_deref() != Some(section) {
            lines.push(String::new());
            lines.push(format!("[{section}]"));
        }
        lines.push(assignment);
    }
    let mut result = lines.join("\n");
    result.push('\n');
    result
}

/// Keys that type a to z on the Turkish F layout (XKB `tr(f)`), as evdev
/// names, indexed by letter.
const TURKISH_F: [&str; 26] = [
    "F",
    "COMMA",
    "V",
    "Y",
    "D",
    "Q",
    "W",
    "O",
    "S",
    "Z",
    "J",
    "L",
    "K",
    "I",
    "T",
    "P",
    "LEFTBRACE",
    "U",
    "M",
    "H",
    "A",
    "C",
    "RIGHTBRACE",
    "BACKSLASH",
    "SEMICOLON",
    "N",
];

/// The key that types `letter` (a to z) on `layout`.
fn letter_key(layout: &str, letter: u8) -> Option<String> {
    let index = usize::from(letter.checked_sub(b'a').filter(|index| *index < 26)?);
    let name = match layout {
        "tr(f)" => TURKISH_F[index].to_owned(),
        // Turkish Q types ı on the I key and i on the apostrophe key.
        "tr" if letter == b'i' => "APOSTROPHE".to_owned(),
        _ => char::from(letter.to_ascii_uppercase()).to_string(),
    };
    Some(format!("KEY_{name}"))
}

/// Wayfire matches physical keys. The template writes letter bindings at US
/// positions; move them to the keys that type those letters on the first
/// layout, so Super+A means the key labelled A.
fn letter_bindings(text: &str, layout: &str) -> String {
    let mut result = String::with_capacity(text.len());
    for line in text.split_inclusive('\n') {
        let binding = option_name(line).is_some_and(|name| name.contains("binding"));
        let Some((head, value)) = line.split_once('=').filter(|_| binding) else {
            result.push_str(line);
            continue;
        };
        let tokens: Vec<String> = value
            .split(' ')
            .map(|token| {
                let word = token.trim_end();
                match word.strip_prefix("KEY_").map(str::as_bytes) {
                    Some([letter]) if letter.is_ascii_uppercase() => {
                        letter_key(layout, letter.to_ascii_lowercase())
                            .map(|key| token.replacen(word, &key, 1))
                            .unwrap_or_else(|| token.to_owned())
                    }
                    _ => token.to_owned(),
                }
            })
            .collect();
        result.push_str(head);
        result.push('=');
        result.push_str(&tokens.join(" "));
    }
    result
}

fn plugins(text: &str, effects: &Effects) -> String {
    let current = text
        .lines()
        .skip_while(|line| section_name(line) != Some("core"))
        .find(|line| option_name(line) == Some("plugins"))
        .and_then(|line| line.split_once('='))
        .map_or("", |(_, value)| value.trim());
    current
        .split_whitespace()
        .filter(|plugin| effects.transparency || *plugin != "blur")
        .filter(|plugin| effects.animations || *plugin != "animate")
        .collect::<Vec<_>>()
        .join(" ")
}

pub fn render(input: &Input, effects: &Effects) -> String {
    let layouts: Vec<&str> = input
        .layouts
        .iter()
        .map(String::as_str)
        .filter(|layout| safe_token(layout))
        .take(4)
        .collect();
    let options = if layouts.len() > 1 && safe_token(&input.toggle) {
        input.toggle.as_str()
    } else {
        ""
    };
    let layout = if layouts.is_empty() {
        "us".to_owned()
    } else {
        layouts.join(",")
    };
    let speed = format!("{:.2}", input.pointer_speed.clamp(-1.0, 1.0));
    let rate = input.repeat_rate.clamp(1, 100).to_string();
    let delay = input.repeat_delay.clamp(100, 2000).to_string();
    let mut text = letter_bindings(
        &format!(
            "# Generated by asena-daemon from Asena settings. Changes here are replaced.\n{TEMPLATE}"
        ),
        layouts.first().copied().unwrap_or("us"),
    );
    for (key, value) in [
        ("xkb_layout", layout.as_str()),
        ("xkb_options", options),
        ("kb_repeat_rate", rate.as_str()),
        ("kb_repeat_delay", delay.as_str()),
        ("mouse_cursor_speed", speed.as_str()),
        ("touchpad_cursor_speed", speed.as_str()),
        ("mouse_natural_scroll", boolean(input.natural_scroll)),
        ("natural_scroll", boolean(input.natural_scroll)),
        ("tap_to_click", boolean(input.tap_to_click)),
    ] {
        text = set_option(&text, "input", key, value);
    }
    let enabled = plugins(&text, effects);
    set_option(&text, "core", "plugins", &enabled)
}

fn write_if_changed(root: &Path, name: &str, contents: &str) -> std::io::Result<bool> {
    let target = root.join(name);
    if std::fs::read_to_string(&target).ok().as_deref() == Some(contents) {
        return Ok(false);
    }
    let stamp = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_err(std::io::Error::other)?
        .as_nanos();
    let temporary = root.join(format!(".{name}.{stamp}.tmp"));
    let mut file = std::fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(&temporary)?;
    file.write_all(contents.as_bytes())?;
    file.sync_all()?;
    // Wayfire reloads when a file with its configuration name is moved in.
    std::fs::rename(temporary, target)?;
    Ok(true)
}

fn checked_root(root: &Path) -> std::io::Result<()> {
    std::fs::create_dir_all(root)?;
    if std::fs::symlink_metadata(root)?.file_type().is_symlink() {
        return Err(std::io::Error::other(
            "The Asena configuration directory must not be a symlink",
        ));
    }
    Ok(())
}

/// Writes the configuration file; returns whether it changed.
pub fn generate(path: &Path, input: &Input, effects: &Effects) -> std::io::Result<bool> {
    let root = path
        .parent()
        .ok_or_else(|| std::io::Error::other("The configuration path has no directory"))?;
    let name = path
        .file_name()
        .and_then(|name| name.to_str())
        .ok_or_else(|| std::io::Error::other("The configuration path has no file name"))?;
    checked_root(root)?;
    write_if_changed(root, name, &render(input, effects))
}

/// The file Wayfire reads in this session: the nested development copy, or
/// the per-user file that asena-session creates before Wayfire starts.
pub fn path() -> PathBuf {
    match std::env::var_os("ASENA_WAYFIRE_CONFIG") {
        Some(path) if std::env::var_os("ASENA_DEV").is_some() => PathBuf::from(path),
        _ => glib::user_config_dir().join("asena").join(FILE_NAME),
    }
}

async fn apply(path: PathBuf, input: Input, effects: Effects) {
    match gio::spawn_blocking(move || generate(&path, &input, &effects)).await {
        Ok(Ok(_)) => {}
        error => tracing::warn!(?error, "Could not write the compositor configuration"),
    }
}

fn current(input: Option<&gio::Settings>, desktop: &gio::Settings) -> (Input, Effects) {
    (
        input.map(Input::from_settings).unwrap_or_default(),
        Effects::from_settings(desktop),
    )
}

/// Generates the configuration now and again whenever the preferences change.
pub fn watch(input: Option<&gio::Settings>, desktop: &gio::Settings) {
    let (first_input, first_effects) = current(input, desktop);
    glib::spawn_future_local(apply(path(), first_input, first_effects));
    let pending: Rc<RefCell<Option<glib::SourceId>>> = Rc::default();
    let schedule = {
        let input = input.cloned();
        let desktop = desktop.clone();
        Rc::new(move || {
            if let Some(source) = pending.borrow_mut().take() {
                source.remove();
            }
            let input = input.clone();
            let desktop = desktop.clone();
            let slot = pending.clone();
            // Sliders change values quickly; write once they settle.
            let source = glib::timeout_add_local_once(Duration::from_millis(400), move || {
                slot.borrow_mut().take();
                let (input, effects) = current(input.as_ref(), &desktop);
                glib::spawn_future_local(apply(path(), input, effects));
            });
            pending.replace(Some(source));
        })
    };
    if let Some(input) = input {
        let schedule = schedule.clone();
        input.connect_changed(None, move |_, _| schedule());
    }
    for key in ["transparency", "enable-animations"] {
        let schedule = schedule.clone();
        desktop.connect_changed(Some(key), move |_, _| schedule());
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn option<'a>(text: &'a str, section: &str, key: &str) -> Option<&'a str> {
        let mut current = None;
        for line in text.lines() {
            if let Some(name) = section_name(line) {
                current = Some(name.to_owned());
            } else if current.as_deref() == Some(section) && option_name(line) == Some(key) {
                return line.split_once('=').map(|(_, value)| value.trim());
            }
        }
        None
    }

    #[test]
    fn generation_is_idempotent_and_keeps_other_config() -> std::io::Result<()> {
        let root = std::env::temp_dir().join(format!("asena-config-test-{}", std::process::id()));
        std::fs::create_dir_all(root.join("wayfire"))?;
        std::fs::write(root.join("wayfire.ini"), "user")?;
        let path = root.join("asena").join(FILE_NAME);
        assert!(generate(&path, &Input::default(), &Effects::default())?);
        assert!(!generate(&path, &Input::default(), &Effects::default())?);
        assert_eq!(std::fs::read_to_string(root.join("wayfire.ini"))?, "user");
        assert!(path.is_file());
        std::fs::remove_dir_all(root)
    }

    #[test]
    fn letter_keys_follow_the_layout() {
        assert_eq!(letter_key("us", b'a').as_deref(), Some("KEY_A"));
        assert_eq!(letter_key("tr", b'a').as_deref(), Some("KEY_A"));
        assert_eq!(letter_key("tr", b'i').as_deref(), Some("KEY_APOSTROPHE"));
        assert_eq!(letter_key("tr(f)", b'a').as_deref(), Some("KEY_F"));
        assert_eq!(letter_key("tr(f)", b'n').as_deref(), Some("KEY_I"));
        assert_eq!(letter_key("tr(f)", b'x').as_deref(), Some("KEY_BACKSLASH"));
        assert_eq!(letter_key("us", b'1'), None);
        // Every letter has its own key.
        let mut keys = TURKISH_F.to_vec();
        keys.sort_unstable();
        keys.dedup();
        assert_eq!(keys.len(), 26);
    }

    #[test]
    fn letter_bindings_move_to_the_first_layout() {
        let render_with = |layouts: &[&str]| {
            render(
                &Input {
                    layouts: layouts.iter().map(|layout| layout.to_string()).collect(),
                    ..Input::default()
                },
                &Effects::default(),
            )
        };
        let turkish_f = render_with(&["tr(f)", "us"]);
        assert_eq!(
            option(&turkish_f, "command", "binding_quick_settings"),
            Some("<super> KEY_F")
        );
        assert_eq!(
            option(&turkish_f, "command", "binding_notifications"),
            Some("<super> KEY_I")
        );
        // Non-letter keys stay where they are.
        assert_eq!(
            option(&turkish_f, "command", "binding_terminal"),
            Some("<super> KEY_ENTER")
        );
        let turkish_q = render_with(&["tr", "us"]);
        assert_eq!(
            option(&turkish_q, "command", "binding_quick_settings"),
            Some("<super> KEY_A")
        );
        // The second layout does not move bindings.
        let english = render_with(&["us", "tr(f)"]);
        assert_eq!(
            option(&english, "command", "binding_notifications"),
            Some("<super> KEY_N")
        );
    }

    #[test]
    fn renders_input_preferences() {
        let input = Input {
            repeat_rate: 400,
            repeat_delay: 300,
            pointer_speed: -0.5,
            natural_scroll: true,
            tap_to_click: false,
            layouts: vec!["tr(f)".into(), "us".into(), "bad layout;rm".into()],
            toggle: "grp:win_space_toggle".into(),
        };
        let text = render(&input, &Effects::default());
        assert_eq!(option(&text, "input", "xkb_layout"), Some("tr(f),us"));
        assert_eq!(
            option(&text, "input", "xkb_options"),
            Some("grp:win_space_toggle")
        );
        assert_eq!(option(&text, "input", "kb_repeat_rate"), Some("100"));
        assert_eq!(option(&text, "input", "kb_repeat_delay"), Some("300"));
        assert_eq!(option(&text, "input", "mouse_cursor_speed"), Some("-0.50"));
        assert_eq!(option(&text, "input", "natural_scroll"), Some("true"));
        assert_eq!(option(&text, "input", "tap_to_click"), Some("false"));
        assert!(!text.contains("bad layout"));
        assert!(text.starts_with("# Generated by asena-daemon"));
        assert_eq!(text.matches("xkb_layout").count(), 1);
    }

    #[test]
    fn a_single_layout_needs_no_switch_option() {
        let input = Input {
            layouts: vec!["tr".into()],
            ..Input::default()
        };
        let text = render(&input, &Effects::default());
        assert_eq!(option(&text, "input", "xkb_layout"), Some("tr"));
        assert_eq!(option(&text, "input", "xkb_options"), Some(""));
    }

    #[test]
    fn effects_choose_plugins() {
        let all = render(&Input::default(), &Effects::default());
        let enabled = option(&all, "core", "plugins").unwrap_or_default();
        assert!(enabled.split_whitespace().any(|plugin| plugin == "blur"));
        assert!(enabled.split_whitespace().any(|plugin| plugin == "animate"));
        let plain = render(
            &Input::default(),
            &Effects {
                transparency: false,
                animations: false,
            },
        );
        let enabled = option(&plain, "core", "plugins").unwrap_or_default();
        assert!(!enabled.split_whitespace().any(|plugin| plugin == "blur"));
        assert!(!enabled.split_whitespace().any(|plugin| plugin == "animate"));
        assert!(enabled
            .split_whitespace()
            .any(|plugin| plugin == "session-lock"));
        // The blur rule stays; without the plugin it has no effect.
        assert!(plain.contains("[blur]"));
    }

    #[test]
    fn options_are_replaced_added_and_never_split_lines() {
        let text = "[a]\nkey = 1\n\n[b]\nother = 2\n";
        assert_eq!(
            set_option(text, "a", "key", "3"),
            "[a]\nkey = 3\n\n[b]\nother = 2\n"
        );
        assert_eq!(
            set_option(text, "a", "new", "4"),
            "[a]\nkey = 1\nnew = 4\n\n[b]\nother = 2\n"
        );
        assert_eq!(
            set_option(text, "c", "key", "5"),
            "[a]\nkey = 1\n\n[b]\nother = 2\n\n[c]\nkey = 5\n"
        );
        assert_eq!(set_option(text, "a", "key", "1\n[core]"), text);
        assert_eq!(
            set_option("[a]\nkey = 1\nkey = 2\n", "a", "key", "3"),
            "[a]\nkey = 3\n"
        );
        assert_eq!(
            set_option("[a]\n# key = 1\n", "a", "key", "3"),
            "[a]\n# key = 1\nkey = 3\n"
        );
    }

    #[test]
    fn template_parses_as_sections() {
        for section in [
            "core",
            "autostart",
            "command",
            "input",
            "blur",
            "decoration",
        ] {
            assert!(
                TEMPLATE
                    .lines()
                    .any(|line| section_name(line) == Some(section)),
                "{section}"
            );
        }
        assert_eq!(option(TEMPLATE, "blur", "method"), Some("kawase"));
        // A hash mark starts a Wayfire comment, so no value may begin with one.
        for line in TEMPLATE.lines().filter(|line| option_name(line).is_some()) {
            let value = line.split_once('=').map_or("", |(_, value)| value.trim());
            assert!(!value.starts_with('#'), "{line}");
        }
        assert_eq!(option(TEMPLATE, "core", "xwayland"), Some("true"));
    }
}
