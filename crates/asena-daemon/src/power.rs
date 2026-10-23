//! Serializes idle effects; no GTK objects or blocking native IO on this thread.
use crate::{child::Helper, idle, sleep};
use asena_core::{i18n::gettext, power::DimLease};
use asena_services::{
    brightness::BrightnessModel,
    lock_client::{self, SleepLease},
};
use asena_wayland::power::Session;
use gio::prelude::*;
use std::{path::PathBuf, time::Duration};
pub type Reply = async_channel::Sender<Result<(), String>>;
pub enum Command {
    Apply,
    Action(String, Reply),
    Prepare(async_channel::Sender<bool>),
    Resume,
    SleepFinished(Result<(), String>, Reply),
    /// A portal idle inhibition started (true) or the last one ended.
    Inhibit(bool),
    Stop,
}
struct Controller {
    brightness: BrightnessModel,
    dim: Option<(String, DimLease)>,
    output: Option<Session>,
    lease: Option<SleepLease>,
    off: bool,
    preparing: bool,
    manual: bool,
    locker: Option<PathBuf>,
    helper: Option<Helper>,
    enabled: bool,
    permitted: bool,
    inhibited: bool,
}
impl Controller {
    async fn lock_held(&mut self) -> Result<(), String> {
        if let Some(lease) = &self.lease {
            if lease.ready().await.is_ok() {
                return Ok(());
            }
            self.lease.take();
        }
        let program = self
            .locker
            .clone()
            .ok_or_else(|| gettext("A verified session locker is not available."))?;
        self.lease = Some(
            lock_client::request_for_sleep(program)
                .await
                .map_err(|error| error.to_string())?,
        );
        tracing::info!("Session lock ready; authentication held");
        Ok(())
    }
    async fn release(&mut self) {
        if !self.off && !self.preparing && !self.manual {
            if let Some(lease) = self.lease.take() {
                lease.release().await;
            }
        }
    }
    async fn output(&mut self, enabled: bool) -> Result<(), String> {
        let prior = self.output.take();
        let (session, result) = gio::spawn_blocking(move || {
            let mut session = match prior {
                Some(session) => session,
                None => match Session::connect() {
                    Ok(session) => session,
                    Err(error) => return (None, Err(error.to_string())),
                },
            };
            let result = session
                .set_power(enabled)
                .map_err(|error| error.to_string());
            (Some(session), result)
        })
        .await
        .map_err(|_| gettext("Screen power control failed."))?;
        self.output = session;
        result
    }
    async fn restore(&mut self) -> Result<(), String> {
        if let Some((device, lease)) = self.dim.take() {
            self.brightness
                .refresh()
                .await
                .map_err(|error| error.to_string())?;
            if self.brightness.device_name() != device {
                return Ok(());
            }
            if let Some(value) = lease.restore(self.brightness.level()) {
                self.brightness
                    .set_level(value)
                    .await
                    .map_err(|error| error.to_string())?;
            }
        }
        Ok(())
    }
    async fn on(&mut self) -> Result<(), String> {
        if self.off {
            self.output(true).await?;
            self.off = false;
        }
        self.release().await;
        Ok(())
    }
    async fn apply(&mut self, settings: &gio::Settings) -> Result<(), String> {
        self.enabled = false;
        if let Some(helper) = self.helper.take() {
            helper.stop().await;
        }
        self.on().await?;
        self.restore().await?;
        let timeouts = idle::timeouts(settings).map_err(|error| error.to_string())?;
        if !settings.boolean("idle-enabled") || !self.permitted || self.inhibited {
            return Ok(());
        }
        if (timeouts.lock > 0 || timeouts.off > 0 || timeouts.suspend > 0) && self.locker.is_none()
        {
            return Err(gettext("A verified session locker is not available."));
        }
        self.helper = idle::start(timeouts)
            .await
            .map_err(|error| error.to_string())?;
        self.enabled = true;
        Ok(())
    }
    async fn action(&mut self, action: &str) -> Result<(), String> {
        if !self.enabled && !matches!(action, "restore" | "on") {
            return Err(gettext("Idle actions are disabled."));
        }
        match action {
            "dim" => {
                if self.dim.is_none() && self.brightness.available() {
                    self.brightness
                        .refresh()
                        .await
                        .map_err(|error| error.to_string())?;
                    let original = self.brightness.level();
                    let desired = original.min(0.2);
                    let applied = self
                        .brightness
                        .set_level(desired)
                        .await
                        .map_err(|error| error.to_string())?;
                    self.dim = DimLease::new(original, applied)
                        .map(|lease| (self.brightness.device_name(), lease));
                }
                Ok(())
            }
            "restore" => self.restore().await,
            "lock" => lock_client::request(
                self.locker
                    .clone()
                    .ok_or_else(|| gettext("A verified session locker is not available."))?,
            )
            .await
            .map_err(|error| error.to_string()),
            "off" => {
                self.lock_held().await?;
                // Even a partial native failure may switch off one monitor.
                self.off = true;
                if let Err(error) = self.output(false).await {
                    if let Err(restore) = self.on().await {
                        tracing::error!(%restore,"Screen restore failed; authentication remains held");
                    }
                    return Err(error);
                }
                Ok(())
            }
            "on" => self.on().await,
            _ => Err(gettext("Unknown idle action.")),
        }
    }
}
pub async fn run(
    receiver: async_channel::Receiver<Command>,
    sender: async_channel::Sender<Command>,
    settings: gio::Settings,
    locker: Option<PathBuf>,
) {
    let permitted = std::env::var_os("ASENA_DEV").is_none()
        || std::env::var("ASENA_POWER_TEST_VM").as_deref() == Ok("1");
    let brightness = BrightnessModel::default();
    if let Err(error) = brightness.start() {
        tracing::warn!(%error,"Idle backlight unavailable");
    }
    let watcher = (permitted && locker.is_some())
        .then(|| glib::spawn_future_local(sleep::run(sender.clone())));
    let mut state = Controller {
        brightness,
        dim: None,
        output: None,
        lease: None,
        off: false,
        preparing: false,
        manual: false,
        locker,
        helper: None,
        enabled: false,
        permitted,
        inhibited: false,
    };
    while let Ok(command) = receiver.recv().await {
        match command {
            Command::Stop => break,
            Command::Inhibit(inhibited) => {
                state.inhibited = inhibited;
                if let Err(error) = state.apply(&settings).await {
                    tracing::warn!(%error, "Idle configuration rejected");
                }
            }
            Command::Apply => {
                if let Err(error) = state.apply(&settings).await {
                    tracing::warn!(%error,"Idle configuration rejected");
                }
            }
            Command::Action(action, reply) => {
                if action == "suspend" {
                    let prepared = if !state.permitted {
                        Err(gettext("Suspend is unavailable in this session."))
                    } else if state.manual || state.preparing {
                        Err(gettext("A sleep request is already in progress."))
                    } else {
                        state.lock_held().await
                    };
                    if let Err(error) = prepared {
                        let _ = reply.try_send(Err(error));
                        continue;
                    }
                    state.manual = true;
                    let completed = sender.clone();
                    glib::spawn_future_local(async move {
                        let result = sleep::suspend().await;
                        let _ = completed.send(Command::SleepFinished(result, reply)).await;
                    });
                } else {
                    let result = state.action(&action).await;
                    let _ = reply.try_send(result);
                }
            }
            Command::Prepare(reply) => {
                state.preparing = true;
                let result = state.lock_held().await;
                if let Err(error) = &result {
                    tracing::error!(%error,"External sleep could not confirm the session lock");
                }
                let _ = reply.try_send(result.is_ok());
            }
            Command::Resume => {
                state.preparing = false;
                state.manual = false;
                if let Err(error) = state.on().await {
                    tracing::error!(%error,"Screen restore after sleep failed");
                }
                if let Err(error) = state.restore().await {
                    tracing::warn!(%error,"Brightness restore after sleep failed");
                }
            }
            Command::SleepFinished(result, reply) => {
                if sleep::awake().await {
                    state.manual = false;
                    state.preparing = false;
                    state.release().await;
                }
                let _ = reply.try_send(result);
            }
        }
    }
    receiver.close();
    if let Some(helper) = state.helper.take() {
        helper.stop().await;
    }
    if let Err(error) = state.on().await {
        tracing::error!(%error,"Final screen restore failed");
    }
    if let Err(error) = state.restore().await {
        tracing::warn!(%error,"Final brightness restore failed");
    }
    if let Some(watcher) = watcher {
        watcher.abort();
    }
    // Give disconnected-owner cleanup a chance to observe the final lease drop.
    state.lease.take();
    glib::timeout_future(Duration::from_millis(1)).await;
}
