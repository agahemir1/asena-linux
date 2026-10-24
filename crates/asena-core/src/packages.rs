//! Contract of the privileged package service `org.asenalinux.Packages` (K21).
//!
//! The service runs as root on the system bus; every change goes through
//! polkit. Plans and upgrade lists travel as JSON so the contract stays
//! readable from `busctl` and stable across library versions.
use serde::{Deserialize, Serialize};

pub const PACKAGES_ID: &str = "org.asenalinux.Packages";
pub const PACKAGES_PATH: &str = "/org/asenalinux/Packages";

/// polkit actions, see data/polkit/org.asenalinux.packages.policy.
pub const ACTION_REFRESH: &str = "org.asenalinux.packages.refresh";
pub const ACTION_DOWNLOAD: &str = "org.asenalinux.packages.download";
pub const ACTION_INSTALL: &str = "org.asenalinux.packages.install";
pub const ACTION_REMOVE: &str = "org.asenalinux.packages.remove";
pub const ACTION_UPGRADE: &str = "org.asenalinux.packages.upgrade";
pub const ACTION_REPAIR: &str = "org.asenalinux.packages.repair";

/// Packages whose removal would leave the system without a desktop, a boot
/// path or a way to repair itself. Plans that remove them are refused;
/// Debian's Essential and required packages are refused as well.
pub const PROTECTED: &[&str] = &[
    "asena-desktop",
    "apt",
    "dpkg",
    "grub-efi-amd64",
    "grub-efi-amd64-signed",
    "shim-signed",
    "linux-image-amd64",
    "systemd",
    "systemd-sysv",
    "sudo",
    "polkitd",
    "network-manager",
    "greetd",
    "wayfire",
];

/// Debian Policy 5.6.1: lower case letters, digits, `+`, `-` and `.`, at
/// least two characters, starting with a letter or digit. Anything else,
/// including option-like input, never reaches APT.
pub fn valid_package_name(name: &str) -> bool {
    let mut characters = name.chars();
    let Some(first) = characters.next() else {
        return false;
    };
    name.len() >= 2
        && name.len() <= 128
        && (first.is_ascii_lowercase() || first.is_ascii_digit())
        && characters
            .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || matches!(c, '+' | '-' | '.'))
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum Action {
    Install,
    Upgrade,
    Downgrade,
    Reinstall,
    Remove,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Change {
    pub name: String,
    pub action: Action,
    /// Candidate version for installs, the installed one for removals.
    pub version: String,
    /// Asked for by the caller, not pulled in as a dependency.
    pub requested: bool,
}

/// What a set of installs and removals does, computed without changing the
/// system. `Commit` applies it only while the package state is unchanged.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct Plan {
    pub id: String,
    pub changes: Vec<Change>,
    pub download_bytes: u64,
    /// Installed size difference; negative when space is freed.
    pub disk_delta_bytes: i64,
}

impl Plan {
    pub fn removes(&self) -> impl Iterator<Item = &Change> {
        self.changes
            .iter()
            .filter(|change| change.action == Action::Remove)
    }
    pub fn needs_removal_authorization(&self) -> bool {
        self.removes().next().is_some()
    }
}

/// Why a plan cannot be used.
#[derive(Debug, PartialEq, Eq, thiserror::Error)]
pub enum PlanError {
    #[error("not a valid package name: {0}")]
    InvalidName(String),
    #[error("package not found: {0}")]
    Unknown(String),
    #[error("would remove a protected package: {0}")]
    Protected(String),
    #[error("nothing to do")]
    Empty,
}

/// Rejects plans that remove protected, Essential or required packages.
pub fn check_removals<'a>(
    changes: impl IntoIterator<Item = &'a Change>,
    essential: impl Fn(&str) -> bool,
) -> Result<(), PlanError> {
    for change in changes {
        if change.action == Action::Remove
            && (PROTECTED.contains(&change.name.as_str()) || essential(&change.name))
        {
            return Err(PlanError::Protected(change.name.clone()));
        }
    }
    Ok(())
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Upgrade {
    pub name: String,
    pub from: String,
    pub to: String,
    /// The candidate comes from a security archive.
    pub security: bool,
}

/// Outcome of the last upgrade applied while restarting.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct UpgradeResult {
    /// Seconds since the Unix epoch.
    pub finished: u64,
    pub success: bool,
    pub packages: u32,
    pub error: Option<String>,
}

/// D-Bus error names of the service, `org.asenalinux.Packages.Error.*`.
pub mod errors {
    pub const NOT_AUTHORIZED: &str = "org.asenalinux.Packages.Error.NotAuthorized";
    pub const INVALID_NAME: &str = "org.asenalinux.Packages.Error.InvalidName";
    pub const UNKNOWN_PACKAGE: &str = "org.asenalinux.Packages.Error.UnknownPackage";
    pub const PROTECTED: &str = "org.asenalinux.Packages.Error.Protected";
    pub const NOTHING_TO_DO: &str = "org.asenalinux.Packages.Error.NothingToDo";
    pub const STALE_PLAN: &str = "org.asenalinux.Packages.Error.StalePlan";
    pub const BUSY: &str = "org.asenalinux.Packages.Error.Busy";
    pub const FAILED: &str = "org.asenalinux.Packages.Error.Failed";
}

/// Names of installed packages in a dpkg status file. Reading it needs no
/// privileges, so Software does not ask the service for the installed state.
pub fn installed_packages(status: &str) -> std::collections::HashSet<String> {
    status
        .split("\n\n")
        .filter_map(|stanza| {
            let mut name = None;
            let mut installed = false;
            for line in stanza.lines() {
                if let Some(value) = line.strip_prefix("Package: ") {
                    name = Some(value.trim());
                } else if let Some(value) = line.strip_prefix("Status: ") {
                    installed = value.trim().ends_with(" installed");
                }
            }
            installed.then_some(name?.to_owned())
        })
        .collect()
}

/// Packages a dpkg run left half done, for example after a failing
/// maintainer script; `dpkg --configure -a` has to finish or undo them.
pub fn interrupted_packages(status: &str) -> Vec<String> {
    const STATES: [&str; 5] = [
        "half-installed",
        "half-configured",
        "unpacked",
        "triggers-awaited",
        "triggers-pending",
    ];
    status
        .split("\n\n")
        .filter_map(|stanza| {
            let mut name = None;
            let mut broken = false;
            for line in stanza.lines() {
                if let Some(value) = line.strip_prefix("Package: ") {
                    name = Some(value.trim());
                } else if let Some(value) = line.strip_prefix("Status: ") {
                    broken = value
                        .split_whitespace()
                        .last()
                        .is_some_and(|state| STATES.contains(&state));
                }
            }
            broken.then_some(name?.to_owned())
        })
        .collect()
}

#[zbus::proxy(
    interface = "org.asenalinux.Packages",
    default_service = "org.asenalinux.Packages",
    default_path = "/org/asenalinux/Packages"
)]
pub trait Packages {
    /// Downloads the package lists (`apt update`).
    fn refresh(&self) -> zbus::Result<()>;
    /// JSON `Plan` for the given installs and removals; changes nothing.
    fn plan(&self, install: &[&str], remove: &[&str]) -> zbus::Result<String>;
    /// Plans installing a package file the caller opened for reading.
    fn plan_file(&self, package: zbus::zvariant::Fd<'_>) -> zbus::Result<String>;
    /// Applies a plan returned to the same caller.
    fn commit(&self, plan_id: &str) -> zbus::Result<()>;
    /// JSON list of `Upgrade`.
    fn upgrades(&self) -> zbus::Result<String>;
    /// Downloads every pending upgrade; returns how many packages are ready.
    fn download_upgrades(&self) -> zbus::Result<u32>;
    /// Installs the downloaded upgrades during the next restart.
    fn schedule_upgrade(&self) -> zbus::Result<()>;
    fn cancel_upgrade(&self) -> zbus::Result<()>;
    /// JSON `UpgradeResult`, empty when no upgrade ran yet.
    fn last_upgrade(&self) -> zbus::Result<String>;
    /// Finishes an interrupted dpkg run.
    fn repair(&self) -> zbus::Result<()>;
    #[zbus(property)]
    fn busy(&self) -> zbus::Result<bool>;
    #[zbus(property)]
    fn upgrade_scheduled(&self) -> zbus::Result<bool>;
    #[zbus(property)]
    fn needs_repair(&self) -> zbus::Result<bool>;
    /// `operation` is refresh, download, install or repair; `percent` -1 when unknown.
    #[zbus(signal)]
    fn progress(&self, operation: &str, percent: f64, detail: &str) -> zbus::Result<()>;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn package_names_follow_debian_policy() {
        for name in [
            "hello",
            "libc6",
            "g++",
            "lib32z1-dev",
            "xserver-xorg-video-all",
            "a0",
        ] {
            assert!(valid_package_name(name), "{name}");
        }
        for name in [
            "",
            "a",
            "-o",
            "--allow-unauthenticated",
            "Hello",
            "hello world",
            "pkg:amd64",
            "../etc",
            ".hidden",
            "+plus",
            "paket/ç",
        ] {
            assert!(!valid_package_name(name), "{name}");
        }
        assert!(!valid_package_name(&"a".repeat(129)));
    }

    fn change(name: &str, action: Action) -> Change {
        Change {
            name: name.into(),
            action,
            version: "1".into(),
            requested: false,
        }
    }

    #[test]
    fn protected_and_essential_removals_are_refused() {
        let installs = [
            change("sudo", Action::Install),
            change("hello", Action::Remove),
        ];
        assert_eq!(check_removals(&installs, |_| false), Ok(()));
        let removes_sudo = [
            change("hello", Action::Remove),
            change("sudo", Action::Remove),
        ];
        assert_eq!(
            check_removals(&removes_sudo, |_| false),
            Err(PlanError::Protected("sudo".into()))
        );
        let removes_bash = [change("bash", Action::Remove)];
        assert_eq!(
            check_removals(&removes_bash, |name| name == "bash"),
            Err(PlanError::Protected("bash".into()))
        );
    }

    #[test]
    fn installed_packages_come_from_the_status_file() {
        let status = concat!(
            "Package: hello\nStatus: install ok installed\nVersion: 2.10-5\n\n",
            "Package: cowsay\nStatus: deinstall ok config-files\n\n",
            "Package: sl\nStatus: install ok half-configured\n\n",
            "Package: bash\nEssential: yes\nStatus: install ok installed\n",
        );
        let installed = installed_packages(status);
        assert!(installed.contains("hello") && installed.contains("bash"));
        assert!(!installed.contains("cowsay") && !installed.contains("sl"));
        assert_eq!(interrupted_packages(status), ["sl"]);
        assert!(interrupted_packages("Package: a\nStatus: install ok unpacked\n").len() == 1);
        assert!(interrupted_packages("Package: a\nStatus: deinstall ok config-files\n").is_empty());
    }

    #[test]
    fn plans_round_trip_as_json() -> Result<(), serde_json::Error> {
        let plan = Plan {
            id: "abc".into(),
            changes: vec![
                change("hello", Action::Install),
                change("cowsay", Action::Remove),
            ],
            download_bytes: 1024,
            disk_delta_bytes: -2048,
        };
        let text = serde_json::to_string(&plan)?;
        assert!(text.contains("\"remove\""));
        assert_eq!(serde_json::from_str::<Plan>(&text)?, plan);
        assert!(plan.needs_removal_authorization());
        Ok(())
    }
}
