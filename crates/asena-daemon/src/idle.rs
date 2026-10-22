//! One swayidle process; settings changes stop and reap its whole command group.
use crate::child::Helper;
use asena_core::power::Timeouts;
use gio::prelude::*;
use std::path::Path;

pub fn timeouts(settings: &gio::Settings) -> Result<Timeouts, asena_core::power::InvalidTimeouts> {
    Timeouts {
        dim: settings.uint("dim-seconds"),
        lock: settings.uint("lock-seconds"),
        off: settings.uint("off-seconds"),
        suspend: settings.uint("suspend-seconds"),
    }
    .validate()
}
fn quote(program: &str) -> String {
    format!("'{}'", program.replace('\'', "'\\''"))
}
pub fn arguments(program: &str, timeouts: Timeouts) -> Vec<String> {
    let prefix = format!("{} idle", quote(program));
    let mut args = vec!["-C".into(), "/dev/null".into(), "-w".into()];
    for (seconds, action, resume) in [
        (timeouts.dim, "dim", Some("restore")),
        (timeouts.lock, "lock", None),
        (timeouts.off, "off", Some("on")),
        (timeouts.suspend, "suspend", None),
    ] {
        if seconds == 0 {
            continue;
        }
        args.extend([
            "timeout".into(),
            seconds.to_string(),
            format!("{prefix} {action}"),
        ]);
        if let Some(action) = resume {
            args.extend(["resume".into(), format!("{prefix} {action}")]);
        }
    }
    args
}
pub async fn start(timeouts: Timeouts) -> anyhow::Result<Option<Helper>> {
    if [timeouts.dim, timeouts.lock, timeouts.off, timeouts.suspend]
        .into_iter()
        .all(|value| value == 0)
    {
        return Ok(None);
    }
    let path = if std::env::var_os("ASENA_DEV").is_some() {
        Path::new(
            &std::env::var_os("ASENA_SOURCE_DIR")
                .ok_or_else(|| anyhow::anyhow!("Source directory missing"))?,
        )
        .join("target/debug/asenactl")
    } else {
        Path::new("/usr/bin/asenactl").to_path_buf()
    };
    let program = path
        .to_str()
        .ok_or_else(|| anyhow::anyhow!("Non-Unicode command path"))?;
    Helper::start("/usr/bin/swayidle".into(), arguments(program, timeouts))
        .await
        .map(Some)
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn disabled_steps_and_shell_metacharacters_are_not_commands() {
        let args = arguments(
            "/tmp/a'$(echo bad)/ctl",
            Timeouts {
                dim: 1,
                lock: 2,
                off: 0,
                suspend: 0,
            },
        );
        assert_eq!(&args[..3], &["-C", "/dev/null", "-w"]);
        assert_eq!(args[5], "'/tmp/a'\\''$(echo bad)/ctl' idle dim");
        assert_eq!(
            args.iter().filter(|arg| arg.as_str() == "timeout").count(),
            2
        );
        assert_eq!(
            args.iter().filter(|arg| arg.as_str() == "resume").count(),
            1
        );
    }
}
