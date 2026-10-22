//! Authorization through polkit, for the D-Bus connection that called.
use std::collections::HashMap;
use zbus::zvariant::Value;

#[zbus::proxy(
    interface = "org.freedesktop.PolicyKit1.Authority",
    default_service = "org.freedesktop.PolicyKit1",
    default_path = "/org/freedesktop/PolicyKit1/Authority"
)]
trait Authority {
    #[allow(clippy::type_complexity)]
    fn check_authorization(
        &self,
        subject: &(&str, HashMap<&str, Value<'_>>),
        action_id: &str,
        details: &HashMap<&str, &str>,
        flags: u32,
        cancellation_id: &str,
    ) -> zbus::Result<(bool, bool, HashMap<String, String>)>;
}

/// The agent may ask the user for a password.
const ALLOW_USER_INTERACTION: u32 = 1;

/// Asks polkit whether the bus name `sender` may perform `action`. The
/// `system-bus-name` subject ties the answer to the calling connection, so a
/// process cannot borrow another process's authorization.
pub async fn check(
    connection: &zbus::Connection,
    sender: &str,
    action: &str,
) -> zbus::Result<bool> {
    let authority = AuthorityProxy::new(connection).await?;
    let subject = (
        "system-bus-name",
        HashMap::from([("name", Value::from(sender))]),
    );
    let (authorized, _, _) = authority
        .check_authorization(
            &subject,
            action,
            &HashMap::new(),
            ALLOW_USER_INTERACTION,
            "",
        )
        .await?;
    Ok(authorized)
}
