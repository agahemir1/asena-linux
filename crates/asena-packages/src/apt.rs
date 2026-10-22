//! APT work through libapt-pkg. Every function blocks and opens its own
//! cache; the service runs them on a worker thread, one at a time.
use asena_core::packages::{
    check_removals, valid_package_name, Action, Change, Plan, PlanError, Upgrade,
};
use rust_apt::{
    cache::Upgrade as UpgradeMode,
    error::AptErrors,
    new_cache,
    progress::{AcquireProgress, DynAcquireProgress, DynInstallProgress, InstallProgress},
    Cache,
};
use std::collections::HashSet;

/// One progress update for the `Progress` signal.
pub struct Progress {
    pub operation: &'static str,
    pub percent: f64,
    pub detail: String,
}

pub type Reporter = async_channel::Sender<Progress>;

#[derive(Debug)]
pub enum Error {
    Plan(PlanError),
    /// The package state changed since the plan was shown.
    Stale,
    Apt(String),
}

impl From<AptErrors> for Error {
    fn from(error: AptErrors) -> Self {
        Self::Apt(error.to_string())
    }
}

impl From<PlanError> for Error {
    fn from(error: PlanError) -> Self {
        Self::Plan(error)
    }
}

impl From<std::io::Error> for Error {
    fn from(error: std::io::Error) -> Self {
        Self::Apt(error.to_string())
    }
}

pub struct Request<'a> {
    pub install: &'a [String],
    pub remove: &'a [String],
}

fn open() -> Result<Cache, Error> {
    Ok(new_cache!()?)
}

/// Upgrades install new dependencies but never remove packages, like `apt upgrade`.
const UPGRADE: UpgradeMode = UpgradeMode::Upgrade;

fn mark(cache: &Cache, request: &Request<'_>) -> Result<(), Error> {
    if let Some(name) = request
        .install
        .iter()
        .chain(request.remove)
        .find(|name| !valid_package_name(name))
    {
        return Err(PlanError::InvalidName(name.clone()).into());
    }
    for name in request.install {
        let package = cache
            .get(name)
            .filter(|package| package.candidate().is_some())
            .ok_or_else(|| PlanError::Unknown(name.clone()))?;
        package.mark_install(true, true);
        package.protect();
    }
    for name in request.remove {
        let package = cache
            .get(name)
            .filter(|package| package.is_installed())
            .ok_or_else(|| PlanError::Unknown(name.clone()))?;
        package.mark_delete(false);
        package.protect();
    }
    cache.resolve(false)?;
    Ok(())
}

fn changes(cache: &Cache, requested: &HashSet<&str>) -> Vec<Change> {
    cache
        .get_changes(true)
        .map(|package| {
            let action = if package.marked_delete() {
                Action::Remove
            } else if package.marked_new_install() {
                Action::Install
            } else if package.marked_upgrade() {
                Action::Upgrade
            } else if package.marked_downgrade() {
                Action::Downgrade
            } else {
                Action::Reinstall
            };
            let version = if action == Action::Remove {
                package.installed()
            } else {
                package.install_version()
            };
            let name = package.name().to_owned();
            Change {
                requested: requested.contains(name.as_str()),
                name,
                action,
                version: version.map(|v| v.version().to_owned()).unwrap_or_default(),
            }
        })
        .collect()
}

/// Debian's Essential packages and those of priority required.
fn essential(cache: &Cache, name: &str) -> bool {
    cache.get(name).is_some_and(|package| {
        package.is_essential()
            || package
                .installed()
                .is_some_and(|version| version.priority_str().is_ok_and(|p| p == "required"))
    })
}

fn requested<'a>(request: &'a Request<'_>) -> HashSet<&'a str> {
    request
        .install
        .iter()
        .chain(request.remove)
        .map(String::as_str)
        .collect()
}

/// What the request would change, without changing anything.
pub fn plan(request: &Request<'_>) -> Result<Plan, Error> {
    let cache = open()?;
    mark(&cache, request)?;
    let changes = changes(&cache, &requested(request));
    if changes.is_empty() {
        return Err(PlanError::Empty.into());
    }
    check_removals(&changes, |name| essential(&cache, name))?;
    let depcache = cache.depcache();
    let disk_delta_bytes = match depcache.disk_size() {
        rust_apt::util::DiskSpace::Require(bytes) => i64::try_from(bytes).unwrap_or(i64::MAX),
        rust_apt::util::DiskSpace::Free(bytes) => -i64::try_from(bytes).unwrap_or(i64::MAX),
    };
    Ok(Plan {
        id: String::new(),
        download_bytes: download_size(&cache),
        changes,
        disk_delta_bytes,
    })
}

fn download_size(cache: &Cache) -> u64 {
    cache
        .get_changes(false)
        .filter(|package| !package.marked_delete())
        .filter_map(|package| package.install_version())
        .filter(|version| !version.is_installed())
        .map(|version| version.size())
        .sum()
}

/// Applies `request` when it still produces exactly `expected`.
pub fn commit(
    request: &Request<'_>,
    expected: &[Change],
    reporter: &Reporter,
) -> Result<(), Error> {
    let cache = open()?;
    mark(&cache, request)?;
    let changes = changes(&cache, &requested(request));
    if changes != expected {
        return Err(Error::Stale);
    }
    check_removals(&changes, |name| essential(&cache, name))?;
    apply(cache, reporter)
}

fn apply(cache: Cache, reporter: &Reporter) -> Result<(), Error> {
    // Keep changed configuration files and take defaults; nobody can answer dpkg.
    rust_apt::config::Config::new()
        .set_vector("Dpkg::Options", &vec!["--force-confdef", "--force-confold"]);
    let mut acquire = AcquireProgress::new(Reporting::new("download", reporter));
    let mut install = InstallProgress::new(Reporting::new("install", reporter));
    cache.commit(&mut acquire, &mut install)?;
    Ok(())
}

pub fn refresh(reporter: &Reporter) -> Result<(), Error> {
    let cache = open()?;
    let mut progress = AcquireProgress::new(Reporting::new("refresh", reporter));
    cache.update(&mut progress)?;
    Ok(())
}

pub fn upgrades() -> Result<Vec<Upgrade>, Error> {
    let cache = open()?;
    cache.upgrade(UPGRADE)?;
    Ok(cache
        .get_changes(true)
        .filter(|package| package.marked_upgrade())
        .filter_map(|package| {
            let from = package.installed()?.version().to_owned();
            let candidate = package.install_version()?;
            let security = candidate.package_files().any(|file| {
                file.archive()
                    .is_some_and(|archive| archive.ends_with("-security"))
                    || file.label().is_some_and(|label| label == "Debian-Security")
            });
            Some(Upgrade {
                name: package.name().to_owned(),
                from,
                to: candidate.version().to_owned(),
                security,
            })
        })
        .collect())
}

/// Downloads every pending upgrade into APT's archive cache.
pub fn download_upgrades(reporter: &Reporter) -> Result<u32, Error> {
    let cache = open()?;
    cache.upgrade(UPGRADE)?;
    let count = u32::try_from(cache.get_changes(false).count()).unwrap_or(u32::MAX);
    if count == 0 {
        return Ok(0);
    }
    rust_apt::util::apt_lock()?;
    let mut progress = AcquireProgress::new(Reporting::new("download", reporter));
    let result = cache.get_archives(&mut progress);
    rust_apt::util::apt_unlock();
    result.map_err(|error| Error::Apt(error.to_string()))?;
    Ok(count)
}

/// Installs the downloaded upgrades; used during system-update.target.
pub fn install_upgrades(reporter: &Reporter) -> Result<u32, Error> {
    let cache = open()?;
    cache.upgrade(UPGRADE)?;
    let count = u32::try_from(cache.get_changes(false).count()).unwrap_or(u32::MAX);
    if count > 0 {
        apply(cache, reporter)?;
    }
    Ok(count)
}

/// dpkg leaves files in its updates directory when a run was interrupted;
/// APT refuses to work until `dpkg --configure -a` finished it.
pub fn needs_repair() -> bool {
    std::fs::read_dir("/var/lib/dpkg/updates").is_ok_and(|mut entries| entries.next().is_some())
}

pub fn repair(reporter: &Reporter) -> Result<(), Error> {
    let status = std::process::Command::new("dpkg")
        .args(["--configure", "-a", "--force-confdef", "--force-confold"])
        .env("DEBIAN_FRONTEND", "noninteractive")
        .status()?;
    if !status.success() {
        return Err(Error::Apt(format!("dpkg --configure -a: {status}")));
    }
    let cache = open()?;
    cache.resolve(true)?;
    if cache.get_changes(false).next().is_some() {
        apply(cache, reporter)?;
    }
    Ok(())
}

/// Forwards APT's download and dpkg progress to the service.
struct Reporting {
    operation: &'static str,
    reporter: Reporter,
    last: i32,
}

impl Reporting {
    fn new(operation: &'static str, reporter: &Reporter) -> Self {
        Self {
            operation,
            reporter: reporter.clone(),
            last: -1,
        }
    }

    fn send(&mut self, percent: f64, detail: String) {
        // Whole percent steps are enough for a progress bar and the splash.
        let whole = percent.floor() as i32;
        if whole == self.last && detail.is_empty() {
            return;
        }
        self.last = whole;
        let _ = self.reporter.try_send(Progress {
            operation: self.operation,
            percent,
            detail,
        });
    }
}

impl DynAcquireProgress for Reporting {
    fn pulse_interval(&self) -> usize {
        500_000
    }
    fn hit(&mut self, _item: &rust_apt::raw::ItemDesc) {}
    fn fetch(&mut self, _item: &rust_apt::raw::ItemDesc) {}
    fn fail(&mut self, item: &rust_apt::raw::ItemDesc) {
        self.send(-1.0, item.description());
    }
    fn pulse(&mut self, status: &rust_apt::raw::AcqTextStatus, _owner: &rust_apt::raw::PkgAcquire) {
        self.send(status.percent(), String::new());
    }
    fn done(&mut self, _item: &rust_apt::raw::ItemDesc) {}
    fn start(&mut self) {
        self.send(0.0, String::new());
    }
    fn stop(&mut self, _status: &rust_apt::raw::AcqTextStatus) {
        self.send(100.0, String::new());
    }
}

impl DynInstallProgress for Reporting {
    fn status_changed(
        &mut self,
        pkgname: String,
        steps_done: u64,
        total_steps: u64,
        _action: String,
    ) {
        let percent = if total_steps == 0 {
            0.0
        } else {
            steps_done as f64 * 100.0 / total_steps as f64
        };
        self.send(percent, pkgname);
    }
    fn error(&mut self, pkgname: String, _steps_done: u64, _total_steps: u64, error: String) {
        self.send(-1.0, format!("{pkgname}: {error}"));
    }
}
