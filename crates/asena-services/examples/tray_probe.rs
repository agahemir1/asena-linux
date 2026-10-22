//! Two independent SNI clients and a dbusmenu; optional external-watcher mode.
use asena_core::dbusmenu::Layout;
use asena_services::tray::TrayModel;
use std::{
    cell::Cell,
    collections::HashMap,
    rc::Rc,
    sync::{
        atomic::{AtomicU32, Ordering},
        Arc,
    },
    time::Duration,
};
use zbus::zvariant::{OwnedObjectPath, OwnedValue, Str, Value};
struct Item {
    actions: Arc<AtomicU32>,
}
#[zbus::interface(name = "org.kde.StatusNotifierItem")]
impl Item {
    fn activate(&self, _x: i32, _y: i32) {
        self.actions.fetch_add(1, Ordering::Relaxed);
    }
    fn secondary_activate(&self, _x: i32, _y: i32) {
        self.actions.fetch_add(2, Ordering::Relaxed);
    }
    fn context_menu(&self, _x: i32, _y: i32) {
        self.actions.fetch_add(4, Ordering::Relaxed);
    }
    #[zbus(property)]
    fn title(&self) -> &str {
        "Asena tray test"
    }
    #[zbus(property)]
    fn status(&self) -> &str {
        "Active"
    }
    #[zbus(property)]
    fn icon_name(&self) -> &str {
        "utilities-terminal-symbolic"
    }
    #[zbus(property)]
    fn menu(&self) -> zbus::fdo::Result<OwnedObjectPath> {
        OwnedObjectPath::try_from("/Menu").map_err(|e| zbus::fdo::Error::InvalidArgs(e.to_string()))
    }
}
struct Menu {
    clicked: Arc<AtomicU32>,
}
#[zbus::interface(name = "com.canonical.dbusmenu")]
impl Menu {
    fn about_to_show(&self, _id: i32) -> bool {
        false
    }
    fn get_layout(
        &self,
        _parent: i32,
        _depth: i32,
        _properties: Vec<String>,
    ) -> zbus::fdo::Result<(u32, Layout)> {
        let child = (
            17_i32,
            HashMap::<String, OwnedValue>::from([(
                "label".into(),
                OwnedValue::from(Str::from("Open")),
            )]),
            Vec::<OwnedValue>::new(),
        );
        let child = OwnedValue::try_from(Value::from(child))
            .map_err(|e| zbus::fdo::Error::InvalidArgs(e.to_string()))?;
        Ok((1, (0, HashMap::new(), vec![child])))
    }
    fn event(
        &self,
        id: i32,
        event: &str,
        _data: OwnedValue,
        _timestamp: u32,
    ) -> zbus::fdo::Result<()> {
        if event != "clicked" {
            return Err(zbus::fdo::Error::InvalidArgs("Unexpected event".into()));
        }
        self.clicked.store(id as u32, Ordering::Relaxed);
        Ok(())
    }
}
struct Existing {
    items: Vec<String>,
    hosts: Arc<AtomicU32>,
}
#[zbus::interface(name = "org.kde.StatusNotifierWatcher")]
impl Existing {
    fn register_status_notifier_host(&self, _name: &str) {
        self.hosts.fetch_add(1, Ordering::Relaxed);
    }
    #[zbus(property)]
    fn registered_status_notifier_items(&self) -> Vec<String> {
        self.items.clone()
    }
    #[zbus(property)]
    fn is_status_notifier_host_registered(&self) -> bool {
        self.hosts.load(Ordering::Relaxed) > 0
    }
    #[zbus(property)]
    fn protocol_version(&self) -> i32 {
        0
    }
}
async fn client(
    actions: Arc<AtomicU32>,
    clicked: Arc<AtomicU32>,
) -> zbus::Result<zbus::Connection> {
    zbus::connection::Builder::session()?
        .serve_at("/StatusNotifierItem", Item { actions })?
        .serve_at("/Menu", Menu { clicked })?
        .build()
        .await
}
async fn until(condition: impl Fn() -> bool) -> anyhow::Result<()> {
    for _ in 0..200 {
        if condition() {
            return Ok(());
        }
        glib::timeout_future(Duration::from_millis(20)).await;
    }
    anyhow::bail!("Tray condition timed out")
}
async fn probe() -> anyhow::Result<()> {
    let actions = Arc::new(AtomicU32::new(0));
    let clicked = Arc::new(AtomicU32::new(0));
    let first = client(actions.clone(), clicked.clone()).await?;
    let collision = std::env::args().any(|arg| arg == "--collision");
    let hosts = Arc::new(AtomicU32::new(0));
    let existing = if collision {
        let id = format!(
            "{}/StatusNotifierItem",
            first
                .unique_name()
                .ok_or_else(|| anyhow::anyhow!("Missing unique name"))?
        );
        let connection = zbus::connection::Builder::session()?
            .serve_at(
                "/StatusNotifierWatcher",
                Existing {
                    items: vec![id],
                    hosts: hosts.clone(),
                },
            )?
            .build()
            .await?;
        connection
            .request_name_with_flags(
                "org.kde.StatusNotifierWatcher",
                zbus::fdo::RequestNameFlags::AllowReplacement
                    | zbus::fdo::RequestNameFlags::DoNotQueue,
            )
            .await?;
        Some(connection)
    } else {
        None
    };
    let model = TrayModel::default();
    model.start();
    until(|| model.available()).await?;
    let second = if collision {
        until(|| hosts.load(Ordering::Relaxed) > 0 && model.items().len() == 1).await?;
        let connection = existing
            .as_ref()
            .ok_or_else(|| anyhow::anyhow!("No existing watcher"))?;
        let bus = zbus::fdo::DBusProxy::new(connection).await?;
        let owner = bus
            .get_name_owner(zbus::names::BusName::try_from(
                "org.kde.StatusNotifierWatcher",
            )?)
            .await?;
        anyhow::ensure!(
            Some(owner.as_str()) == connection.unique_name().map(|v| v.as_str()),
            "Existing watcher was replaced"
        );
        None
    } else {
        let proxy = zbus::Proxy::new(
            &first,
            "org.kde.StatusNotifierWatcher",
            "/StatusNotifierWatcher",
            "org.kde.StatusNotifierWatcher",
        )
        .await?;
        proxy
            .call::<_, _, ()>("RegisterStatusNotifierItem", &("/StatusNotifierItem",))
            .await?;
        let second = client(Arc::new(AtomicU32::new(0)), Arc::new(AtomicU32::new(0))).await?;
        let proxy = zbus::Proxy::new(
            &second,
            "org.kde.StatusNotifierWatcher",
            "/StatusNotifierWatcher",
            "org.kde.StatusNotifierWatcher",
        )
        .await?;
        proxy
            .call::<_, _, ()>("RegisterStatusNotifierItem", &("/StatusNotifierItem",))
            .await?;
        until(|| model.items().len() == 2).await?;
        Some(second)
    };
    let owner = first
        .unique_name()
        .ok_or_else(|| anyhow::anyhow!("Missing owner"))?
        .as_str();
    let item = model
        .items()
        .into_iter()
        .find(|item| item.owner == owner)
        .ok_or_else(|| anyhow::anyhow!("Client not found"))?;
    model.invoke(&item.id, "Activate", 1, 2).await?;
    model.invoke(&item.id, "SecondaryActivate", 1, 2).await?;
    anyhow::ensure!(
        actions.load(Ordering::Relaxed) == 3,
        "Actions were not delivered"
    );
    let menu = model.menu(&item.id).await?;
    anyhow::ensure!(
        menu.children.len() == 1 && menu.children[0].label == "Open",
        "Menu layout mismatch"
    );
    model.menu_event(&item.id, 17, 0).await?;
    anyhow::ensure!(
        clicked.load(Ordering::Relaxed) == 17,
        "Menu event not delivered"
    );
    first.close().await?;
    until(|| model.items().len() == usize::from(second.is_some())).await?;
    println!("SNI clients, activation, secondary activation, dbusmenu and removal passed; external watcher preserved: {collision}.");
    Ok(())
}
fn main() -> anyhow::Result<()> {
    let loop_ = glib::MainLoop::new(None, false);
    let finished = loop_.clone();
    let success = Rc::new(Cell::new(false));
    let result = success.clone();
    glib::spawn_future_local(async move {
        let outcome = futures_lite::future::race(probe(), async {
            glib::timeout_future(Duration::from_secs(20)).await;
            Err(anyhow::anyhow!("Tray probe timed out"))
        })
        .await;
        match outcome {
            Ok(()) => result.set(true),
            Err(error) => eprintln!("{error:#}"),
        };
        finished.quit();
    });
    loop_.run();
    anyhow::ensure!(success.get(), "Tray probe failed");
    Ok(())
}
