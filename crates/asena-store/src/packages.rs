//! Client of the package service `org.asenalinux.Packages` (K21).
use asena_app::prelude::gettext;
use asena_core::packages::{errors, PackagesProxy, Plan, Upgrade, UpgradeResult};

thread_local! {
    static SYSTEM: std::cell::RefCell<Option<zbus::Connection>> = const { std::cell::RefCell::new(None) };
}

/// One system bus connection for the whole application: the service hands a
/// plan only to the connection that asked for it.
async fn system() -> zbus::Result<zbus::Connection> {
    if let Some(connection) = SYSTEM.with(|slot| slot.borrow().clone()) {
        return Ok(connection);
    }
    let connection = zbus::Connection::system().await?;
    SYSTEM.with(|slot| *slot.borrow_mut() = Some(connection.clone()));
    Ok(connection)
}

pub async fn proxy() -> zbus::Result<PackagesProxy<'static>> {
    PackagesProxy::new(&system().await?).await
}

/// A message for people, from the service's error name.
pub fn message(error: &zbus::Error) -> String {
    let name = match error {
        zbus::Error::MethodError(name, _, _) => name.as_str(),
        _ => "",
    };
    match name {
        errors::NOT_AUTHORIZED => gettext("You are not allowed to change the installed software."),
        errors::INVALID_NAME | errors::UNKNOWN_PACKAGE => {
            gettext("This package is not in the configured archives.")
        }
        errors::PROTECTED => gettext("This change would remove a part of the system."),
        errors::NOTHING_TO_DO => gettext("There is nothing to change."),
        errors::STALE_PLAN => gettext("The installed software changed in the meantime. Try again."),
        errors::BUSY => gettext("Another software change is running."),
        errors::FAILED => match error {
            zbus::Error::MethodError(_, Some(detail), _) => detail.clone(),
            _ => gettext("The software change failed."),
        },
        _ => gettext("The package service is unavailable."),
    }
}

pub async fn plan(install: &[&str], remove: &[&str]) -> zbus::Result<Plan> {
    let text = proxy().await?.plan(install, remove).await?;
    serde_json::from_str(&text).map_err(|error| zbus::Error::Failure(error.to_string()))
}

/// Plans installing a local `.deb`; the service reads it through the file descriptor.
pub async fn plan_file(path: std::path::PathBuf) -> zbus::Result<Plan> {
    let file = gio::spawn_blocking(move || std::fs::File::open(path))
        .await
        .map_err(|_| zbus::Error::Failure("stopped".into()))?
        .map_err(|error| zbus::Error::Failure(error.to_string()))?;
    use std::os::fd::AsFd;
    let text = proxy()
        .await?
        .plan_file(zbus::zvariant::Fd::from(file.as_fd()))
        .await?;
    serde_json::from_str(&text).map_err(|error| zbus::Error::Failure(error.to_string()))
}

/// Package, version and summary of a `.deb` file, from its control data.
pub fn describe_deb(path: &std::path::Path) -> Option<(String, String, String)> {
    let output = std::process::Command::new("dpkg-deb")
        .arg("--field")
        .arg(path)
        .args(["Package", "Version", "Description"])
        .output()
        .ok()?;
    if !output.status.success() {
        return None;
    }
    let text = String::from_utf8_lossy(&output.stdout);
    let field = |name: &str| {
        text.lines()
            .find_map(|line| line.strip_prefix(&format!("{name}: ")))
            .unwrap_or_default()
            .trim()
            .to_owned()
    };
    let name = field("Package");
    (!name.is_empty()).then(|| (name, field("Version"), field("Description")))
}

pub async fn upgrades() -> zbus::Result<Vec<Upgrade>> {
    let text = proxy().await?.upgrades().await?;
    serde_json::from_str(&text).map_err(|error| zbus::Error::Failure(error.to_string()))
}

pub async fn last_upgrade() -> Option<UpgradeResult> {
    let text = proxy().await.ok()?.last_upgrade().await.ok()?;
    serde_json::from_str(&text).ok()
}

/// Restarts through logind; the scheduled upgrade runs during the restart.
pub async fn restart() -> zbus::Result<()> {
    let connection = system().await?;
    let manager = zbus::Proxy::new(
        &connection,
        "org.freedesktop.login1",
        "/org/freedesktop/login1",
        "org.freedesktop.login1.Manager",
    )
    .await?;
    manager.call::<_, _, ()>("Reboot", &(true,)).await
}
