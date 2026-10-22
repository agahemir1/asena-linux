//! Shared, fail-closed lock acquisition. Starting a process is not readiness.
use crate::error::ServiceError;
use asena_core::{
    dbus::{LockProxy, LOCK_ID},
    i18n::gettext,
};
use futures_lite::StreamExt;
use std::{
    path::{Path, PathBuf},
    process::{Command, Stdio},
    time::Duration,
};

/// Worker-only policy preflight. Never fall back to PAM's generic "other" stack.
pub fn trusted_policy() -> bool {
    use std::os::unix::fs::MetadataExt;
    std::fs::metadata("/etc/pam.d/asena-lock").is_ok_and(|metadata| {
        metadata.is_file() && metadata.uid() == 0 && metadata.mode() & 0o022 == 0
    })
}
pub async fn request(program: PathBuf) -> Result<(), ServiceError> {
    futures_lite::future::race(acquire(program, false), async {
        async_io::Timer::after(Duration::from_secs(5)).await;
        Err(ServiceError(gettext(
            "The session lock did not become ready.",
        )))
    })
    .await
    .map(|_| ())
}
struct Locked {
    connection: zbus::Connection,
    owner: String,
    scope: String,
}
pub struct SleepLease(Locked);
impl SleepLease {
    pub async fn ready(&self) -> Result<(), ServiceError> {
        futures_lite::future::race(self.confirm_ready(), async {
            async_io::Timer::after(Duration::from_secs(2)).await;
            Err(ServiceError(gettext(
                "The session lock did not become ready.",
            )))
        })
        .await
    }
    async fn confirm_ready(&self) -> Result<(), ServiceError> {
        let proxy = LockProxy::builder(&self.0.connection)
            .destination(self.0.owner.as_str())
            .map_err(error)?
            .build()
            .await
            .map_err(error)?;
        proxy.wait_ready(&self.0.scope).await.map_err(error)
    }
    pub async fn release(self) {
        let _ = futures_lite::future::race(
            async {
                let proxy = LockProxy::builder(&self.0.connection)
                    .destination(self.0.owner.as_str())?
                    .build()
                    .await?;
                proxy.set_sleep_pending(&self.0.scope, false).await
            },
            async {
                async_io::Timer::after(Duration::from_secs(1)).await;
                Err(zbus::Error::Failure("Sleep hold release timed out".into()))
            },
        )
        .await;
    }
}
pub async fn request_for_sleep(program: PathBuf) -> Result<SleepLease, ServiceError> {
    futures_lite::future::race(acquire(program, true), async {
        async_io::Timer::after(Duration::from_secs(5)).await;
        Err(ServiceError(gettext(
            "The session lock did not become ready.",
        )))
    })
    .await
    .map(SleepLease)
}
async fn acquire(program: PathBuf, hold: bool) -> Result<Locked, ServiceError> {
    let scope = asena_core::session::current_scope()
        .map_err(|_| ServiceError(gettext("The current session could not be identified.")))?;
    let connection = zbus::Connection::session().await.map_err(error)?;
    let bus = zbus::fdo::DBusProxy::new(&connection)
        .await
        .map_err(error)?;
    // Subscribe before checking the owner, including the name-acquisition race.
    let mut owners = bus.receive_name_owner_changed().await.map_err(error)?;
    let name = zbus::names::BusName::try_from(LOCK_ID).map_err(error)?;
    if !bus.name_has_owner(name.clone()).await.map_err(error)? {
        let child = gio::spawn_blocking(move || {
            if !trusted_policy() {
                return Err(std::io::Error::new(
                    std::io::ErrorKind::PermissionDenied,
                    gettext("Authentication policy is unavailable."),
                ));
            }
            Command::new(program)
                .stdin(Stdio::null())
                .stdout(Stdio::null())
                .spawn()
        })
        .await
        .map_err(|_| ServiceError(gettext("The session could not be locked.")))?
        .map_err(error)?;
        // One blocking reaper per newly started locker; no wait on the UI thread.
        gio::spawn_blocking(move || {
            let mut child = child;
            child.wait()
        });
        while !bus.name_has_owner(name.clone()).await.map_err(error)? {
            let Some(event) = owners.next().await else {
                return Err(ServiceError(gettext("The session could not be locked.")));
            };
            let args = event.args().map_err(error)?;
            if args.name().as_str() == LOCK_ID && args.new_owner().is_some() {
                break;
            }
        }
    }
    let owner = bus.get_name_owner(name).await.map_err(error)?;
    let proxy = LockProxy::builder(&connection)
        .destination(owner.clone())
        .map_err(error)?
        .build()
        .await
        .map_err(error)?;
    if hold {
        proxy.set_sleep_pending(&scope, true).await.map_err(error)?;
    }
    proxy.wait_ready(&scope).await.map_err(error)?;
    let _ = proxy.focus().await;
    Ok(Locked {
        connection,
        owner: owner.to_string(),
        scope,
    })
}
fn error(error: impl std::fmt::Display) -> ServiceError {
    ServiceError(error.to_string())
}

/// Worker-only selection. Development previews stay disabled except VM fixtures.
pub fn program() -> Option<PathBuf> {
    let nested = std::env::var_os("ASENA_DEV").is_some();
    let path = if nested {
        if std::env::var("ASENA_LOCK_TEST_VM").as_deref() != Ok("1") {
            return None;
        }
        PathBuf::from(std::env::var_os("ASENA_SOURCE_DIR")?).join("target/debug/asena-lock")
    } else {
        Path::new("/usr/bin/asena-lock").to_path_buf()
    };
    (path.is_file() && trusted_policy()).then_some(path)
}
