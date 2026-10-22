//! Reconnect and confirmed-property test against a private mock D-Bus service.
use asena_services::property_model::{Bus, Endpoint, PropertyModel};
use std::{cell::Cell, rc::Rc, time::Duration};
const NAME: &str = "org.asenalinux.TestProperties";
struct Device {
    enabled: bool,
}
#[zbus::interface(name = "org.asenalinux.TestProperties")]
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
    anyhow::bail!("Service model condition timed out")
}
async fn server(enabled: bool) -> zbus::Result<zbus::Connection> {
    let connection = zbus::connection::Builder::session()?
        .serve_at("/org/asenalinux/TestProperties", Device { enabled })?
        .build()
        .await?;
    asena_core::dbus::claim_name(&connection, NAME).await?;
    Ok(connection)
}
async fn probe() -> anyhow::Result<()> {
    let model = PropertyModel::new(Endpoint {
        bus: Bus::Session,
        name: NAME.into(),
        path: "/org/asenalinux/TestProperties".into(),
        interface: NAME.into(),
    });
    model.start();
    glib::timeout_future(Duration::from_millis(100)).await;
    anyhow::ensure!(!model.available(), "Missing service was shown as available");
    let first = server(true).await?;
    until(|| model.available() && model.boolean("Enabled") == Some(true)).await?;
    model.set_property("Enabled", false.into()).await?;
    until(|| model.boolean("Enabled") == Some(false)).await?;
    anyhow::ensure!(!model.busy(), "Operation stayed busy");
    first.release_name(NAME).await?;
    until(|| !model.available()).await?;
    anyhow::ensure!(
        model.boolean("Enabled").is_none(),
        "Disconnected state was retained"
    );
    let _second = server(true).await?;
    until(|| model.available() && model.boolean("Enabled") == Some(true)).await?;
    println!("Service absent, arrival, confirmed property update, loss, state clearing and reconnection passed.");
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
            Err(anyhow::anyhow!("Probe timed out"))
        })
        .await;
        match outcome {
            Ok(()) => result.set(true),
            Err(error) => eprintln!("{error:#}"),
        };
        finished.quit();
    });
    loop_.run();
    anyhow::ensure!(success.get(), "Service probe failed");
    Ok(())
}
