//! Software updates in the background (K28, Phase 6F-D).
//!
//! Once a day, when the network is up and not metered, the package lists are
//! refreshed and pending upgrades are downloaded; one notification offers to
//! install them while restarting. Installing always needs the user, in
//! Software. The outcome of an upgrade applied while restarting is shown once.
use asena_core::{
    i18n::{gettext, ngettext},
    packages::{PackagesProxy, Upgrade, UpgradeResult},
};
use futures_lite::StreamExt;
use gio::prelude::*;
use std::{
    collections::HashMap,
    path::PathBuf,
    time::{Duration, SystemTime, UNIX_EPOCH},
};
use zbus::zvariant::Value;

const FIRST_CHECK: Duration = Duration::from_secs(5 * 60);
const INTERVAL: u64 = 24 * 60 * 60;
const POLL: Duration = Duration::from_secs(60 * 60);

fn state_file(name: &str) -> PathBuf {
    std::env::var_os("XDG_STATE_HOME")
        .map(PathBuf::from)
        .filter(|path| path.is_absolute())
        .unwrap_or_else(|| glib::home_dir().join(".local/state"))
        .join("asena")
        .join(name)
}

fn now() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|time| time.as_secs())
        .unwrap_or_default()
}

async fn read_stamp(name: &'static str) -> u64 {
    gio::spawn_blocking(move || {
        std::fs::read_to_string(state_file(name))
            .ok()
            .and_then(|text| text.trim().parse().ok())
            .unwrap_or(0)
    })
    .await
    .unwrap_or(0)
}

async fn write_stamp(name: &'static str, value: u64) {
    let _ = gio::spawn_blocking(move || {
        let path = state_file(name);
        if let Some(directory) = path.parent() {
            std::fs::create_dir_all(directory)?;
        }
        std::fs::write(path, value.to_string())
    })
    .await;
}

/// Whether a check is due `elapsed` seconds after the last one.
fn due(last: u64, now: u64) -> bool {
    now < last || now - last >= INTERVAL
}

#[zbus::proxy(
    interface = "org.freedesktop.Notifications",
    default_service = "org.freedesktop.Notifications",
    default_path = "/org/freedesktop/Notifications"
)]
trait Notifications {
    #[allow(clippy::too_many_arguments)]
    fn notify(
        &self,
        app_name: &str,
        replaces_id: u32,
        app_icon: &str,
        summary: &str,
        body: &str,
        actions: &[&str],
        hints: &HashMap<&str, Value<'_>>,
        expire_timeout: i32,
    ) -> zbus::Result<u32>;
    #[zbus(signal)]
    fn action_invoked(&self, id: u32, action_key: &str) -> zbus::Result<()>;
}

/// Shows a notification that opens Software's updates page when clicked.
async fn notify(session: &zbus::Connection, summary: &str, body: &str) {
    let Ok(notifications) = NotificationsProxy::new(session).await else {
        return;
    };
    let Ok(mut invoked) = notifications.receive_action_invoked().await else {
        return;
    };
    let hints = HashMap::from([("desktop-entry", Value::from("org.asenalinux.Store"))]);
    let open = gettext("Open Software");
    let Ok(id) = notifications
        .notify(
            &gettext("Software"),
            0,
            "org.asenalinux.Store",
            summary,
            body,
            &["default", &open],
            &hints,
            -1,
        )
        .await
    else {
        return;
    };
    glib::spawn_future_local(async move {
        while let Some(signal) = invoked.next().await {
            let Ok(args) = signal.args() else {
                continue;
            };
            if *args.id() == id {
                if let Err(error) = gio::Subprocess::newv(
                    &["asena-store".as_ref(), "--updates".as_ref()],
                    gio::SubprocessFlags::NONE,
                ) {
                    tracing::warn!(%error, "Could not open Software");
                }
                break;
            }
        }
    });
}

async fn check(system: &zbus::Connection, session: &zbus::Connection) {
    let monitor = gio::NetworkMonitor::default();
    if !monitor.is_network_available() || monitor.is_network_metered() {
        return;
    }
    let Ok(packages) = PackagesProxy::new(system).await else {
        return;
    };
    if let Err(error) = packages.refresh().await {
        tracing::info!(%error, "Package lists were not refreshed");
        return;
    }
    write_stamp("updates-checked", now()).await;
    let upgrades: Vec<Upgrade> = match packages.upgrades().await {
        Ok(text) => serde_json::from_str(&text).unwrap_or_default(),
        Err(_) => return,
    };
    if upgrades.is_empty() {
        return;
    }
    if let Err(error) = packages.download_upgrades().await {
        tracing::info!(%error, "Upgrades were not downloaded");
        return;
    }
    let count = u32::try_from(upgrades.len()).unwrap_or(u32::MAX);
    let security = upgrades.iter().filter(|upgrade| upgrade.security).count();
    let summary = ngettext(
        "{count} update is ready",
        "{count} updates are ready",
        count,
    )
    .replace("{count}", &count.to_string());
    let body = if security > 0 {
        gettext("Includes security updates. They are installed while the computer restarts.")
    } else {
        gettext("They are installed while the computer restarts.")
    };
    notify(session, &summary, &body).await;
}

/// Reports an upgrade applied while restarting, once.
async fn report_last_upgrade(system: &zbus::Connection, session: &zbus::Connection) {
    let Ok(packages) = PackagesProxy::new(system).await else {
        return;
    };
    let Some(result) = packages
        .last_upgrade()
        .await
        .ok()
        .and_then(|text| serde_json::from_str::<UpgradeResult>(&text).ok())
    else {
        return;
    };
    if result.finished <= read_stamp("upgrade-reported").await {
        return;
    }
    write_stamp("upgrade-reported", result.finished).await;
    if result.success {
        let summary = ngettext(
            "{count} package was updated",
            "{count} packages were updated",
            result.packages,
        )
        .replace("{count}", &result.packages.to_string());
        notify(session, &summary, "").await;
    } else {
        notify(
            session,
            &gettext("The system update could not be installed"),
            &result.error.unwrap_or_default(),
        )
        .await;
    }
}

pub async fn run(session: zbus::Connection) {
    // The nested development session must not touch the real system.
    if std::env::var_os("ASENA_DEV").is_some() {
        return;
    }
    let Ok(system) = zbus::Connection::system().await else {
        return;
    };
    report_last_upgrade(&system, &session).await;
    async_io::Timer::after(FIRST_CHECK).await;
    loop {
        if due(read_stamp("updates-checked").await, now()) {
            check(&system, &session).await;
        }
        async_io::Timer::after(POLL).await;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn checks_run_once_a_day_and_after_clock_changes() {
        assert!(due(0, 1_000_000));
        assert!(!due(1_000_000, 1_000_000 + INTERVAL - 1));
        assert!(due(1_000_000, 1_000_000 + INTERVAL));
        assert!(due(2_000_000, 1_000_000));
    }
}
