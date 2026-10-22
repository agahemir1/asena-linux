//! Screen capture through `org.freedesktop.portal.Screenshot`.
//!
//! The portal backend selected for the session (xdg-desktop-portal-wlr on
//! Wayfire) performs the capture, so the shell stays compositor independent.
use crate::i18n::gettext;
use futures_lite::StreamExt;
use gio::prelude::*;
use std::{collections::HashMap, path::PathBuf};
use zbus::zvariant::{OwnedValue, Value};

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("{0}")]
    Bus(#[from] zbus::Error),
    #[error("{0}")]
    Cancelled(String),
    #[error("{0}")]
    Failed(String),
    #[error("{0}")]
    Io(#[from] std::io::Error),
}

/// Object path where the portal publishes the request for `token`.
pub fn request_path(unique_name: &str, token: &str) -> String {
    let sender = unique_name.trim_start_matches(':').replace('.', "_");
    format!("/org/freedesktop/portal/desktop/request/{sender}/{token}")
}

/// File name for a capture taken at `stamp` (for example "2026-10-21 17-40-02").
pub fn file_name(stamp: &str) -> String {
    format!(
        "{}.png",
        gettext("Screenshot {date}").replace("{date}", stamp)
    )
}

/// `~/Pictures/Screenshots`, or the home folder without a Pictures folder.
pub fn directory() -> PathBuf {
    glib::user_special_dir(glib::UserDirectory::Pictures)
        .unwrap_or_else(glib::home_dir)
        .join(gettext("Screenshots"))
}

/// Asks the portal for a capture and returns the URI of the image it wrote.
pub async fn capture(connection: &zbus::Connection, interactive: bool) -> Result<String, Error> {
    let unique = connection
        .unique_name()
        .map(|name| name.to_string())
        .ok_or_else(|| Error::Failed(gettext("The session bus connection has no name.")))?;
    let token = format!(
        "asena{}",
        std::process::id() as u64 * 1000 + u64::from(glib::random_int() % 1000)
    );
    let path = request_path(&unique, &token);
    // Subscribe before calling so the response cannot be missed.
    let rule = zbus::MatchRule::builder()
        .msg_type(zbus::message::Type::Signal)
        .interface("org.freedesktop.portal.Request")?
        .member("Response")?
        .path(path.as_str())?
        .build();
    let mut responses = zbus::MessageStream::for_match_rule(rule, connection, Some(4)).await?;
    let mut options: HashMap<&str, Value<'_>> = HashMap::new();
    options.insert("handle_token", Value::from(token.as_str()));
    options.insert("interactive", Value::from(interactive));
    let proxy = zbus::Proxy::new(
        connection,
        "org.freedesktop.portal.Desktop",
        "/org/freedesktop/portal/desktop",
        "org.freedesktop.portal.Screenshot",
    )
    .await?;
    let _: zbus::zvariant::OwnedObjectPath = proxy.call("Screenshot", &("", options)).await?;
    if let Some(message) = responses.next().await {
        let message = message?;
        let (code, results): (u32, HashMap<String, OwnedValue>) = message.body().deserialize()?;
        return match code {
            0 => results
                .get("uri")
                .and_then(|value| <&str>::try_from(value).ok())
                .map(ToOwned::to_owned)
                .ok_or_else(|| Error::Failed(gettext("The screenshot service returned no image."))),
            1 => Err(Error::Cancelled(gettext("The screenshot was cancelled."))),
            _ => Err(Error::Failed(gettext("The screenshot could not be taken."))),
        };
    }
    Err(Error::Failed(gettext(
        "The screenshot service stopped responding.",
    )))
}

/// Moves the portal's image into the screenshots folder. Blocking; call it
/// from a worker thread.
pub fn store(uri: &str, stamp: &str) -> Result<PathBuf, Error> {
    let source = gio::File::for_uri(uri)
        .path()
        .ok_or_else(|| Error::Failed(gettext("The screenshot is not a local file.")))?;
    let directory = directory();
    std::fs::create_dir_all(&directory)?;
    let mut target = directory.join(file_name(stamp));
    let mut index = 2;
    while target.exists() {
        target = directory.join(file_name(&format!("{stamp} ({index})")));
        index += 1;
    }
    if std::fs::rename(&source, &target).is_err() {
        // Different file systems: copy, then remove the portal's temporary file.
        std::fs::copy(&source, &target)?;
        let _ = std::fs::remove_file(&source);
    }
    Ok(target)
}

/// Local timestamp used in file names; colons are avoided for portability.
pub fn stamp() -> String {
    glib::DateTime::now_local()
        .ok()
        .and_then(|now| now.format("%Y-%m-%d %H-%M-%S").ok())
        .map(|text| text.to_string())
        .unwrap_or_else(|| "screenshot".into())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn request_paths_follow_the_portal_convention() {
        assert_eq!(
            request_path(":1.42", "asena7"),
            "/org/freedesktop/portal/desktop/request/1_42/asena7"
        );
    }

    #[test]
    fn file_names_use_the_stamp() {
        assert_eq!(
            file_name("2026-10-21 17-40-02"),
            "Screenshot 2026-10-21 17-40-02.png"
        );
        assert!(stamp().len() >= 10);
    }
}
