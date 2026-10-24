//! Readers for `/proc`: processes, processor time, memory and network totals.
use std::{collections::HashMap, path::Path};

#[derive(Clone, Debug, PartialEq)]
pub struct Process {
    pub pid: u32,
    pub name: String,
    /// The full command line, empty for kernel threads.
    pub command: String,
    pub uid: u32,
    /// User plus system time in clock ticks.
    pub ticks: u64,
    /// Resident memory in bytes.
    pub memory: u64,
}

/// `/proc/<pid>/stat`: the name sits in parentheses and may itself contain
/// spaces and parentheses, so fields are counted from the last `)`.
pub fn parse_stat(text: &str) -> Option<(String, u64)> {
    let open = text.find('(')?;
    let close = text.rfind(')')?;
    let name = text.get(open + 1..close)?.to_owned();
    let fields: Vec<&str> = text.get(close + 1..)?.split_whitespace().collect();
    // After the name: state is field 3; utime and stime are fields 14 and 15.
    let utime: u64 = fields.get(11)?.parse().ok()?;
    let stime: u64 = fields.get(12)?.parse().ok()?;
    Some((name, utime + stime))
}

/// Real user id and resident memory from `/proc/<pid>/status`.
pub fn parse_status(text: &str) -> (Option<u32>, u64) {
    let mut uid = None;
    let mut memory = 0;
    for line in text.lines() {
        if let Some(value) = line.strip_prefix("Uid:") {
            uid = value
                .split_whitespace()
                .next()
                .and_then(|id| id.parse().ok());
        } else if let Some(value) = line.strip_prefix("VmRSS:") {
            memory = value
                .split_whitespace()
                .next()
                .and_then(|kib| kib.parse::<u64>().ok())
                .unwrap_or(0)
                * 1024;
        }
    }
    (uid, memory)
}

pub fn command_line(raw: &[u8]) -> String {
    raw.split(|byte| *byte == 0)
        .filter(|part| !part.is_empty())
        .map(String::from_utf8_lossy)
        .collect::<Vec<_>>()
        .join(" ")
}

/// The kernel cuts process names to 15 bytes; the program file name is used
/// when it continues the cut name.
pub fn display_name(name: &str, command: &str) -> String {
    let program = command
        .split(' ')
        .next()
        .and_then(|path| path.rsplit('/').next())
        .unwrap_or_default();
    if name.len() >= 15 && program.len() > name.len() && program.starts_with(name) {
        program.to_owned()
    } else {
        name.to_owned()
    }
}

/// Every process that can be read; processes that exit while reading are skipped.
pub fn processes(root: &Path) -> Vec<Process> {
    let Ok(entries) = std::fs::read_dir(root) else {
        return Vec::new();
    };
    entries
        .flatten()
        .filter_map(|entry| {
            let pid: u32 = entry.file_name().to_str()?.parse().ok()?;
            let directory = entry.path();
            let (name, ticks) = parse_stat(&std::fs::read_to_string(directory.join("stat")).ok()?)?;
            let (uid, memory) =
                parse_status(&std::fs::read_to_string(directory.join("status")).ok()?);
            let command = std::fs::read(directory.join("cmdline"))
                .map(|raw| command_line(&raw))
                .unwrap_or_default();
            Some(Process {
                pid,
                name,
                command,
                uid: uid?,
                ticks,
                memory,
            })
        })
        .collect()
}

/// Total and idle ticks of all processors from the first line of `/proc/stat`.
pub fn parse_cpu(text: &str) -> Option<(u64, u64)> {
    let line = text.lines().find(|line| line.starts_with("cpu "))?;
    let values: Vec<u64> = line
        .split_whitespace()
        .skip(1)
        .filter_map(|value| value.parse().ok())
        .collect();
    if values.len() < 4 {
        return None;
    }
    // Guest time is already counted in user time.
    let total = values.iter().take(8).sum();
    let idle = values[3] + values.get(4).copied().unwrap_or(0);
    Some((total, idle))
}

pub fn processor_count(text: &str) -> usize {
    text.lines()
        .filter(|line| {
            line.starts_with("cpu") && line.as_bytes().get(3).is_some_and(u8::is_ascii_digit)
        })
        .count()
        .max(1)
}

#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Memory {
    pub total: u64,
    pub available: u64,
    pub swap_total: u64,
    pub swap_free: u64,
}

pub fn parse_memory(text: &str) -> Memory {
    let values: HashMap<&str, u64> = text
        .lines()
        .filter_map(|line| {
            let (name, value) = line.split_once(':')?;
            let kib: u64 = value.split_whitespace().next()?.parse().ok()?;
            Some((name, kib * 1024))
        })
        .collect();
    let value = |name: &str| values.get(name).copied().unwrap_or(0);
    Memory {
        total: value("MemTotal"),
        available: value("MemAvailable"),
        swap_total: value("SwapTotal"),
        swap_free: value("SwapFree"),
    }
}

/// Received and sent bytes over all interfaces except loopback.
pub fn parse_network(text: &str) -> (u64, u64) {
    text.lines()
        .skip(2)
        .filter_map(|line| {
            let (name, values) = line.split_once(':')?;
            if name.trim() == "lo" {
                return None;
            }
            let values: Vec<u64> = values
                .split_whitespace()
                .filter_map(|value| value.parse().ok())
                .collect();
            Some((*values.first()?, *values.get(8)?))
        })
        .fold((0, 0), |(received, sent), (r, s)| (received + r, sent + s))
}

/// Read and written bytes of the physical disks in `/proc/diskstats`; `disks`
/// are the names under `/sys/block` that have a device behind them.
pub fn parse_disks(text: &str, disks: &[String]) -> (u64, u64) {
    text.lines()
        .filter_map(|line| {
            let fields: Vec<&str> = line.split_whitespace().collect();
            let name = fields.get(2)?;
            if !disks.iter().any(|disk| disk == name) {
                return None;
            }
            let read: u64 = fields.get(5)?.parse().ok()?;
            let written: u64 = fields.get(9)?.parse().ok()?;
            // Sectors are always 512 bytes in this file.
            Some((read * 512, written * 512))
        })
        .fold((0, 0), |(read, written), (r, w)| (read + r, written + w))
}

pub fn physical_disks(root: &Path) -> Vec<String> {
    std::fs::read_dir(root)
        .map(|entries| {
            entries
                .flatten()
                .filter(|entry| entry.path().join("device").exists())
                .filter_map(|entry| entry.file_name().into_string().ok())
                .collect()
        })
        .unwrap_or_default()
}

/// User names by id from `/etc/passwd`.
pub fn parse_users(text: &str) -> HashMap<u32, String> {
    text.lines()
        .filter_map(|line| {
            let mut fields = line.split(':');
            let name = fields.next()?;
            let uid = fields.nth(1)?.parse().ok()?;
            Some((uid, name.to_owned()))
        })
        .collect()
}

/// Share of all processors, in percent, for `ticks` used out of `total` processor ticks.
pub fn share(ticks: u64, total: u64) -> f64 {
    if total == 0 {
        0.0
    } else {
        (ticks as f64 * 100.0 / total as f64).clamp(0.0, 100.0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn stat_names_may_contain_parentheses() {
        let text = "1234 (Web (Content)) S 1 1234 1234 0 -1 4194560 100 0 0 0 250 75 0 0 20 0 30 0 5000 0 0";
        assert_eq!(parse_stat(text), Some(("Web (Content)".into(), 325)));
        assert_eq!(parse_stat("broken"), None);
    }

    #[test]
    fn status_gives_owner_and_resident_memory() {
        let text = "Name:\tfoot\nUid:\t1000\t1000\t1000\t1000\nVmRSS:\t   20480 kB\n";
        assert_eq!(parse_status(text), (Some(1000), 20480 * 1024));
        assert_eq!(
            parse_status("Name:\tkthreadd\nUid:\t0\t0\t0\t0\n"),
            (Some(0), 0)
        );
    }

    #[test]
    fn cut_names_take_the_program_name() {
        assert_eq!(
            display_name("asena-polkit-ag", "/usr/bin/asena-polkit-agent --replace"),
            "asena-polkit-agent"
        );
        assert_eq!(
            display_name(
                "Web Content",
                "/usr/lib/firefox-esr/firefox-esr -contentproc"
            ),
            "Web Content"
        );
        assert_eq!(display_name("kworker/0:1", ""), "kworker/0:1");
    }

    #[test]
    fn command_lines_are_joined() {
        assert_eq!(
            command_line(b"/usr/bin/foot\0--server\0"),
            "/usr/bin/foot --server"
        );
        assert_eq!(command_line(b""), "");
    }

    #[test]
    fn processor_totals_and_count() {
        let text = "cpu  100 5 50 800 20 3 2 0 7 0\ncpu0 50 2 25 400 10 1 1 0 0 0\ncpu1 50 3 25 400 10 2 1 0 0 0\nintr 1\n";
        assert_eq!(parse_cpu(text), Some((980, 820)));
        assert_eq!(processor_count(text), 2);
        assert_eq!(processor_count(""), 1);
    }

    #[test]
    fn memory_in_bytes() {
        let text = "MemTotal:  8000 kB\nMemFree: 1000 kB\nMemAvailable:  6000 kB\nSwapTotal: 4096 kB\nSwapFree: 4000 kB\n";
        let memory = parse_memory(text);
        assert_eq!(memory.total, 8000 * 1024);
        assert_eq!(memory.available, 6000 * 1024);
        assert_eq!(memory.swap_total - memory.swap_free, 96 * 1024);
    }

    #[test]
    fn network_skips_loopback() {
        let text = "Inter-|   Receive\n face |bytes packets\n    lo: 500 5 0 0 0 0 0 0 500 5 0 0 0 0 0 0\n  eth0: 1000 10 0 0 0 0 0 0 300 3 0 0 0 0 0 0\nwlan0: 24 1 0 0 0 0 0 0 6 1 0 0 0 0 0 0\n";
        assert_eq!(parse_network(text), (1024, 306));
    }

    #[test]
    fn disks_count_only_whole_devices() {
        let text = "   8       0 sda 100 0 2000 50 40 0 800 30 0 0 0\n   8       1 sda1 90 0 1800 45 40 0 800 30 0 0 0\n   7       0 loop0 5 0 10 0 0 0 0 0 0 0 0\n";
        assert_eq!(parse_disks(text, &["sda".into()]), (2000 * 512, 800 * 512));
        assert_eq!(parse_disks(text, &[]), (0, 0));
    }

    #[test]
    fn users_by_id() {
        let users = parse_users("root:x:0:0:root:/root:/bin/bash\nasena:x:1000:1000:Asena,,,:/home/asena:/bin/bash\nbroken\n");
        assert_eq!(users.get(&1000).map(String::as_str), Some("asena"));
        assert_eq!(users.len(), 2);
    }

    #[test]
    fn shares_are_clamped() {
        assert_eq!(share(50, 200), 25.0);
        assert_eq!(share(5, 0), 0.0);
        assert_eq!(share(300, 200), 100.0);
    }

    #[test]
    fn this_process_is_listed() {
        let own = std::process::id();
        let list = processes(Path::new("/proc"));
        if cfg!(target_os = "linux") {
            assert!(list
                .iter()
                .any(|process| process.pid == own && process.memory > 0));
        }
    }
}
