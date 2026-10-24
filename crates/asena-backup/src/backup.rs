//! Backups of the home folder with rsync. Each backup is a dated folder; on
//! Linux filesystems unchanged files are hard links to the previous backup, so
//! every backup is complete but only changes take space.
use std::{
    io::{BufRead, BufReader},
    path::{Path, PathBuf},
    process::{Child, Command, Stdio},
};

/// Folders under the destination: `Asena Backup/<user>@<host>/`.
pub const ROOT: &str = "Asena Backup";
/// Newest backups kept; older ones are removed after a successful backup.
pub const KEEP: usize = 10;
const PARTIAL: &str = ".partial";

/// Caches, the trash and installed applications are not personal files.
pub const EXCLUDED: &[&str] = &[
    "/.cache/",
    "/.local/share/Trash/",
    "/.local/share/flatpak/",
    "/.var/app/*/cache/",
    "/.thumbnails/",
    "/snap/",
];

pub fn machine_folder(destination: &Path, user: &str, host: &str) -> PathBuf {
    destination.join(ROOT).join(format!("{user}@{host}"))
}

/// Backup folder names are their start time, so they sort by name.
pub fn is_backup_name(name: &str) -> bool {
    let bytes = name.as_bytes();
    name.len() == 19
        && bytes.iter().enumerate().all(|(index, byte)| match index {
            4 | 7 => *byte == b'-',
            10 => *byte == b' ',
            13 | 16 => *byte == b'-',
            _ => byte.is_ascii_digit(),
        })
}

/// Complete backups in `folder`, newest first.
pub fn list(folder: &Path) -> Vec<String> {
    let mut names: Vec<String> = std::fs::read_dir(folder)
        .map(|entries| {
            entries
                .flatten()
                .filter(|entry| entry.path().is_dir())
                .filter_map(|entry| entry.file_name().into_string().ok())
                .filter(|name| is_backup_name(name))
                .collect()
        })
        .unwrap_or_default();
    names.sort_unstable_by(|a, b| b.cmp(a));
    names
}

/// Backups beyond the newest `keep`.
pub fn expired(backups: &[String], keep: usize) -> &[String] {
    backups.get(keep..).unwrap_or_default()
}

/// Filesystems that store owners, permissions and hard links.
pub fn keeps_history(filesystem: &str) -> bool {
    matches!(
        filesystem,
        "ext2" | "ext3" | "ext4" | "btrfs" | "xfs" | "f2fs" | "zfs"
    )
}

pub fn arguments(
    source: &Path,
    target: &Path,
    previous: Option<&Path>,
    history: bool,
) -> Vec<String> {
    let mut arguments: Vec<String> = if history {
        vec![
            "--archive".into(),
            "--hard-links".into(),
            "--acls".into(),
            "--xattrs".into(),
        ]
    } else {
        // FAT, exFAT and NTFS keep neither owners nor links, and round times to two seconds.
        vec![
            "--recursive".into(),
            "--times".into(),
            "--links".into(),
            "--modify-window=2".into(),
        ]
    };
    arguments.extend([
        "--delete".into(),
        "--delete-excluded".into(),
        "--one-file-system".into(),
        "--no-inc-recursive".into(),
        "--info=progress2".into(),
        "--outbuf=L".into(),
    ]);
    for pattern in EXCLUDED {
        arguments.push(format!("--exclude={pattern}"));
    }
    if let Some(previous) = previous.filter(|_| history) {
        arguments.push(format!("--link-dest={}", previous.display()));
    }
    arguments.push(format!("{}/", source.display()));
    arguments.push(format!("{}/", target.display()));
    arguments
}

/// The percentage in an `--info=progress2` line such as
/// `  1,234,567  45%   12.34MB/s    0:00:12 (xfr#3, to-chk=10/20)`.
pub fn progress(line: &str) -> Option<f64> {
    line.split_whitespace()
        .find_map(|part| part.strip_suffix('%'))
        .and_then(|number| number.parse::<f64>().ok())
        .map(|percent| (percent / 100.0).clamp(0.0, 1.0))
}

pub enum Outcome {
    Done,
    Failed(String),
    Stopped,
}

/// Runs one backup; `report` receives progress between 0 and 1. The rsync
/// process is handed to `started` so the caller can stop it.
pub fn run(
    source: &Path,
    folder: &Path,
    stamp: &str,
    history: bool,
    started: impl FnOnce(&Child),
    mut report: impl FnMut(f64),
) -> Outcome {
    if !is_backup_name(stamp) || folder.starts_with(source) || !source.is_dir() {
        return Outcome::Failed("Invalid backup source or destination".into());
    }
    if let Err(error) = std::fs::create_dir_all(folder) {
        return Outcome::Failed(error.to_string());
    }
    let backups = list(folder);
    let target = folder.join(format!("{stamp}{PARTIAL}"));
    if target.exists() || folder.join(stamp).exists() {
        return Outcome::Failed("A backup with this date already exists".into());
    }
    let previous = backups.first().map(|name| folder.join(name));
    let mut child = match Command::new("rsync")
        .args(arguments(source, &target, previous.as_deref(), history))
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
    {
        Ok(child) => child,
        Err(error) => return Outcome::Failed(error.to_string()),
    };
    // Drain stderr concurrently; a long series of permission errors must not
    // fill its pipe and stop rsync while the worker waits for progress.
    let errors = child.stderr.take().map(|mut stderr| {
        std::thread::spawn(move || {
            use std::io::Read;
            let mut bytes = Vec::new();
            let mut chunk = [0u8; 4096];
            while let Ok(count) = stderr.read(&mut chunk) {
                if count == 0 {
                    break;
                }
                if bytes.len() < 65536 {
                    bytes.extend_from_slice(&chunk[..count]);
                }
            }
            bytes
        })
    });
    started(&child);
    if let Some(stdout) = child.stdout.take() {
        // Progress lines end with a carriage return.
        let mut reader = BufReader::new(stdout);
        let mut line = Vec::new();
        while reader
            .read_until(b'\r', &mut line)
            .map(|read| read > 0)
            .unwrap_or(false)
        {
            if let Some(value) = progress(&String::from_utf8_lossy(&line)) {
                report(value);
            }
            line.clear();
        }
    }
    let output = match child.wait_with_output() {
        Ok(output) => output,
        Err(error) => return Outcome::Failed(error.to_string()),
    };
    let errors = errors
        .and_then(|thread| thread.join().ok())
        .unwrap_or_default();
    match output.status.code() {
        // 24: some files vanished while copying, which is normal for a live home folder.
        Some(0 | 24) => {}
        None => {
            let _ = std::fs::remove_dir_all(&target);
            return Outcome::Stopped;
        }
        Some(_) => {
            // An incomplete copy is never listed; remove it so failed runs do not fill the drive.
            let _ = std::fs::remove_dir_all(&target);
            return Outcome::Failed(String::from_utf8_lossy(&errors).trim().to_owned());
        }
    }
    if let Err(error) = std::fs::rename(&target, folder.join(stamp)) {
        return Outcome::Failed(error.to_string());
    }
    // Without hard links every backup is a full copy; only the new one stays.
    let backups = list(folder);
    for name in expired(&backups, if history { KEEP } else { 1 }) {
        let _ = std::fs::remove_dir_all(folder.join(name));
    }
    Outcome::Done
}

/// Copies a backup back into the home folder. Newer files are replaced, files
/// added since the backup stay.
pub fn restore(backup: &Path, home: &Path) -> Result<(), String> {
    let output = Command::new("rsync")
        .args(["--recursive", "--links", "--times", "--perms"])
        .arg(format!("{}/", backup.display()))
        .arg(format!("{}/", home.display()))
        .output()
        .map_err(|error| error.to_string())?;
    if output.status.success() {
        Ok(())
    } else {
        Err(String::from_utf8_lossy(&output.stderr).trim().to_owned())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn backup_names_are_timestamps() {
        assert!(is_backup_name("2026-10-23 14-05-33"));
        assert!(!is_backup_name("2026-10-23 14-05-33.partial"));
        assert!(!is_backup_name("Notes"));
        assert!(!is_backup_name("2026-1x-23 14-05-33"));
    }

    #[test]
    fn only_old_backups_expire() {
        let backups: Vec<String> = (0..12).map(|n| format!("b{n}")).collect();
        assert_eq!(expired(&backups, 10), ["b10", "b11"]);
        assert!(expired(&backups[..3], 10).is_empty());
    }

    #[test]
    fn progress_lines_give_a_fraction() {
        assert_eq!(
            progress("    1,234,567  45%   12.34MB/s    0:00:12 (xfr#3, to-chk=10/20)"),
            Some(0.45)
        );
        assert_eq!(progress("sending incremental file list"), None);
        assert_eq!(progress("  100%"), Some(1.0));
    }

    #[test]
    fn linux_drives_link_to_the_previous_backup() {
        let arguments = arguments(
            Path::new("/home/a"),
            Path::new("/media/a/USB/x.partial"),
            Some(Path::new("/media/a/USB/old")),
            true,
        );
        assert!(arguments.contains(&"--archive".to_owned()));
        assert!(arguments.contains(&"--link-dest=/media/a/USB/old".to_owned()));
        assert!(arguments.contains(&"--exclude=/.cache/".to_owned()));
        assert_eq!(arguments[arguments.len() - 2], "/home/a/");
        assert_eq!(arguments[arguments.len() - 1], "/media/a/USB/x.partial/");
    }

    #[test]
    fn other_drives_copy_without_links() {
        let arguments = arguments(
            Path::new("/h"),
            Path::new("/t"),
            Some(Path::new("/p")),
            false,
        );
        assert!(arguments.contains(&"--modify-window=2".to_owned()));
        assert!(!arguments
            .iter()
            .any(|argument| argument.starts_with("--link-dest")));
        assert!(keeps_history("ext4"));
        assert!(!keeps_history("exfat"));
    }

    #[test]
    fn machine_folders_name_the_user_and_host() {
        assert_eq!(
            machine_folder(Path::new("/media/USB"), "asena", "laptop"),
            PathBuf::from("/media/USB/Asena Backup/asena@laptop")
        );
    }

    #[test]
    fn a_backup_copies_and_keeps_history() -> Result<(), Box<dyn std::error::Error>> {
        if Command::new("rsync").arg("--version").output().is_err() {
            return Ok(());
        }
        let root = std::env::temp_dir().join(format!("asena-backup-{}", std::process::id()));
        let home = root.join("home");
        let folder = root.join("backups");
        std::fs::create_dir_all(home.join(".cache"))?;
        std::fs::write(home.join("note.txt"), "first")?;
        std::fs::write(home.join(".cache/skip"), "cache")?;
        let first = run(&home, &folder, "2026-10-23 10-00-00", true, |_| {}, |_| {});
        assert!(matches!(first, Outcome::Done));
        std::fs::write(home.join("note.txt"), "second")?;
        let second = run(&home, &folder, "2026-10-23 11-00-00", true, |_| {}, |_| {});
        assert!(matches!(second, Outcome::Done));
        assert_eq!(
            list(&folder),
            ["2026-10-23 11-00-00", "2026-10-23 10-00-00"]
        );
        assert_eq!(
            std::fs::read_to_string(folder.join("2026-10-23 10-00-00/note.txt"))?,
            "first"
        );
        assert!(!folder.join("2026-10-23 11-00-00/.cache/skip").exists());
        std::fs::write(home.join("note.txt"), "changed")?;
        restore(&folder.join("2026-10-23 11-00-00"), &home)?;
        assert_eq!(std::fs::read_to_string(home.join("note.txt"))?, "second");
        std::fs::remove_dir_all(root)?;
        Ok(())
    }
}
