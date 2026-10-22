//! ObjectManager initial state, delta signals, removal and owner loss.
use asena_services::{
    object_model::ObjectModel,
    property_model::{Bus, Endpoint},
};
use std::{cell::Cell, rc::Rc, time::Duration};
const NAME: &str = "org.asenalinux.TestObjects";
const ROOT: &str = "/org/asenalinux/TestObjects";
struct Device {
    enabled: bool,
}
#[zbus::interface(name = "org.asenalinux.TestDevice")]
impl Device {
    #[zbus(property)]
    fn enabled(&self) -> bool {
        self.enabled
    }
    #[zbus(property)]
    fn set_enabled(&mut self, value: bool) {
        self.enabled = value;
    }
}
async fn until(condition: impl Fn() -> bool) -> anyhow::Result<()> {
    for _ in 0..100 {
        if condition() {
            return Ok(());
        }
        glib::timeout_future(Duration::from_millis(20)).await;
    }
    anyhow::bail!("Object cache condition timed out")
}
fn enabled(model: &ObjectModel, path: &str) -> Option<bool> {
    model.with_objects(|objects| {
        objects
            .iter()
            .find_map(|(key, interfaces)| (key.as_str() == path).then_some(interfaces))
            .and_then(|interfaces| interfaces.get("org.asenalinux.TestDevice"))
            .and_then(|properties| properties.get("Enabled"))
            .and_then(|v| bool::try_from(v).ok())
    })
}
async fn probe() -> anyhow::Result<()> {
    let model = ObjectModel::new(Endpoint {
        bus: Bus::Session,
        name: NAME.into(),
        path: ROOT.into(),
        interface: "org.freedesktop.DBus.ObjectManager".into(),
    });
    model.start();
    let connection = zbus::connection::Builder::session()?
        .serve_at(ROOT, zbus::fdo::ObjectManager)?
        .serve_at(format!("{ROOT}/first"), Device { enabled: true })?
        .build()
        .await?;
    asena_core::dbus::claim_name(&connection, NAME).await?;
    let first = format!("{ROOT}/first");
    let second = format!("{ROOT}/second");
    until(|| model.available() && enabled(&model, &first) == Some(true)).await?;
    model
        .set_bool(&first, "org.asenalinux.TestDevice", "Enabled", false)
        .await?;
    until(|| enabled(&model, &first) == Some(false)).await?;
    connection
        .object_server()
        .at(second.clone(), Device { enabled: true })
        .await?;
    until(|| enabled(&model, &second) == Some(true)).await?;
    connection
        .object_server()
        .remove::<Device, _>(second.as_str())
        .await?;
    until(|| enabled(&model, &second).is_none()).await?;
    connection.release_name(NAME).await?;
    until(|| !model.available()).await?;
    anyhow::ensure!(
        model.with_objects(|objects| objects.is_empty()),
        "Stale objects survived owner loss"
    );
    println!(
        "ObjectManager initial state, property delta, add/remove and owner-loss clearing passed."
    );
    Ok(())
}
fn main() -> anyhow::Result<()> {
    let loop_ = glib::MainLoop::new(None, false);
    let finished = loop_.clone();
    let success = Rc::new(Cell::new(false));
    let result = success.clone();
    glib::spawn_future_local(async move {
        let outcome = futures_lite::future::race(probe(), async {
            glib::timeout_future(Duration::from_secs(15)).await;
            Err(anyhow::anyhow!("Object probe timed out"))
        })
        .await;
        match outcome {
            Ok(()) => result.set(true),
            Err(error) => eprintln!("{error:#}"),
        };
        finished.quit();
    });
    loop_.run();
    anyhow::ensure!(success.get(), "Object probe failed");
    Ok(())
}
