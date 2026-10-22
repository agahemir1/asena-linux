//! Disposable private-bus protocol/lifecycle regression probe.
use asena_services::notifications::NotificationModel;
use futures_lite::StreamExt;
use std::{cell::Cell, collections::HashMap, rc::Rc};
use zbus::zvariant::OwnedValue;

async fn probe() -> anyhow::Result<()> {
    let connection = zbus::Connection::session().await?;
    let collision = std::env::args().any(|a| a == "--collision");
    if collision {
        connection
            .request_name_with_flags(
                "org.freedesktop.Notifications",
                zbus::fdo::RequestNameFlags::AllowReplacement
                    | zbus::fdo::RequestNameFlags::DoNotQueue,
            )
            .await?;
    }
    let model = NotificationModel::default();
    model.start();
    for _ in 0..100 {
        if model.available() {
            break;
        }
        glib::timeout_future(std::time::Duration::from_millis(20)).await;
    }
    if collision {
        anyhow::ensure!(
            !model.available(),
            "Existing notification server was replaced"
        );
        let bus = zbus::fdo::DBusProxy::new(&connection).await?;
        let name = zbus::names::BusName::try_from("org.freedesktop.Notifications")?;
        anyhow::ensure!(
            bus.get_name_owner(name).await?.as_str()
                == connection.unique_name().map(|s| s.as_str()).unwrap_or(""),
            "Original owner changed"
        );
        println!("Notification name collision preserves the existing owner.");
        return Ok(());
    }
    anyhow::ensure!(model.available(), "Server did not start");
    let proxy = zbus::Proxy::new(
        &connection,
        "org.freedesktop.Notifications",
        "/org/freedesktop/Notifications",
        "org.freedesktop.Notifications",
    )
    .await?;
    let info: (String, String, String, String) = proxy.call("GetServerInformation", &()).await?;
    anyhow::ensure!(info.3 == "1.2", "Wrong spec version");
    let mut closed = proxy.receive_signal("NotificationClosed").await?;
    let mut invoked = proxy.receive_signal("ActionInvoked").await?;
    let notify = |replaces, summary: &str| {
        (
            "Asena probe".to_owned(),
            replaces,
            String::new(),
            summary.to_owned(),
            "<b>kalın</b><script>x</script>".to_owned(),
            vec!["yes".to_owned(), "Evet".to_owned()],
            HashMap::<String, OwnedValue>::new(),
            0_i32,
        )
    };
    let id: u32 = proxy.call("Notify", &notify(0, "First")).await?;
    anyhow::ensure!(id > 0, "Zero identifier");
    let replaced: u32 = proxy.call("Notify", &notify(id, "Replaced")).await?;
    anyhow::ensure!(
        replaced == id && model.items().len() == 1,
        "Replacement duplicated history"
    );
    anyhow::ensure!(
        !model.items()[0].body.contains("<script>"),
        "Unsafe markup reached the model"
    );
    model.invoke(id, "yes", None);
    let action = invoked
        .next()
        .await
        .ok_or_else(|| anyhow::anyhow!("Missing action signal"))?;
    let (action_id, key): (u32, String) = action.body().deserialize()?;
    anyhow::ensure!(action_id == id && key == "yes", "Wrong action signal");
    let signal = closed
        .next()
        .await
        .ok_or_else(|| anyhow::anyhow!("Missing close signal"))?;
    anyhow::ensure!(
        signal.body().deserialize::<(u32, u32)>()? == (id, 2),
        "Wrong dismissal reason"
    );
    let id: u32 = proxy.call("Notify", &notify(0, "Close")).await?;
    proxy.call::<_, _, ()>("CloseNotification", &(id,)).await?;
    let signal = closed
        .next()
        .await
        .ok_or_else(|| anyhow::anyhow!("Missing close signal"))?;
    anyhow::ensure!(
        signal.body().deserialize::<(u32, u32)>()? == (id, 3),
        "Wrong client closure reason"
    );
    anyhow::ensure!(
        proxy
            .call::<_, _, ()>("CloseNotification", &(id,))
            .await
            .is_err(),
        "Closed identifier stayed valid"
    );
    let id: u32 = proxy.call("Notify", &notify(0, "Expire")).await?;
    model.close(id, 1);
    let signal = closed
        .next()
        .await
        .ok_or_else(|| anyhow::anyhow!("Missing expiry signal"))?;
    anyhow::ensure!(
        signal.body().deserialize::<(u32, u32)>()? == (id, 1),
        "Wrong expiry reason"
    );
    anyhow::ensure!(
        model.active(id).is_none() && model.items().iter().any(|item| item.id == id),
        "Expiry history semantics"
    );
    model.clear();
    anyhow::ensure!(model.items().is_empty(), "Clear failed");
    println!("Notifications 1.2: replace, sanitized body, action, reasons 1/2/3, invalid ID and in-memory history passed.");
    Ok(())
}
fn main() -> anyhow::Result<()> {
    let main_loop = glib::MainLoop::new(None, false);
    let finished = main_loop.clone();
    let success = Rc::new(Cell::new(false));
    let result = success.clone();
    glib::spawn_future_local(async move {
        let outcome = futures_lite::future::race(probe(), async {
            glib::timeout_future(std::time::Duration::from_secs(15)).await;
            Err(anyhow::anyhow!("Probe timed out"))
        })
        .await;
        match outcome {
            Ok(()) => result.set(true),
            Err(error) => eprintln!("{error:#}"),
        };
        finished.quit();
    });
    main_loop.run();
    anyhow::ensure!(success.get(), "Notification protocol probe failed");
    Ok(())
}
