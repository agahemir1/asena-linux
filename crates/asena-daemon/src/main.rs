use gio::prelude::*;
use std::{
    cell::Cell,
    rc::Rc,
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc,
    },
};
mod appearance;
mod child;
mod displays;
mod idle;
mod locale;
mod night_light;
mod portal;
mod portal_inhibit;
mod power;
mod sleep;
mod wayfire;
struct DaemonBus {
    status: Arc<night_light::Status>,
    locker: Option<std::path::PathBuf>,
    locking: Arc<AtomicBool>,
    scope: Option<String>,
    power: async_channel::Sender<power::Command>,
    displays: async_channel::Sender<displays::Command>,
    display_status: Arc<displays::Status>,
}
impl DaemonBus {
    fn same_session(&self, scope: &str) -> zbus::fdo::Result<()> {
        if self.scope.as_deref() == Some(scope) {
            Ok(())
        } else {
            Err(zbus::fdo::Error::AccessDenied(asena_core::i18n::gettext(
                "This request comes from another session.",
            )))
        }
    }
    fn send_display(&self, command: displays::Command) -> zbus::fdo::Result<()> {
        self.displays.try_send(command).map_err(|_| {
            zbus::fdo::Error::LimitsExceeded(asena_core::i18n::gettext(
                "Too many display requests.",
            ))
        })
    }
}
async fn display_reply<T>(
    response: async_channel::Receiver<Result<T, String>>,
) -> zbus::fdo::Result<T> {
    use asena_core::i18n::gettext;
    futures_lite::future::race(
        async {
            response
                .recv()
                .await
                .map_err(|_| {
                    zbus::fdo::Error::Failed(gettext("Display settings are unavailable."))
                })?
                .map_err(zbus::fdo::Error::Failed)
        },
        async {
            async_io::Timer::after(std::time::Duration::from_secs(30)).await;
            Err(zbus::fdo::Error::Failed(gettext(
                "The display request timed out.",
            )))
        },
    )
    .await
}
struct LockAttempt(Arc<AtomicBool>);
impl Drop for LockAttempt {
    fn drop(&mut self) {
        self.0.store(false, Ordering::SeqCst);
    }
}
#[zbus::interface(name = "org.asenalinux.Daemon")]
impl DaemonBus {
    async fn idle(&self, scope: &str, action: &str) -> zbus::fdo::Result<()> {
        use asena_core::i18n::gettext;
        if self.scope.as_deref() != Some(scope) {
            return Err(zbus::fdo::Error::AccessDenied(gettext(
                "Run this command in the session you want to lock.",
            )));
        }
        if !matches!(
            action,
            "dim" | "restore" | "lock" | "off" | "on" | "suspend"
        ) {
            return Err(zbus::fdo::Error::InvalidArgs(gettext(
                "Unknown idle action.",
            )));
        }
        let (reply, response) = async_channel::bounded(1);
        self.power
            .try_send(power::Command::Action(action.into(), reply))
            .map_err(|_| zbus::fdo::Error::LimitsExceeded(gettext("Too many power requests.")))?;
        futures_lite::future::race(
            async {
                response
                    .recv()
                    .await
                    .map_err(|_| {
                        zbus::fdo::Error::Failed(gettext("Power management is unavailable."))
                    })?
                    .map_err(zbus::fdo::Error::Failed)
            },
            async {
                async_io::Timer::after(std::time::Duration::from_secs(60)).await;
                Err(zbus::fdo::Error::Failed(gettext(
                    "The power request timed out.",
                )))
            },
        )
        .await
    }
    #[zbus(property)]
    fn lock_available(&self) -> bool {
        self.locker.is_some() && self.scope.is_some()
    }
    async fn lock(&self, scope: &str) -> zbus::fdo::Result<()> {
        if self.scope.as_deref() != Some(scope) {
            return Err(zbus::fdo::Error::AccessDenied(asena_core::i18n::gettext(
                "Run this command in the session you want to lock.",
            )));
        }
        let Some(program) = self.locker.clone() else {
            return Err(zbus::fdo::Error::NotSupported(asena_core::i18n::gettext(
                "A verified session locker is not available.",
            )));
        };
        if self
            .locking
            .compare_exchange(false, true, Ordering::SeqCst, Ordering::SeqCst)
            .is_err()
        {
            return Err(zbus::fdo::Error::Failed(asena_core::i18n::gettext(
                "A lock request is already in progress.",
            )));
        }
        let _attempt = LockAttempt(self.locking.clone());
        asena_services::lock_client::request(program)
            .await
            .map_err(|error| zbus::fdo::Error::Failed(error.to_string()))
    }
    #[zbus(property)]
    fn night_light_available(&self) -> bool {
        self.status.available.load(Ordering::SeqCst)
    }
    #[zbus(property)]
    fn night_light_active(&self) -> bool {
        self.status.active.load(Ordering::SeqCst)
    }
    #[zbus(property)]
    fn night_light_error(&self) -> String {
        self.status.error()
    }
    #[zbus(signal)]
    async fn night_light_changed(
        emitter: &zbus::object_server::SignalEmitter<'_>,
    ) -> zbus::Result<()>;
    /// JSON `asena_core::displays::Layout`.
    fn display_layout(&self) -> String {
        self.display_status.json()
    }
    /// Applies a JSON list of `asena_core::displays::Config` and returns the
    /// seconds left to confirm it, or zero when nothing changed.
    async fn apply_displays(
        &self,
        scope: &str,
        layout: &str,
        #[zbus(header)] header: zbus::message::Header<'_>,
    ) -> zbus::fdo::Result<u32> {
        use asena_core::i18n::gettext;
        self.same_session(scope)?;
        let configs = asena_core::displays::configs_from_json(layout).ok_or_else(|| {
            zbus::fdo::Error::InvalidArgs(gettext("The display layout is invalid."))
        })?;
        // The change is reverted as soon as this caller leaves the bus.
        let caller = header
            .sender()
            .map(|sender| sender.to_string())
            .ok_or_else(|| {
                zbus::fdo::Error::AccessDenied(gettext("The display request has no owner."))
            })?;
        let (reply, response) = async_channel::bounded(1);
        self.send_display(displays::Command::Apply {
            configs,
            caller,
            reply,
        })?;
        display_reply(response).await
    }
    async fn confirm_displays(&self, scope: &str) -> zbus::fdo::Result<()> {
        self.same_session(scope)?;
        let (reply, response) = async_channel::bounded(1);
        self.send_display(displays::Command::Confirm(reply))?;
        display_reply(response).await
    }
    async fn revert_displays(&self, scope: &str) -> zbus::fdo::Result<()> {
        self.same_session(scope)?;
        let (reply, response) = async_channel::bounded(1);
        self.send_display(displays::Command::Revert(reply))?;
        display_reply(response).await
    }
    #[zbus(signal)]
    async fn displays_changed(emitter: &zbus::object_server::SignalEmitter<'_>)
        -> zbus::Result<()>;
}
fn main() -> anyhow::Result<()> {
    asena_core::logging::init("asena-daemon").map_err(|e| anyhow::anyhow!("{e}"))?;
    asena_core::i18n::init().map_err(|e| anyhow::anyhow!("{e}"))?;
    let desktop = asena_core::settings::load("org.asenalinux.desktop")?;
    let preferences = asena_core::settings::load("org.asenalinux.night-light")?;
    let power_preferences = asena_core::settings::load("org.asenalinux.power")?;
    let (power_sender, power_receiver) = async_channel::bounded(16);
    let power_changes = power_sender.clone();
    power_preferences.connect_changed(None, move |_, _| {
        let _ = power_changes.try_send(power::Command::Apply);
    });
    let _ = power_sender.try_send(power::Command::Apply);
    let power_commands = power_sender.clone();
    let (display_sender, display_receiver) = async_channel::bounded(8);
    let display_status = Arc::new(displays::Status::default());
    let (settings_portal, portal_state) = portal::create(&desktop);
    let main_loop = glib::MainLoop::new(None, false);
    let success = Rc::new(Cell::new(false));
    let result = success.clone();
    let finished = main_loop.clone();
    let (status_sender, receiver) = async_channel::unbounded();
    let changes = status_sender.clone();
    preferences.connect_changed(None, move |settings, _| {
        let _ = changes.try_send(night_light::Command::Apply(
            settings.boolean("enabled"),
            settings.uint("temperature"),
        ));
    });
    let _ = status_sender.try_send(night_light::Command::Apply(
        preferences.boolean("enabled"),
        preferences.uint("temperature"),
    ));
    let task = glib::spawn_future_local(async move {
        let status = Arc::new(night_light::Status::default());
        let locker = gio::spawn_blocking(|| {
            let program = asena_services::lock_client::program()?;
            asena_wayland::capabilities::globals()
                .ok()?
                .contains_key("ext_session_lock_manager_v1")
                .then_some(program)
        })
        .await
        .unwrap_or(None);
        let inhibit = portal_inhibit::Inhibit::new(power_commands.clone());
        let connection = match async {
            let connection = zbus::connection::Builder::session()?
                .serve_at(
                    asena_core::dbus::DAEMON_PATH,
                    DaemonBus {
                        status: status.clone(),
                        locker: locker.clone(),
                        locking: Arc::new(AtomicBool::new(false)),
                        scope: asena_core::session::current_scope().ok(),
                        power: power_commands.clone(),
                        displays: display_sender.clone(),
                        display_status: display_status.clone(),
                    },
                )?
                .serve_at(portal::PATH, settings_portal)?
                .serve_at(portal::PATH, inhibit.clone())?
                .serve_at(portal::PATH, portal_inhibit::Background)?
                .build()
                .await?;
            asena_core::dbus::claim_name(&connection, asena_core::dbus::DAEMON_ID).await?;
            Ok::<_, zbus::Error>(connection)
        }
        .await
        {
            Ok(connection) => connection,
            Err(error) => {
                tracing::error!(%error,"Could not own the daemon bus name");
                finished.quit();
                return;
            }
        };
        appearance::start(&desktop);
        portal::start(&connection, &desktop, portal_state).await;
        glib::spawn_future_local(inhibit.watcher(connection.clone()));
        // Runs until the process exits; a pending change ends with the session.
        glib::spawn_future_local(displays::run(
            display_receiver,
            display_sender,
            display_status,
            connection.clone(),
        ));
        // Bound here so the settings objects live as long as the daemon task.
        let input = asena_core::settings::load("org.asenalinux.input");
        if let Err(error) = &input {
            tracing::warn!(%error, "Input settings are unavailable");
        }
        wayfire::watch(input.as_ref().ok(), &desktop);
        let session = asena_core::settings::load("org.asenalinux.session");
        match &session {
            Ok(session) => locale::watch(session),
            Err(error) => tracing::warn!(%error, "Session settings are unavailable"),
        }
        result.set(true);
        futures_lite::future::zip(
            night_light::run(receiver, status, connection, preferences),
            power::run(power_receiver, power_commands, power_preferences, locker),
        )
        .await;
        finished.quit();
    });
    let mut signals = Vec::new();
    // Linux SIGINT/SIGTERM are routed through GLib so children are reaped first.
    for signal in [2, 15] {
        let sender = status_sender.clone();
        let power_stop = power_sender.clone();
        signals.push(glib::unix_signal_add_local(signal, move || {
            let _ = sender.try_send(night_light::Command::Stop);
            let stopping = power_stop.clone();
            glib::spawn_future_local(async move {
                let _ = stopping.send(power::Command::Stop).await;
            });
            glib::ControlFlow::Continue
        }));
    }
    main_loop.run();
    for signal in signals {
        signal.remove();
    }
    task.abort();
    anyhow::ensure!(success.get(), "Daemon startup failed");
    Ok(())
}
