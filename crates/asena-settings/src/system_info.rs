//! Readers for the About panel: operating system, processor and memory.

/// An os-release value with quotes removed; `None` when missing or empty.
fn os_release_value(os_release: &str, key: &str) -> Option<String> {
    os_release.lines().find_map(|line| {
        let value = line.strip_prefix(key)?.strip_prefix('=')?;
        let value = value.trim().trim_matches('"').trim_matches('\'');
        (!value.is_empty()).then(|| value.to_owned())
    })
}

/// `PRETTY_NAME` from os-release.
pub fn os_name(os_release: &str) -> Option<String> {
    os_release_value(os_release, "PRETTY_NAME")
}

/// Icon name of the distribution logo, `LOGO` in os-release.
pub fn os_logo(os_release: &str) -> Option<String> {
    os_release_value(os_release, "LOGO")
}

/// Processor model and number of logical processors from /proc/cpuinfo.
pub fn processor(cpuinfo: &str) -> Option<(String, usize)> {
    let model = cpuinfo.lines().find_map(|line| {
        let (key, value) = line.split_once(':')?;
        (key.trim() == "model name").then(|| value.split_whitespace().collect::<Vec<_>>().join(" "))
    })?;
    let count = cpuinfo
        .lines()
        .filter(|line| line.split(':').next().map(str::trim) == Some("processor"))
        .count()
        .max(1);
    Some((model, count))
}

/// Total memory in bytes from /proc/meminfo.
pub fn memory_bytes(meminfo: &str) -> Option<u64> {
    meminfo.lines().find_map(|line| {
        let rest = line.strip_prefix("MemTotal:")?;
        let kibibytes: u64 = rest.trim().trim_end_matches("kB").trim().parse().ok()?;
        Some(kibibytes * 1024)
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_system_files() {
        assert_eq!(
            os_name("NAME=\"Debian GNU/Linux\"\nPRETTY_NAME=\"Debian GNU/Linux 13 (trixie)\"\n")
                .as_deref(),
            Some("Debian GNU/Linux 13 (trixie)")
        );
        assert_eq!(os_name("NAME=x"), None);
        assert_eq!(
            os_logo("ID=asena\nLOGO=asena-logo\n").as_deref(),
            Some("asena-logo")
        );
        assert_eq!(os_logo("LOGO=\"\"\n"), None);
        assert_eq!(os_logo("ID=debian\n"), None);
        let cpu = "processor\t: 0\nmodel name\t: AMD   Ryzen 5  5600X\nprocessor\t: 1\nmodel name\t: AMD Ryzen 5 5600X\n";
        assert_eq!(processor(cpu), Some(("AMD Ryzen 5 5600X".to_owned(), 2)));
        assert_eq!(
            memory_bytes("MemTotal:        8048576 kB\nMemFree: 1 kB"),
            Some(8048576 * 1024)
        );
        assert_eq!(memory_bytes("nothing"), None);
    }
}
