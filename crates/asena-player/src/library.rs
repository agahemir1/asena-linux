//! Media library scanning, independent of GTK widgets.
use asena_core::text::fold;
use std::path::{Path, PathBuf};

const MAX_ITEMS: usize = 5000;
const MAX_DEPTH: usize = 5;
const AUDIO: [&str; 9] = [
    "mp3", "ogg", "oga", "opus", "flac", "wav", "m4a", "aac", "wma",
];
const VIDEO: [&str; 7] = ["mp4", "mkv", "webm", "ogv", "avi", "mov", "m4v"];

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Kind {
    Audio,
    Video,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Track {
    pub path: PathBuf,
    pub title: String,
    pub kind: Kind,
}

pub fn kind_of(path: &Path) -> Option<Kind> {
    let extension = path.extension()?.to_str()?.to_ascii_lowercase();
    if AUDIO.contains(&extension.as_str()) {
        Some(Kind::Audio)
    } else if VIDEO.contains(&extension.as_str()) {
        Some(Kind::Video)
    } else {
        None
    }
}

/// File name without extension, underscores shown as spaces.
pub fn title_of(path: &Path) -> String {
    path.file_stem()
        .map(|stem| stem.to_string_lossy().replace('_', " "))
        .unwrap_or_default()
}

pub fn track(path: PathBuf) -> Option<Track> {
    let kind = kind_of(&path)?;
    Some(Track {
        title: title_of(&path),
        path,
        kind,
    })
}

/// Media files below the given folders, deduplicated by path.
pub fn scan(roots: &[PathBuf]) -> Vec<Track> {
    let mut tracks: Vec<Track> = Vec::new();
    for root in roots {
        let mut pending = vec![(root.clone(), 0)];
        while let Some((directory, depth)) = pending.pop() {
            let Ok(entries) = std::fs::read_dir(&directory) else {
                continue;
            };
            for entry in entries.flatten() {
                if tracks.len() >= MAX_ITEMS {
                    return tracks;
                }
                let path = entry.path();
                if entry.file_name().to_string_lossy().starts_with('.') {
                    continue;
                }
                let Ok(file_type) = entry.file_type() else {
                    continue;
                };
                if file_type.is_dir() && depth < MAX_DEPTH {
                    pending.push((path, depth + 1));
                } else if file_type.is_file() && !tracks.iter().any(|item| item.path == path) {
                    if let Some(track) = track(path) {
                        tracks.push(track);
                    }
                }
            }
        }
    }
    tracks
}

pub fn matches(track: &Track, query: &str) -> bool {
    let query = fold(query.trim());
    query.is_empty() || fold(&track.title).contains(&query)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn detects_kinds_titles_and_search() {
        assert_eq!(kind_of(Path::new("a/Şarkı.FLAC")), Some(Kind::Audio));
        assert_eq!(kind_of(Path::new("film.mkv")), Some(Kind::Video));
        assert_eq!(kind_of(Path::new("belge.pdf")), None);
        assert_eq!(title_of(Path::new("/m/Yeni_Parça.mp3")), "Yeni Parça");
        let item = Track {
            path: "x.mp3".into(),
            title: "İstanbul Hatırası".into(),
            kind: Kind::Audio,
        };
        assert!(matches(&item, "istanbul"));
        assert!(!matches(&item, "ankara"));
    }
}
