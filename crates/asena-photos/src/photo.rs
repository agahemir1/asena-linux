//! Photo library scanning, independent of GTK widgets.
use std::path::{Path, PathBuf};

pub const MAX_PHOTOS: usize = 5000;
const MAX_DEPTH: usize = 4;
const EXTENSIONS: [&str; 10] = [
    "jpg", "jpeg", "png", "webp", "gif", "bmp", "tif", "tiff", "svg", "avif",
];

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Photo {
    pub path: PathBuf,
    pub modified: i64,
}

pub fn is_image(path: &Path) -> bool {
    path.extension()
        .and_then(|extension| extension.to_str())
        .is_some_and(|extension| EXTENSIONS.contains(&extension.to_ascii_lowercase().as_str()))
}

fn modified(path: &Path) -> i64 {
    std::fs::metadata(path)
        .and_then(|metadata| metadata.modified())
        .ok()
        .and_then(|time| time.duration_since(std::time::UNIX_EPOCH).ok())
        .map(|duration| duration.as_secs() as i64)
        .unwrap_or(0)
}

/// Images below `root`, newest first. Hidden folders are skipped.
pub fn scan(root: &Path) -> Vec<Photo> {
    let mut photos = Vec::new();
    let mut pending = vec![(root.to_path_buf(), 0)];
    while let Some((directory, depth)) = pending.pop() {
        let Ok(entries) = std::fs::read_dir(&directory) else {
            continue;
        };
        for entry in entries.flatten() {
            let path = entry.path();
            let hidden = entry.file_name().to_string_lossy().starts_with('.');
            let Ok(kind) = entry.file_type() else {
                continue;
            };
            if kind.is_dir() && !hidden && depth < MAX_DEPTH {
                pending.push((path, depth + 1));
            } else if kind.is_file() && !hidden && is_image(&path) {
                photos.push(Photo {
                    modified: modified(&path),
                    path,
                });
                if photos.len() >= MAX_PHOTOS {
                    break;
                }
            }
        }
        if photos.len() >= MAX_PHOTOS {
            break;
        }
    }
    photos.sort_by(|a, b| {
        b.modified
            .cmp(&a.modified)
            .then_with(|| a.path.cmp(&b.path))
    });
    photos
}

/// Images in the same folder as `path`, by name, for browsing an opened file.
pub fn siblings(path: &Path) -> Vec<Photo> {
    let Some(directory) = path.parent() else {
        return Vec::new();
    };
    let Ok(entries) = std::fs::read_dir(directory) else {
        return Vec::new();
    };
    let mut photos: Vec<Photo> = entries
        .flatten()
        .map(|entry| entry.path())
        .filter(|path| path.is_file() && is_image(path))
        .take(MAX_PHOTOS)
        .map(|path| Photo {
            modified: modified(&path),
            path,
        })
        .collect();
    photos.sort_by(|a, b| a.path.cmp(&b.path));
    photos
}

/// Next index when browsing, wrapping around.
pub fn step(index: usize, len: usize, forward: bool) -> usize {
    if len == 0 {
        return 0;
    }
    if forward {
        (index + 1) % len
    } else {
        (index + len - 1) % len
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn recognizes_images_and_steps_around() {
        assert!(is_image(Path::new("/a/Tatil.JPG")));
        assert!(is_image(Path::new("b.webp")));
        assert!(!is_image(Path::new("notlar.txt")));
        assert!(!is_image(Path::new("noextension")));
        assert_eq!(step(0, 3, false), 2);
        assert_eq!(step(2, 3, true), 0);
        assert_eq!(step(0, 0, true), 0);
    }

    #[test]
    fn scans_nested_folders_newest_first() -> std::io::Result<()> {
        let root = std::env::temp_dir().join(format!("asena-photos-{}", std::process::id()));
        std::fs::create_dir_all(root.join("2026/ekim"))?;
        std::fs::create_dir_all(root.join(".gizli"))?;
        std::fs::write(root.join("a.png"), b"x")?;
        std::fs::write(root.join("2026/ekim/b.jpg"), b"x")?;
        std::fs::write(root.join(".gizli/c.png"), b"x")?;
        std::fs::write(root.join("not.txt"), b"x")?;
        let found = scan(&root);
        assert_eq!(found.len(), 2);
        assert_eq!(siblings(&root.join("a.png")).len(), 1);
        std::fs::remove_dir_all(root)
    }
}
