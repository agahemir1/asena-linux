//! Worker-only usage persistence. A malformed or oversized cache is disposable.
use std::{
    collections::BTreeMap,
    io::{Read, Write},
    path::Path,
};
pub type Usage = BTreeMap<String, u32>;
pub fn load(path: &Path) -> Usage {
    let Ok(file) = std::fs::File::open(path) else {
        return Usage::new();
    };
    let mut bytes = Vec::new();
    if file.take(1_048_577).read_to_end(&mut bytes).is_err() || bytes.len() > 1_048_576 {
        return Usage::new();
    }
    serde_json::from_slice(&bytes).unwrap_or_default()
}
pub fn save(path: &Path, usage: &Usage) -> Result<(), std::io::Error> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    let temporary = path.with_extension("json.tmp");
    let mut options = std::fs::OpenOptions::new();
    options.create(true).truncate(true).write(true);
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt;
        options.mode(0o600);
    }
    let mut file = options.open(&temporary)?;
    serde_json::to_writer(&mut file, usage)?;
    file.flush()?;
    std::fs::rename(temporary, path)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn atomic_round_trip_and_corrupt_cache() -> Result<(), std::io::Error> {
        let directory =
            std::env::temp_dir().join(format!("asena-usage-test-{}", std::process::id()));
        let path = directory.join("usage.json");
        assert!(load(&path).is_empty());
        let usage = Usage::from([("org.example.App.desktop".into(), 42)]);
        save(&path, &usage)?;
        assert_eq!(load(&path), usage);
        std::fs::write(&path, b"not JSON")?;
        assert!(load(&path).is_empty());
        assert!(!path.with_extension("json.tmp").exists());
        std::fs::remove_dir_all(directory)
    }
}
