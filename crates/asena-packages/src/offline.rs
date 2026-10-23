//! Upgrades applied while restarting (K28, Phase 6F-D).
//!
//! `ScheduleUpgrade` points systemd's `/system-update` marker at APT's archive
//! cache; systemd then boots into system-update.target, where
//! asena-offline-update.service runs `asena-packages --offline-update`.
use crate::apt;
use asena_core::{i18n::gettext, packages::UpgradeResult};
use std::{
    path::Path,
    time::{SystemTime, UNIX_EPOCH},
};

pub const MARKER: &str = "/system-update";
const ARCHIVES: &str = "/var/cache/apt/archives";
pub const RESULT: &str = "/var/lib/asena/packages/last-upgrade.json";

/// Only Asena's own marker counts; another updater may own a different one.
pub fn scheduled() -> bool {
    std::fs::read_link(MARKER).is_ok_and(|target| target == Path::new(ARCHIVES))
}

pub fn schedule() -> std::io::Result<()> {
    if scheduled() {
        return Ok(());
    }
    if std::fs::symlink_metadata(MARKER).is_ok() {
        return Err(std::io::Error::new(
            std::io::ErrorKind::AlreadyExists,
            "another system update is scheduled",
        ));
    }
    std::os::unix::fs::symlink(ARCHIVES, MARKER)
}

pub fn cancel() -> std::io::Result<()> {
    if scheduled() {
        std::fs::remove_file(MARKER)?;
    }
    Ok(())
}

pub fn last_result() -> String {
    std::fs::read_to_string(RESULT).unwrap_or_default()
}

fn write_result(result: &UpgradeResult) -> anyhow::Result<()> {
    let path = Path::new(RESULT);
    if let Some(directory) = path.parent() {
        std::fs::create_dir_all(directory)?;
    }
    let temporary = path.with_extension("json.new");
    std::fs::write(&temporary, serde_json::to_vec(result)?)?;
    std::fs::rename(temporary, path)?;
    Ok(())
}

/// Shows the progress on the boot splash, when there is one.
fn splash(text: &str) {
    let _ = std::process::Command::new("plymouth")
        .args(["display-message", "--text", text])
        .status();
}

pub fn run() -> anyhow::Result<()> {
    if !scheduled() {
        return Ok(());
    }
    // The marker goes first: a failing upgrade must not run again on every boot.
    std::fs::remove_file(MARKER)?;
    let (reporter, progress) = async_channel::unbounded::<apt::Progress>();
    let forward = std::thread::spawn(move || {
        while let Ok(update) = progress.recv_blocking() {
            if update.operation == "install" && update.percent >= 0.0 {
                splash(
                    &gettext("Installing updates: {percent} %")
                        .replace("{percent}", &format!("{:.0}", update.percent)),
                );
            }
        }
    });
    splash(&gettext("Installing updates"));
    let outcome = apt::install_upgrades(&reporter);
    drop(reporter);
    let _ = forward.join();
    let finished = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|time| time.as_secs())
        .unwrap_or_default();
    let result = match outcome {
        Ok(packages) => UpgradeResult {
            finished,
            success: true,
            packages,
            error: None,
        },
        Err(error) => {
            tracing::error!(%error, "Offline upgrade failed");
            UpgradeResult {
                finished,
                success: false,
                packages: 0,
                error: Some(error.to_string()),
            }
        }
    };
    if let Err(error) = write_result(&result) {
        tracing::error!(%error, "Could not record the upgrade result");
    }
    // `systemctl reboot` would ask logind over D-Bus first, which this boot
    // does not start; systemd itself takes the reboot job.
    let status = std::process::Command::new("systemctl")
        .args([
            "--no-block",
            "start",
            "reboot.target",
            "--job-mode=replace-irreversibly",
        ])
        .status()?;
    anyhow::ensure!(
        status.success(),
        "systemctl could not start reboot.target: {status}"
    );
    Ok(())
}
