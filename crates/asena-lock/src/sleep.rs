//! Clear abandoned holds only when logind says the machine is awake.
use crate::bus::Readiness;
use futures_lite::StreamExt;
pub async fn watch(
    readiness: Readiness,
    session: zbus::Connection,
    update: impl Fn() + 'static,
) -> zbus::Result<()> {
    loop {
        if let Err(error) = watch_once(readiness.clone(), session.clone(), &update).await {
            tracing::warn!(%error,"Sleep monitoring disconnected; retaining the lock");
        }
        glib::timeout_future(std::time::Duration::from_secs(3)).await;
    }
}
async fn watch_once(
    readiness: Readiness,
    session: zbus::Connection,
    update: &impl Fn(),
) -> zbus::Result<()> {
    let system = zbus::Connection::system().await?;
    let manager = zbus::Proxy::new(
        &system,
        "org.freedesktop.login1",
        "/org/freedesktop/login1",
        "org.freedesktop.login1.Manager",
    )
    .await?;
    let mut sleep = manager.receive_signal("PrepareForSleep").await?;
    let root_bus = zbus::fdo::DBusProxy::new(&system).await?;
    let mut root_owners = root_bus.receive_name_owner_changed().await?;
    let bus = zbus::fdo::DBusProxy::new(&session).await?;
    let mut owners = bus.receive_name_owner_changed().await?;
    if let Some(owner) = readiness.owner() {
        if let Ok(name) = zbus::names::BusName::try_from(owner.as_str()) {
            if !bus.name_has_owner(name).await?
                && !manager.get_property::<bool>("PreparingForSleep").await?
            {
                readiness.owner_gone();
                update();
            }
        }
    }
    readiness.system_sleep(manager.get_property::<bool>("PreparingForSleep").await?);
    update();
    enum Event<T> {
        Sleep(Option<zbus::Message>),
        Owner(Option<T>),
        Root,
    }
    loop {
        let event = futures_lite::future::race(
            futures_lite::future::race(async { Event::Sleep(sleep.next().await) }, async {
                Event::Owner(owners.next().await)
            }),
            async {
                while let Some(event) = root_owners.next().await {
                    if event
                        .args()
                        .is_ok_and(|args| args.name().as_str() == "org.freedesktop.login1")
                    {
                        break;
                    }
                }
                Event::Root
            },
        )
        .await;
        match event {
            Event::Sleep(Some(message)) => {
                let (pending,) = message.body().deserialize::<(bool,)>()?;
                readiness.system_sleep(pending);
                if !pending {
                    if let Some(owner) = readiness.owner() {
                        if let Ok(name) = zbus::names::BusName::try_from(owner.as_str()) {
                            if !bus.name_has_owner(name).await? {
                                readiness.owner_gone();
                            }
                        }
                    }
                }
                update();
            }
            Event::Owner(Some(change)) => {
                let args = change.args()?;
                if args.new_owner().is_none()
                    && readiness.owner().as_deref() == Some(args.name().as_str())
                    && manager
                        .get_property::<bool>("PreparingForSleep")
                        .await
                        .is_ok_and(|pending| !pending)
                {
                    readiness.owner_gone();
                    update();
                }
            }
            _ => return Ok(()),
        }
    }
}
