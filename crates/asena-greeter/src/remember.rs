//! Worker-only storage: the last username, never credentials or PAM messages.
use std::{
    io::Write,
    os::unix::fs::{OpenOptionsExt, PermissionsExt},
    path::PathBuf,
};
fn path() -> PathBuf {
    std::env::var_os("XDG_STATE_HOME")
        .map(PathBuf::from)
        .filter(|path| path.is_absolute())
        .unwrap_or_else(|| glib::home_dir().join(".local/state"))
        .join("asena-greeter/last-user")
}
pub fn valid(name: &str) -> bool {
    !name.is_empty()
        && name.len() <= 256
        && !name
            .chars()
            .any(|c| c.is_control() || matches!(c, ':' | '/'))
}
pub fn read() -> Option<String> {
    let path = path();
    if std::fs::metadata(&path).ok()?.len() > 256 {
        return None;
    }
    let value = std::fs::read_to_string(path).ok()?;
    valid(&value).then_some(value)
}
pub fn write(username: &str) -> std::io::Result<()> {
    if !valid(username) {
        return Err(std::io::Error::other("Invalid username"));
    }
    let path = path();
    let directory = path
        .parent()
        .ok_or_else(|| std::io::Error::other("Invalid state path"))?;
    std::fs::create_dir_all(directory)?;
    std::fs::set_permissions(directory, std::fs::Permissions::from_mode(0o700))?;
    let temporary = directory.join(format!(".last-user-{}", std::process::id()));
    let mut file = std::fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .mode(0o600)
        .open(&temporary)?;
    let result = (|| {
        file.write_all(username.as_bytes())?;
        file.sync_all()?;
        std::fs::rename(&temporary, &path)
    })();
    if result.is_err() {
        let _ = std::fs::remove_file(temporary);
    }
    result
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn invalid_identity_text_is_never_remembered() {
        assert!(valid("agah"));
        for name in ["", "user\nsecret", "user:secret", "../user"] {
            assert!(!valid(name));
        }
    }
}
