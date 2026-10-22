//! Own a helper process group, including any commands it starts.
use std::{
    os::unix::process::CommandExt,
    process::{Child, Command, Stdio},
};
pub struct Helper(Option<Child>);
impl Helper {
    pub async fn start(program: String, arguments: Vec<String>) -> anyhow::Result<Self> {
        gio::spawn_blocking(move || {
            Command::new(program)
                .args(arguments)
                .stdin(Stdio::null())
                .stdout(Stdio::null())
                .process_group(0)
                .spawn()
                .map(|child| Self(Some(child)))
        })
        .await
        .map_err(|error| anyhow::anyhow!("{error:?}"))?
        .map_err(Into::into)
    }
    pub async fn stop(mut self) {
        if let Some(child) = self.0.take() {
            let _ = gio::spawn_blocking(move || stop(child)).await;
        }
    }
}
fn stop(mut child: Child) {
    // The unreaped leader reserves this ID until its whole group is signalled.
    let _ = Command::new("/bin/kill")
        .args(["-KILL", "--", &format!("-{}", child.id())])
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status();
    let _ = child.kill();
    let _ = child.wait();
}
impl Drop for Helper {
    fn drop(&mut self) {
        if let Some(child) = self.0.take() {
            gio::spawn_blocking(move || stop(child));
        }
    }
}
