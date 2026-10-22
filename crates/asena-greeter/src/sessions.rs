//! Only root-owned system Wayland session records are eligible for login.
use std::{
    os::unix::fs::MetadataExt,
    path::{Path, PathBuf},
};
#[derive(Clone, Debug)]
pub struct Session {
    pub id: String,
    pub name: String,
    pub command: Vec<String>,
    pub environment: Vec<String>,
}
pub fn preferred(sessions: &[Session]) -> Option<usize> {
    sessions
        .iter()
        .position(|session| session.id == "asena")
        .or_else(|| (!sessions.is_empty()).then_some(0))
}
fn trusted(path: &Path) -> bool {
    std::fs::metadata(path).is_ok_and(|m| m.uid() == 0 && m.mode() & 0o022 == 0)
}
fn parse(path: &Path, key: &glib::KeyFile) -> anyhow::Result<Session> {
    let group = "Desktop Entry";
    anyhow::ensure!(
        key.string(group, "Type")?.as_str() == "Application",
        "Invalid session type"
    );
    for flag in ["Hidden", "NoDisplay", "Terminal"] {
        anyhow::ensure!(
            !key.boolean(group, flag).unwrap_or(false),
            "Session not eligible"
        );
    }
    let id = path
        .file_stem()
        .and_then(|name| name.to_str())
        .ok_or_else(|| anyhow::anyhow!("Invalid session ID"))?
        .to_owned();
    anyhow::ensure!(
        !id.is_empty()
            && id
                .bytes()
                .all(|c| c.is_ascii_alphanumeric() || b"._-".contains(&c)),
        "Invalid session ID"
    );
    let name = key.locale_string(group, "Name", None)?.to_string();
    anyhow::ensure!(
        !name.trim().is_empty() && name.len() <= 256,
        "Invalid session name"
    );
    let exec = key.string(group, "Exec")?;
    anyhow::ensure!(exec.len() <= 4096, "Session command too long");
    let command = glib::shell_parse_argv(exec.as_str())?
        .into_iter()
        .map(|arg| {
            let arg = arg
                .into_string()
                .map_err(|_| anyhow::anyhow!("Non-Unicode argument"))?;
            let mut result = String::new();
            let mut chars = arg.chars();
            while let Some(c) = chars.next() {
                if c != '%' {
                    result.push(c);
                    continue;
                }
                match chars.next() {
                    Some('%') => result.push('%'),
                    Some('c') => result.push_str(&name),
                    Some('k') => result.push_str(
                        path.to_str()
                            .ok_or_else(|| anyhow::anyhow!("Non-Unicode path"))?,
                    ),
                    _ => anyhow::bail!("Unsupported session field code"),
                }
            }
            Ok(result)
        })
        .collect::<anyhow::Result<Vec<_>>>()?;
    anyhow::ensure!(
        !command.is_empty() && command.len() <= 64,
        "Invalid argument count"
    );
    let desktop = key
        .string(group, "DesktopNames")
        .map(|s| s.trim_end_matches(';').replace(';', ":"))
        .unwrap_or_else(|_| id.clone());
    anyhow::ensure!(
        !desktop.is_empty()
            && desktop
                .bytes()
                .all(|c| c.is_ascii_alphanumeric() || b":._-".contains(&c)),
        "Invalid desktop names"
    );
    let environment = vec![
        "XDG_SESSION_TYPE=wayland".into(),
        format!("XDG_SESSION_DESKTOP={id}"),
        format!("XDG_CURRENT_DESKTOP={desktop}"),
        format!("DESKTOP_SESSION={id}"),
    ];
    Ok(Session {
        id,
        name,
        command,
        environment,
    })
}
fn executable(name: &str) -> Option<PathBuf> {
    let paths = if Path::new(name).is_absolute() {
        vec![PathBuf::from(name)]
    } else {
        if name.contains('/') {
            return None;
        }
        ["/usr/local/bin", "/usr/bin", "/bin"]
            .into_iter()
            .map(|directory| Path::new(directory).join(name))
            .collect()
    };
    paths.into_iter().find(|path| {
        trusted(path) && std::fs::metadata(path).is_ok_and(|m| m.is_file() && m.mode() & 0o111 != 0)
    })
}
/// Worker-only: record parsing and executable preflight never run on GTK's thread.
pub fn load() -> Vec<Session> {
    let mut sessions = std::collections::BTreeMap::new();
    for directory in [
        "/usr/share/wayland-sessions",
        "/usr/local/share/wayland-sessions",
    ] {
        let directory = Path::new(directory);
        if !trusted(directory) {
            continue;
        }
        let Ok(entries) = std::fs::read_dir(directory) else {
            continue;
        };
        for entry in entries.flatten().take(128) {
            let path = entry.path();
            if path.extension().is_none_or(|ext| ext != "desktop") || !trusted(&path) {
                continue;
            }
            let result: anyhow::Result<Session> = (|| {
                anyhow::ensure!(
                    std::fs::metadata(&path)?.len() <= 65536,
                    "Session file too large"
                );
                let key = glib::KeyFile::new();
                key.load_from_file(&path, glib::KeyFileFlags::NONE)?;
                let mut session = parse(&path, &key)?;
                if let Ok(name) = key.string("Desktop Entry", "TryExec") {
                    anyhow::ensure!(executable(&name).is_some(), "TryExec unavailable");
                }
                let program = executable(&session.command[0])
                    .ok_or_else(|| anyhow::anyhow!("Session executable unavailable"))?;
                session.command[0] = program
                    .to_str()
                    .ok_or_else(|| anyhow::anyhow!("Non-Unicode program"))?
                    .into();
                Ok(session)
            })();
            if let Ok(session) = result {
                sessions.insert(session.id.clone(), session);
            }
        }
    }
    let mut result: Vec<_> = sessions.into_values().collect();
    result.sort_by(|a, b| {
        glib::CollationKey::from(a.name.as_str()).cmp(&glib::CollationKey::from(b.name.as_str()))
    });
    result
}
#[cfg(test)]
#[allow(clippy::expect_used)]
mod tests {
    use super::*;
    fn record(exec: &str) -> glib::KeyFile {
        let key = glib::KeyFile::new();
        key.load_from_data(
            &format!("[Desktop Entry]\nType=Application\nName=Asena\nExec={exec}\n"),
            glib::KeyFileFlags::NONE,
        )
        .expect("fixture");
        key
    }
    #[test]
    fn arguments_are_parsed_without_a_shell_and_unknown_field_codes_fail() {
        let value = parse(
            Path::new("/usr/share/wayland-sessions/asena.desktop"),
            &record("/bin/program '--name=%c' '$(touch /tmp/bad)' '%%'"),
        )
        .expect("valid record");
        assert_eq!(
            value.command,
            vec!["/bin/program", "--name=Asena", "$(touch /tmp/bad)", "%"]
        );
        assert!(parse(Path::new("asena.desktop"), &record("program %f")).is_err());
        let hidden = record("program");
        hidden.set_boolean("Desktop Entry", "Hidden", true);
        assert!(parse(Path::new("asena.desktop"), &hidden).is_err());
    }
    #[test]
    fn asena_takes_precedence_over_alphabetical_alternatives() {
        let session = |id: &str| Session {
            id: id.into(),
            name: id.into(),
            command: Vec::new(),
            environment: Vec::new(),
        };
        let mut sessions = vec![session("wayfire"), session("gnome"), session("asena")];
        assert_eq!(preferred(&sessions), Some(2));
        sessions.pop();
        assert_eq!(preferred(&sessions), Some(0));
        assert_eq!(preferred(&[]), None);
    }
}
