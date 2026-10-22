//! Greeter-only public account metadata; never inspect another user's home.
#[derive(Clone)]
pub struct User {
    pub username: String,
    pub name: String,
}
pub async fn users() -> zbus::Result<Vec<User>> {
    let connection = zbus::Connection::system().await?;
    let proxy = zbus::Proxy::new(
        &connection,
        "org.freedesktop.Accounts",
        "/org/freedesktop/Accounts",
        "org.freedesktop.Accounts",
    )
    .await?;
    let paths: Vec<zbus::zvariant::OwnedObjectPath> = proxy.call("ListCachedUsers", &()).await?;
    let mut users = Vec::new();
    for path in paths.into_iter().take(256) {
        let user = zbus::Proxy::new(
            &connection,
            "org.freedesktop.Accounts",
            path,
            "org.freedesktop.Accounts.User",
        )
        .await?;
        if user
            .get_property::<bool>("SystemAccount")
            .await
            .unwrap_or(true)
            || user.get_property::<u64>("Uid").await.unwrap_or(0) < 1000
        {
            continue;
        }
        let Ok(username) = user.get_property::<String>("UserName").await else {
            continue;
        };
        let name = user
            .get_property::<String>("RealName")
            .await
            .ok()
            .filter(|name| !name.trim().is_empty())
            .unwrap_or_else(|| username.clone());
        users.push(User { username, name });
    }
    users.sort_by(|a, b| {
        glib::CollationKey::from(a.name.as_str()).cmp(&glib::CollationKey::from(b.name.as_str()))
    });
    Ok(users)
}
