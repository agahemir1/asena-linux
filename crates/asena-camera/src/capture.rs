//! Where photos and videos go, what they are called, and the recording pipeline.
use asena_app::prelude::gettext;
use std::path::{Path, PathBuf};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Kind {
    Photo,
    Video,
}

/// `~/Pictures/Camera` or `~/Videos/Camera`, the home folder without those.
pub fn directory(kind: Kind) -> PathBuf {
    let special = match kind {
        Kind::Photo => glib::UserDirectory::Pictures,
        Kind::Video => glib::UserDirectory::Videos,
    };
    glib::user_special_dir(special)
        .unwrap_or_else(glib::home_dir)
        .join(gettext("Camera"))
}

/// File name for a capture taken at `stamp` (for example "2026-10-23 14-05-33").
pub fn file_name(kind: Kind, stamp: &str) -> String {
    match kind {
        Kind::Photo => format!("{}.jpg", gettext("Photo {date}").replace("{date}", stamp)),
        Kind::Video => format!("{}.webm", gettext("Video {date}").replace("{date}", stamp)),
    }
}

/// `name` in `directory`, numbered when two captures fall in the same second.
pub fn unique(directory: &Path, name: &str, exists: impl Fn(&Path) -> bool) -> PathBuf {
    let candidate = directory.join(name);
    if !exists(&candidate) {
        return candidate;
    }
    let (stem, extension) = name.rsplit_once('.').unwrap_or((name, ""));
    (2..)
        .map(|number| directory.join(format!("{stem} ({number}).{extension}")))
        .find(|path| !exists(path))
        .unwrap_or(candidate)
}

/// `0:05`, `12:30`, `1:02:03`.
pub fn elapsed(seconds: u64) -> String {
    let (hours, minutes, seconds) = (seconds / 3600, seconds / 60 % 60, seconds % 60);
    if hours > 0 {
        format!("{hours}:{minutes:02}:{seconds:02}")
    } else {
        format!("{minutes}:{seconds:02}")
    }
}

/// The recording branch: VP8 video and, with a microphone, Opus sound in WebM.
/// The location is quoted for `gst-launch` syntax.
pub fn recording_description(location: &Path, sound: bool) -> String {
    let location = location
        .to_string_lossy()
        .replace('\\', "\\\\")
        .replace('"', "\\\"");
    let mut description = format!(
        "queue name=video-queue ! videoconvert ! vp8enc deadline=1 cpu-used=8 threads=4 \
         ! queue ! webmmux name=mux offset-to-zero=true ! filesink name=file location=\"{location}\""
    );
    if sound {
        description.push_str(
            " pulsesrc name=microphone ! queue ! audioconvert ! audioresample ! opusenc ! queue ! mux.",
        );
    }
    description
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn names_carry_the_time_and_type() {
        assert_eq!(
            file_name(Kind::Photo, "2026-10-23 14-05-33"),
            "Photo 2026-10-23 14-05-33.jpg"
        );
        assert!(file_name(Kind::Video, "x").ends_with(".webm"));
    }

    #[test]
    fn names_are_numbered_when_taken() {
        let taken = [PathBuf::from("/p/a.jpg"), PathBuf::from("/p/a (2).jpg")];
        let path = unique(Path::new("/p"), "a.jpg", |path| {
            taken.iter().any(|t| t == path)
        });
        assert_eq!(path, PathBuf::from("/p/a (3).jpg"));
        assert_eq!(
            unique(Path::new("/p"), "b.jpg", |_| false),
            PathBuf::from("/p/b.jpg")
        );
    }

    #[test]
    fn elapsed_time_labels() {
        assert_eq!(elapsed(5), "0:05");
        assert_eq!(elapsed(750), "12:30");
        assert_eq!(elapsed(3723), "1:02:03");
    }

    #[test]
    fn recording_quotes_the_location() {
        let description = recording_description(Path::new("/home/a/Video \"x\".webm"), false);
        assert!(description.contains("location=\"/home/a/Video \\\"x\\\".webm\""));
        assert!(!description.contains("pulsesrc"));
        assert!(recording_description(Path::new("/v.webm"), true).contains("pulsesrc"));
    }
}
