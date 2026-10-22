//! Archive listing helpers and extraction safety checks for Asena Archive.
//!
//! Reading and extracting is done by `bsdtar` (libarchive-tools), which
//! supports ZIP, TAR (gzip, bzip2, xz, zstd), 7z, ISO and more.
use std::path::{Component, Path};

/// Command used for listing and extraction.
pub const TOOL: &str = "bsdtar";

/// Validates that an archive entry stays inside the destination directory,
/// rejecting absolute paths and `..` components (zip slip).
pub fn is_safe_extraction_path(target_dir: &Path, entry_path: &str) -> bool {
    let entry = Path::new(entry_path);
    if entry.is_absolute() || entry_path.contains('\0') {
        return false;
    }
    if entry.components().any(|component| {
        matches!(
            component,
            Component::ParentDir | Component::RootDir | Component::Prefix(_)
        )
    }) {
        return false;
    }
    target_dir.join(entry).starts_with(target_dir)
}

/// Entry names from `bsdtar -tf`, one per line, empty lines dropped.
pub fn parse_listing(output: &str) -> Vec<String> {
    output
        .lines()
        .map(|line| line.trim_end_matches('\r'))
        .filter(|line| !line.is_empty())
        .map(ToOwned::to_owned)
        .collect()
}

/// Folder name for extraction: the archive name without archive extensions.
pub fn folder_name(archive: &str) -> String {
    let lower = archive.to_lowercase();
    for suffix in [
        ".tar.gz", ".tar.xz", ".tar.bz2", ".tar.zst", ".tgz", ".tbz2", ".txz", ".zip", ".tar",
        ".7z", ".iso", ".rar",
    ] {
        if lower.ends_with(suffix) && archive.len() > suffix.len() {
            return archive[..archive.len() - suffix.len()].to_owned();
        }
    }
    archive.to_owned()
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    #[test]
    fn zip_slip_is_rejected() {
        let target = PathBuf::from("/home/asena/Çıkarılan");
        assert!(is_safe_extraction_path(&target, "belgeler/rapor.pdf"));
        assert!(is_safe_extraction_path(&target, "./a.txt"));
        assert!(!is_safe_extraction_path(&target, "../../etc/passwd"));
        assert!(!is_safe_extraction_path(&target, "a/../../b"));
        assert!(!is_safe_extraction_path(&target, "/etc/shadow"));
    }

    #[test]
    fn listing_and_folder_names() {
        assert_eq!(parse_listing("a/\na/b.txt\n\n"), vec!["a/", "a/b.txt"]);
        assert_eq!(folder_name("yedek.tar.gz"), "yedek");
        assert_eq!(folder_name("Fotoğraflar.ZIP"), "Fotoğraflar");
        assert_eq!(folder_name("README"), "README");
    }
}
