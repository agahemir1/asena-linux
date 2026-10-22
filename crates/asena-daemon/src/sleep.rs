//! logind owns lid policy. This task owns only a finite sleep-delay inhibitor.
use crate::power::Command;
use asena_core::i18n::gettext;
use futures_lite::{future::race, StreamExt};
use std::time::Duration;

pub async fn manager(connection: &zbus::Connection) -> zbus::Result<zbus::Proxy<'_>> {
    zbus::Proxy::new(
        connection,
        "org.freedesktop.login1",
        "/org/freedesktop/login1",
        "org.freedesktop.login1.Manager",
    )
    .await
}
async fn inhibit(proxy: &zbus::Proxy<'_>) -> zbus::Result<zbus::zvariant::OwnedFd> {
    proxy
        .call(
            "Inhibit",
            &(
                "sleep",
                "Asena",
                gettext("Lock the session before sleeping."),
                "delay",
            ),
        )
        .await
}
async fn watch(sender: &async_channel::Sender<Command>) -> anyhow::Result<()> {
    let connection = zbus::Connection::system().await?;
    let proxy = manager(&connection).await?;
    let mut changes = proxy.receive_signal("PrepareForSleep").await?;
    let bus = zbus::fdo::DBusProxy::new(&connection).await?;
    let mut owners = bus.receive_name_owner_changed().await?;
    let budget = proxy.get_property::<u64>("InhibitDelayMaxUSec").await?;
    let mut guard = Some(inhibit(&proxy).await?);
    let mut initial = proxy
        .get_property::<bool>("PreparingForSleep")
        .await?
        .then_some(true);
    // No synthetic resume at startup: a retained screen-off lease may be valid.
    loop {
        let pending = if let Some(pending) = initial.take() {
            pending
        } else {
            let message = race(async { changes.next().await }, async {
                while let Some(event) = owners.next().await {
                    if event
                        .args()
                        .is_ok_and(|args| args.name().as_str() == "org.freedesktop.login1")
                    {
                        break;
                    }
                }
                None
            })
            .await;
            let Some(message) = message else {
                break;
            };
            message.body().deserialize::<(bool,)>()?.0
        };
        if pending {
            let (reply, response) = async_channel::bounded(1);
            let deadline = Duration::from_micros(budget.saturating_sub(250_000));
            let prepared = race(
                async {
                    sender.send(Command::Prepare(reply)).await.ok()?;
                    response.recv().await.ok()
                },
                async {
                    glib::timeout_future(deadline).await;
                    None
                },
            )
            .await;
            if prepared != Some(true) {
                tracing::error!(
                    budget_us = budget,
                    "Sleep lock was not confirmed before the finite logind deadline"
                );
                // On failure retain the FD through the remaining finite deadline.
                glib::timeout_future(Duration::from_millis(250)).await;
            }
            guard.take();
            tracing::info!(
                ready = prepared == Some(true),
                "Sleep delay inhibitor released"
            );
        } else {
            guard = Some(inhibit(&proxy).await?);
            sender.send(Command::Resume).await?;
            tracing::info!("Sleep resumed; delay inhibitor reacquired");
        }
    }
    anyhow::bail!("logind sleep event stream closed")
}
pub async fn run(sender: async_channel::Sender<Command>) {
    loop {
        if let Err(error) = watch(&sender).await {
            tracing::warn!(%error,"Sleep coordination disconnected");
        }
        if sender.is_closed() {
            return;
        }
        glib::timeout_future(Duration::from_secs(3)).await;
    }
}
pub async fn suspend() -> Result<(), String> {
    let connection = zbus::Connection::system()
        .await
        .map_err(|error| error.to_string())?;
    let proxy = manager(&connection)
        .await
        .map_err(|error| error.to_string())?;
    let available: String = proxy
        .call("CanSuspend", &())
        .await
        .map_err(|error| error.to_string())?;
    if available != "yes" {
        return Err(gettext("Suspend is unavailable in this session."));
    }
    // Never open an authorization dialog behind the already locked session.
    proxy
        .call::<_, _, ()>("Suspend", &(false,))
        .await
        .map_err(|error| error.to_string())
}
pub async fn awake() -> bool {
    let Ok(connection) = zbus::Connection::system().await else {
        return false;
    };
    let Ok(proxy) = manager(&connection).await else {
        return false;
    };
    proxy
        .get_property::<bool>("PreparingForSleep")
        .await
        .is_ok_and(|pending| !pending)
}
