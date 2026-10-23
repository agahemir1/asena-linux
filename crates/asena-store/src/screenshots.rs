//! Screenshots for the detail page, fetched over HTTPS only when the page
//! opens and kept in the user's cache.
use gio::prelude::*;
use soup::prelude::*;
use std::path::PathBuf;

/// Larger answers are not screenshots.
const LIMIT: usize = 8 * 1024 * 1024;

fn cache_path(url: &str) -> PathBuf {
    let key = glib::compute_checksum_for_string(glib::ChecksumType::Sha256, url)
        .map(|sum| sum.to_string())
        .unwrap_or_default();
    glib::user_cache_dir()
        .join("asena/store/screenshots")
        .join(key)
}

thread_local! {
    static SESSION: soup::Session = {
        let session = soup::Session::new();
        session.set_timeout(30);
        session.set_user_agent("Asena Software");
        session
    };
}

pub async fn load(url: String) -> Option<gtk::gdk::Texture> {
    if !url.starts_with("https://") {
        return None;
    }
    let path = cache_path(&url);
    let cached = gio::File::for_path(&path);
    if let Ok((bytes, _)) = cached.load_contents_future().await {
        if let Ok(texture) = gtk::gdk::Texture::from_bytes(&glib::Bytes::from(bytes.as_ref())) {
            return Some(texture);
        }
    }
    let message = soup::Message::new("GET", &url).ok()?;
    let session = SESSION.with(Clone::clone);
    let bytes = session
        .send_and_read_future(&message, glib::Priority::LOW)
        .await
        .ok()?;
    if message.status() != soup::Status::Ok || bytes.len() > LIMIT {
        return None;
    }
    let texture = gtk::gdk::Texture::from_bytes(&bytes).ok()?;
    if let Some(directory) = path.parent().map(PathBuf::from) {
        let _ = gio::spawn_blocking(move || std::fs::create_dir_all(directory)).await;
    }
    let _ = cached
        .replace_contents_future(
            bytes,
            None,
            false,
            gio::FileCreateFlags::REPLACE_DESTINATION,
        )
        .await;
    Some(texture)
}
