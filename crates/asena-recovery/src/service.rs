use std::{
    io::Write,
    process::{Command, Stdio},
    sync::atomic::{AtomicBool, Ordering},
};
use zeroize::Zeroizing;

const ID: &str = "org.asenalinux.Recovery";
const PATH: &str = "/org/asenalinux/Recovery";
const ENGINE: &str = "/usr/lib/asena/recovery.py";

fn engine(command: &str, input: &str) -> zbus::fdo::Result<String> {
    let mut child = Command::new("/usr/bin/python3")
        .args(["-I", ENGINE, command])
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|error| zbus::fdo::Error::Failed(error.to_string()))?;
    if let Some(mut stdin) = child.stdin.take() {
        stdin
            .write_all(input.as_bytes())
            .map_err(|error| zbus::fdo::Error::Failed(error.to_string()))?;
    }
    let output = child
        .wait_with_output()
        .map_err(|error| zbus::fdo::Error::Failed(error.to_string()))?;
    if !output.status.success() {
        // The engine never includes passwords or account files in errors.
        return Err(zbus::fdo::Error::Failed(
            String::from_utf8_lossy(&output.stderr).trim().into(),
        ));
    }
    Ok(String::from_utf8_lossy(&output.stdout).trim().into())
}

#[derive(Default)]
struct Service {
    busy: AtomicBool,
}

struct Busy<'a>(&'a AtomicBool);
impl Drop for Busy<'_> {
    fn drop(&mut self) {
        self.0.store(false, Ordering::Release);
    }
}

impl Service {
    async fn authorize(
        &self,
        connection: &zbus::Connection,
        header: &zbus::message::Header<'_>,
    ) -> zbus::fdo::Result<()> {
        use std::collections::HashMap;
        use zbus::zvariant::Value;
        let sender = header
            .sender()
            .ok_or_else(|| zbus::fdo::Error::AccessDenied("Missing sender".into()))?;
        let proxy = zbus::Proxy::new(
            connection,
            "org.freedesktop.PolicyKit1",
            "/org/freedesktop/PolicyKit1/Authority",
            "org.freedesktop.PolicyKit1.Authority",
        )
        .await?;
        let subject = (
            "system-bus-name",
            HashMap::from([("name", Value::from(sender.as_str()))]),
        );
        let (allowed, _, _): (bool, bool, HashMap<String, String>) = proxy
            .call(
                "CheckAuthorization",
                &(
                    subject,
                    "org.asenalinux.recovery.reset",
                    HashMap::<&str, &str>::new(),
                    1u32,
                    "",
                ),
            )
            .await?;
        if allowed {
            Ok(())
        } else {
            Err(zbus::fdo::Error::AccessDenied(
                "Reset was not authorized".into(),
            ))
        }
    }

    async fn work(
        &self,
        command: &'static str,
        input: Zeroizing<String>,
    ) -> zbus::fdo::Result<String> {
        if self
            .busy
            .compare_exchange(false, true, Ordering::AcqRel, Ordering::Acquire)
            .is_err()
        {
            return Err(zbus::fdo::Error::Failed(
                "A recovery operation is running".into(),
            ));
        }
        let _busy = Busy(&self.busy);
        gio::spawn_blocking(move || engine(command, &input))
            .await
            .map_err(|_| zbus::fdo::Error::Failed("Recovery worker stopped".into()))?
    }
}

#[zbus::interface(name = "org.asenalinux.Recovery")]
impl Service {
    async fn status(&self) -> zbus::fdo::Result<String> {
        self.work("status", Zeroizing::new(String::new())).await
    }

    async fn schedule(
        &self,
        mode: &str,
        #[zbus(connection)] connection: &zbus::Connection,
        #[zbus(header)] header: zbus::message::Header<'_>,
    ) -> zbus::fdo::Result<()> {
        self.authorize(connection, &header).await?;
        let input = Zeroizing::new(serde_json::json!({"mode": mode}).to_string());
        self.work("schedule", input).await?;
        Ok(())
    }

    async fn cancel(
        &self,
        #[zbus(connection)] connection: &zbus::Connection,
        #[zbus(header)] header: zbus::message::Header<'_>,
    ) -> zbus::fdo::Result<()> {
        self.authorize(connection, &header).await?;
        self.work("cancel", Zeroizing::new(String::new())).await?;
        Ok(())
    }

    async fn reset(
        &self,
        mode: &str,
        username: &str,
        password: &str,
        #[zbus(connection)] connection: &zbus::Connection,
        #[zbus(header)] header: zbus::message::Header<'_>,
    ) -> zbus::fdo::Result<()> {
        self.authorize(connection, &header).await?;
        let input = Zeroizing::new(
            serde_json::json!({"mode": mode, "username": username, "password": password})
                .to_string(),
        );
        self.work("reset", input).await?;
        Ok(())
    }
}

pub fn run() -> anyhow::Result<()> {
    let main_loop = glib::MainLoop::new(None, false);
    let finished = main_loop.clone();
    let failure = std::rc::Rc::new(std::cell::RefCell::new(None));
    let error_slot = failure.clone();
    let task = glib::spawn_future_local(async move {
        let result = async {
            let connection = zbus::connection::Builder::system()?
                .serve_at(PATH, Service::default())?
                .build()
                .await?;
            asena_core::dbus::claim_name(&connection, ID).await?;
            Ok::<_, zbus::Error>(connection)
        }
        .await;
        match result {
            Ok(_connection) => std::future::pending::<()>().await,
            Err(error) => {
                error_slot.replace(Some(error.to_string()));
                finished.quit();
            }
        }
    });
    main_loop.run();
    task.abort();
    if let Some(error) = failure.borrow().as_ref() {
        anyhow::bail!("{error}");
    }
    Ok(())
}
