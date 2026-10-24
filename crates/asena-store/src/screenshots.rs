//! Screenshots for discovery and detail pages, fetched over HTTPS and cached.
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
        let session = soup::Session::builder().max_conns(3).max_conns_per_host(2).build();
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
    let stream = session
        .send_future(&message, glib::Priority::LOW)
        .await
        .ok()?;
    if message.status() != soup::Status::Ok {
        return None;
    }
    // Stop while reading, rather than allocating an arbitrarily large response
    // before checking its size. The session bounds simultaneous feed requests.
    let mut data = Vec::new();
    loop {
        let chunk = stream
            .read_bytes_future(64 * 1024, glib::Priority::LOW)
            .await
            .ok()?;
        if chunk.is_empty() {
            break;
        }
        if data.len() + chunk.len() > LIMIT {
            return None;
        }
        data.extend_from_slice(&chunk);
    }
    let bytes = glib::Bytes::from_owned(data);
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
