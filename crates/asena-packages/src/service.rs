//! `org.asenalinux.Packages` on the system bus.
use crate::{apt, offline, plans, polkit};
use asena_core::packages::{
    Plan, PlanError, ACTION_DOWNLOAD, ACTION_INSTALL, ACTION_REFRESH, ACTION_REMOVE, ACTION_REPAIR,
    ACTION_UPGRADE, PACKAGES_ID, PACKAGES_PATH,
};
use std::{
    sync::{Arc, Mutex},
    time::{Duration, Instant},
};
use zbus::object_server::SignalEmitter;

/// Error names under `org.asenalinux.Packages.Error`; Software translates them.
#[derive(Debug, zbus::DBusError)]
#[zbus(prefix = "org.asenalinux.Packages.Error")]
pub enum Error {
    #[zbus(error)]
    ZBus(zbus::Error),
    NotAuthorized(String),
    InvalidName(String),
    UnknownPackage(String),
    Protected(String),
    NothingToDo(String),
    StalePlan(String),
    Busy(String),
    Failed(String),
}

impl From<apt::Error> for Error {
    fn from(error: apt::Error) -> Self {
        match error {
            apt::Error::Plan(PlanError::InvalidName(name)) => Self::InvalidName(name),
            apt::Error::Plan(PlanError::Unknown(name)) => Self::UnknownPackage(name),
            apt::Error::Plan(PlanError::Protected(name)) => Self::Protected(name),
            apt::Error::Plan(PlanError::Empty) => Self::NothingToDo(String::new()),
            apt::Error::Stale => Self::StalePlan(String::new()),
            apt::Error::Apt(message) => Self::Failed(message),
        }
    }
}

#[derive(Default)]
struct State {
    plans: plans::Plans,
    busy: bool,
    last_activity: Option<Instant>,
}

#[derive(Clone)]
pub struct Service {
    state: Arc<Mutex<State>>,
    progress: apt::Reporter,
}

/// Clears the busy flag even when the operation fails.
struct Busy(Arc<Mutex<State>>);

impl Drop for Busy {
    fn drop(&mut self) {
        if let Ok(mut state) = self.0.lock() {
            state.busy = false;
            state.last_activity = Some(Instant::now());
        }
    }
}

fn sender(header: &zbus::message::Header<'_>) -> Result<String, Error> {
    header
        .sender()
        .map(|name| name.to_string())
        .ok_or_else(|| Error::NotAuthorized(String::new()))
}

impl Service {
    fn lock(&self) -> Result<std::sync::MutexGuard<'_, State>, Error> {
        self.state
            .lock()
            .map_err(|_| Error::Failed("state lock poisoned".into()))
    }

    fn touch(&self) {
        if let Ok(mut state) = self.state.lock() {
            state.last_activity = Some(Instant::now());
        }
    }

    async fn authorize(
        &self,
        connection: &zbus::Connection,
        sender: &str,
        action: &str,
    ) -> Result<(), Error> {
        self.touch();
        if polkit::check(connection, sender, action).await? {
            Ok(())
        } else {
            Err(Error::NotAuthorized(action.into()))
        }
    }

    /// Runs one APT operation at a time on a worker thread.
    async fn run<T: Send + 'static>(
        &self,
        emitter: &SignalEmitter<'_>,
        work: impl FnOnce(apt::Reporter) -> Result<T, apt::Error> + Send + 'static,
    ) -> Result<T, Error> {
        {
            let mut state = self.lock()?;
            if state.busy {
                return Err(Error::Busy(String::new()));
            }
            state.busy = true;
        }
        let guard = Busy(self.state.clone());
        let _ = self.busy_changed(emitter).await;
        let reporter = self.progress.clone();
        let result = gio::spawn_blocking(move || work(reporter))
            .await
            .map_err(|_| Error::Failed("the APT worker stopped".into()))?;
        drop(guard);
        let _ = self.busy_changed(emitter).await;
        Ok(result?)
    }

    /// A caller that left the bus can no longer commit its plans.
    fn forget(&self, owner: &str) {
        if let Ok(mut state) = self.state.lock() {
            state.plans.forget(owner);
        }
    }

    /// True when the service has been idle long enough to exit.
    pub fn idle_for(&self, limit: Duration) -> bool {
        self.state.lock().is_ok_and(|mut state| {
            state.plans.prune(Instant::now());
            !state.busy
                && state.plans.is_empty()
                && state
                    .last_activity
                    .is_none_or(|last| last.elapsed() >= limit)
        })
    }
}

/// Package files waiting for a commit; emptied when the service starts.
const FILES: &str = "/var/cache/asena-packages/files";
/// Larger files are not packages a person installs by hand.
const FILE_LIMIT: u64 = 4 * 1024 * 1024 * 1024;

fn copy_package(source: std::os::fd::OwnedFd) -> std::io::Result<std::path::PathBuf> {
    use std::{
        io::{Read, Seek},
        os::unix::fs::DirBuilderExt,
    };
    let mut source = std::fs::File::from(source);
    let metadata = source.metadata()?;
    if !metadata.is_file() || metadata.len() > FILE_LIMIT {
        return Err(std::io::Error::other(
            "not a regular package file within the size limit",
        ));
    }
    source.rewind()?;
    std::fs::DirBuilder::new()
        .recursive(true)
        .mode(0o700)
        .create(FILES)?;
    let path = std::path::Path::new(FILES).join(format!("{}.deb", glib::uuid_string_random()));
    let mut target = std::fs::File::create_new(&path)?;
    let copied = std::io::copy(&mut source.take(FILE_LIMIT + 1), &mut target)?;
    if copied > FILE_LIMIT {
        let _ = std::fs::remove_file(&path);
        return Err(std::io::Error::other("the file is too large"));
    }
    Ok(path)
}

#[zbus::interface(name = "org.asenalinux.Packages")]
impl Service {
    async fn refresh(
        &self,
        #[zbus(header)] header: zbus::message::Header<'_>,
        #[zbus(connection)] connection: &zbus::Connection,
        #[zbus(signal_emitter)] emitter: SignalEmitter<'_>,
    ) -> Result<(), Error> {
        self.authorize(connection, &sender(&header)?, ACTION_REFRESH)
            .await?;
        self.run(&emitter, |reporter| apt::refresh(&reporter)).await
    }

    async fn plan(
        &self,
        install: Vec<String>,
        remove: Vec<String>,
        #[zbus(header)] header: zbus::message::Header<'_>,
        #[zbus(signal_emitter)] emitter: SignalEmitter<'_>,
    ) -> Result<String, Error> {
        let owner = sender(&header)?;
        self.touch();
        let (install_names, remove_names) = (install.clone(), remove.clone());
        let mut plan = self
            .run(&emitter, move |_| {
                apt::plan(&apt::Request {
                    install: &install_names,
                    remove: &remove_names,
                    files: &[],
                })
            })
            .await?;
        plan.id = glib::uuid_string_random().to_string();
        let now = Instant::now();
        self.lock()?.plans.insert(
            plan.id.clone(),
            plans::Stored {
                owner,
                install,
                remove,
                files: Vec::new(),
                changes: plan.changes.clone(),
                created: now,
            },
            now,
        );
        serde_json::to_string(&plan).map_err(|error| Error::Failed(error.to_string()))
    }

    async fn commit(
        &self,
        plan_id: &str,
        #[zbus(header)] header: zbus::message::Header<'_>,
        #[zbus(connection)] connection: &zbus::Connection,
        #[zbus(signal_emitter)] emitter: SignalEmitter<'_>,
    ) -> Result<(), Error> {
        let owner = sender(&header)?;
        let stored = self
            .lock()?
            .plans
            .take(plan_id, &owner, Instant::now())
            .map_err(|_| Error::StalePlan(String::new()))?;
        let shown = Plan {
            changes: stored.changes.clone(),
            ..Plan::default()
        };
        if shown.needs_removal_authorization() {
            self.authorize(connection, &owner, ACTION_REMOVE).await?;
        }
        if shown
            .changes
            .iter()
            .any(|change| change.action != asena_core::packages::Action::Remove)
        {
            self.authorize(connection, &owner, ACTION_INSTALL).await?;
        }
        let files = stored.files.clone();
        let result = self
            .run(&emitter, move |reporter| {
                apt::commit(
                    &apt::Request {
                        install: &stored.install,
                        remove: &stored.remove,
                        files: &stored.files,
                    },
                    &stored.changes,
                    &reporter,
                )
            })
            .await;
        for file in files {
            let _ = std::fs::remove_file(file);
        }
        result
    }

    /// Plans installing a `.deb` the caller opened. The service copies it into
    /// its own directory, so the caller cannot change it before the commit.
    async fn plan_file(
        &self,
        package: zbus::zvariant::OwnedFd,
        #[zbus(header)] header: zbus::message::Header<'_>,
        #[zbus(connection)] connection: &zbus::Connection,
        #[zbus(signal_emitter)] emitter: SignalEmitter<'_>,
    ) -> Result<String, Error> {
        let owner = sender(&header)?;
        self.authorize(connection, &owner, ACTION_INSTALL).await?;
        let path = gio::spawn_blocking(move || copy_package(std::os::fd::OwnedFd::from(package)))
            .await
            .map_err(|_| Error::Failed("the copy stopped".into()))?
            .map_err(|error| Error::Failed(error.to_string()))?;
        let files = vec![path.clone()];
        let planned = files.clone();
        let mut plan = match self
            .run(&emitter, move |_| {
                apt::plan(&apt::Request {
                    install: &[],
                    remove: &[],
                    files: &planned,
                })
            })
            .await
        {
            Ok(plan) => plan,
            Err(error) => {
                let _ = std::fs::remove_file(&path);
                return Err(error);
            }
        };
        plan.id = glib::uuid_string_random().to_string();
        let now = Instant::now();
        self.lock()?.plans.insert(
            plan.id.clone(),
            plans::Stored {
                owner,
                install: Vec::new(),
                remove: Vec::new(),
                files,
                changes: plan.changes.clone(),
                created: now,
            },
            now,
        );
        serde_json::to_string(&plan).map_err(|error| Error::Failed(error.to_string()))
    }

    async fn upgrades(
        &self,
        #[zbus(signal_emitter)] emitter: SignalEmitter<'_>,
    ) -> Result<String, Error> {
        self.touch();
        let upgrades = self.run(&emitter, |_| apt::upgrades()).await?;
        serde_json::to_string(&upgrades).map_err(|error| Error::Failed(error.to_string()))
    }

    async fn download_upgrades(
        &self,
        #[zbus(header)] header: zbus::message::Header<'_>,
        #[zbus(connection)] connection: &zbus::Connection,
        #[zbus(signal_emitter)] emitter: SignalEmitter<'_>,
    ) -> Result<u32, Error> {
        self.authorize(connection, &sender(&header)?, ACTION_DOWNLOAD)
            .await?;
        self.run(&emitter, |reporter| apt::download_upgrades(&reporter))
            .await
    }

    async fn schedule_upgrade(
        &self,
        #[zbus(header)] header: zbus::message::Header<'_>,
        #[zbus(connection)] connection: &zbus::Connection,
        #[zbus(signal_emitter)] emitter: SignalEmitter<'_>,
    ) -> Result<(), Error> {
        self.authorize(connection, &sender(&header)?, ACTION_UPGRADE)
            .await?;
        // Everything the restart installs is on disk before it is scheduled.
        let ready = self
            .run(&emitter, |reporter| apt::download_upgrades(&reporter))
            .await?;
        if ready == 0 {
            return Err(Error::NothingToDo(String::new()));
        }
        offline::schedule().map_err(|error| Error::Failed(error.to_string()))?;
        let _ = self.upgrade_scheduled_changed(&emitter).await;
        Ok(())
    }

    async fn cancel_upgrade(
        &self,
        #[zbus(header)] header: zbus::message::Header<'_>,
        #[zbus(connection)] connection: &zbus::Connection,
        #[zbus(signal_emitter)] emitter: SignalEmitter<'_>,
    ) -> Result<(), Error> {
        self.authorize(connection, &sender(&header)?, ACTION_UPGRADE)
            .await?;
        offline::cancel().map_err(|error| Error::Failed(error.to_string()))?;
        let _ = self.upgrade_scheduled_changed(&emitter).await;
        Ok(())
    }

    fn last_upgrade(&self) -> String {
        self.touch();
        offline::last_result()
    }

    async fn repair(
        &self,
        #[zbus(header)] header: zbus::message::Header<'_>,
        #[zbus(connection)] connection: &zbus::Connection,
        #[zbus(signal_emitter)] emitter: SignalEmitter<'_>,
    ) -> Result<(), Error> {
        self.authorize(connection, &sender(&header)?, ACTION_REPAIR)
            .await?;
        self.run(&emitter, |reporter| apt::repair(&reporter))
            .await?;
        let _ = self.needs_repair_changed(&emitter).await;
        Ok(())
    }

    #[zbus(property)]
    fn busy(&self) -> bool {
        self.state.lock().is_ok_and(|state| state.busy)
    }

    #[zbus(property)]
    fn upgrade_scheduled(&self) -> bool {
        offline::scheduled()
    }

    #[zbus(property)]
    fn needs_repair(&self) -> bool {
        apt::needs_repair()
    }

    #[zbus(signal)]
    async fn progress(
        emitter: &SignalEmitter<'_>,
        operation: &str,
        percent: f64,
        detail: &str,
    ) -> zbus::Result<()>;
}

/// Serves until the service has been idle for five minutes; D-Bus activation
/// starts it again on the next call.
pub fn run() -> anyhow::Result<()> {
    // Files of plans that expired with the previous run.
    let _ = std::fs::remove_dir_all(FILES);
    let main_loop = glib::MainLoop::new(None, false);
    let (reporter, progress) = async_channel::unbounded::<apt::Progress>();
    let service = Service {
        state: Arc::new(Mutex::new(State::default())),
        progress: reporter,
    };
    let idle = service.clone();
    let departures = service.clone();
    let finished = main_loop.clone();
    let failed = Arc::new(Mutex::new(None::<String>));
    let failure = failed.clone();
    let task = glib::spawn_future_local(async move {
        let connection = match async {
            let connection = zbus::connection::Builder::system()?
                .serve_at(PACKAGES_PATH, service)?
                .build()
                .await?;
            asena_core::dbus::claim_name(&connection, PACKAGES_ID).await?;
            Ok::<_, zbus::Error>(connection)
        }
        .await
        {
            Ok(connection) => connection,
            Err(error) => {
                if let Ok(mut slot) = failure.lock() {
                    *slot = Some(error.to_string());
                }
                finished.quit();
                return;
            }
        };
        let emitter = match SignalEmitter::new(&connection, PACKAGES_PATH) {
            Ok(emitter) => emitter,
            Err(error) => {
                tracing::error!(%error, "Cannot emit progress");
                return;
            }
        };
        let forward = async {
            while let Ok(update) = progress.recv().await {
                let _ =
                    Service::progress(&emitter, update.operation, update.percent, &update.detail)
                        .await;
            }
        };
        let watch = async {
            let Ok(bus) = zbus::fdo::DBusProxy::new(&connection).await else {
                return;
            };
            let Ok(mut changes) = bus.receive_name_owner_changed().await else {
                return;
            };
            use futures_lite::StreamExt;
            while let Some(change) = changes.next().await {
                if let Ok(args) = change.args() {
                    if args.new_owner().is_none() {
                        departures.forget(args.name());
                    }
                }
            }
        };
        futures_lite::future::zip(forward, watch).await;
    });
    let stop = main_loop.clone();
    // Removed by returning Break; the process ends with the loop.
    let _timer = glib::timeout_add_seconds_local(60, move || {
        if idle.idle_for(Duration::from_secs(5 * 60)) {
            stop.quit();
            glib::ControlFlow::Break
        } else {
            glib::ControlFlow::Continue
        }
    });
    main_loop.run();
    task.abort();
    if let Ok(slot) = failed.lock() {
        if let Some(error) = slot.as_ref() {
            anyhow::bail!("Could not serve {PACKAGES_ID}: {error}");
        }
    }
    Ok(())
}
