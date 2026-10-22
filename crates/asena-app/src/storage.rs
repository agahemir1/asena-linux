//! Per-application data locations and crash-safe writes.
use std::{
    io::Write,
    path::{Path, PathBuf},
};

/// `$XDG_DATA_HOME/asena/<name>`, created on first use by the caller.
pub fn data_dir(name: &str) -> PathBuf {
    glib::user_data_dir().join("asena").join(name)
}

/// Writes through a private temporary file and renames it into place, so a
/// crash never leaves a half written document behind.
pub fn write_atomic(path: &Path, bytes: &[u8]) -> std::io::Result<()> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    let mut temporary = path.as_os_str().to_owned();
    temporary.push(".tmp");
    let temporary = PathBuf::from(temporary);
    let mut options = std::fs::OpenOptions::new();
    options.create(true).truncate(true).write(true);
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt;
        options.mode(0o600);
    }
    let mut file = options.open(&temporary)?;
    file.write_all(bytes)?;
    file.sync_all()?;
    std::fs::rename(temporary, path)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn atomic_write_replaces_content() -> std::io::Result<()> {
        let directory = std::env::temp_dir().join(format!("asena-storage-{}", std::process::id()));
        let path = directory.join("nested/note.md");
        write_atomic(&path, b"first")?;
        write_atomic(&path, b"second")?;
        assert_eq!(std::fs::read(&path)?, b"second");
        assert!(!path.with_extension("md.tmp").exists());
        std::fs::remove_dir_all(directory)
    }
}
