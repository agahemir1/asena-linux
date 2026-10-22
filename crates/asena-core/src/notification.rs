//! Untrusted notification payload validation and bounded lifecycle model.
use quick_xml::{events::Event, Reader};
use std::collections::VecDeque;

pub fn sanitize_markup(input: &str) -> String {
    let mut reader = Reader::from_str(input);
    let mut output = String::new();
    let mut stack = Vec::new();
    loop {
        match reader.read_event() {
            Ok(Event::Eof) => break,
            Ok(Event::Text(text)) => {
                let Ok(text) = text.unescape() else {
                    return glib::markup_escape_text(input).into();
                };
                output.push_str(&glib::markup_escape_text(&text));
            }
            Ok(Event::Start(tag)) => {
                if stack.len() >= 32 {
                    return glib::markup_escape_text(input).into();
                }
                let name = tag.name();
                let name = name.as_ref();
                let allowed = match name {
                    b"b" | b"i" | b"u" if tag.attributes().next().is_none() => {
                        Some(String::from_utf8_lossy(name).into_owned())
                    }
                    b"a" => {
                        let attributes: Result<Vec<_>, _> = tag.attributes().collect();
                        attributes
                            .ok()
                            .filter(|a| a.len() == 1 && a[0].key.as_ref() == b"href")
                            .and_then(|a| a[0].unescape_value().ok().map(|s| s.into_owned()))
                            .filter(|uri| safe_uri(uri))
                            .map(|uri| format!("a href=\"{}\"", glib::markup_escape_text(&uri)))
                    }
                    _ => None,
                };
                if let Some(allowed) = allowed {
                    output.push('<');
                    output.push_str(&allowed);
                    output.push('>');
                    stack.push(Some(String::from_utf8_lossy(name).into_owned()));
                } else {
                    output.push_str(&glib::markup_escape_text(&format!(
                        "<{}>",
                        String::from_utf8_lossy(tag.as_ref())
                    )));
                    stack.push(None);
                }
            }
            Ok(Event::End(tag)) => {
                if stack.pop().flatten().is_some() {
                    output.push_str(&format!(
                        "</{}>",
                        String::from_utf8_lossy(tag.name().as_ref())
                    ));
                } else {
                    output.push_str(&glib::markup_escape_text(&format!(
                        "</{}>",
                        String::from_utf8_lossy(tag.name().as_ref())
                    )));
                }
            }
            Ok(Event::CData(text)) => {
                output.push_str(&glib::markup_escape_text(&String::from_utf8_lossy(&text)))
            }
            Ok(Event::Empty(tag)) => output.push_str(&glib::markup_escape_text(&format!(
                "<{} />",
                String::from_utf8_lossy(tag.as_ref())
            ))),
            Ok(_) | Err(_) => return glib::markup_escape_text(input).into(),
        }
    }
    if stack.is_empty() {
        output
    } else {
        glib::markup_escape_text(input).into()
    }
}
pub fn safe_uri(uri: &str) -> bool {
    uri.starts_with("https://") || uri.starts_with("http://") || uri.starts_with("mailto:")
}

#[derive(Clone, Debug)]
pub struct Image {
    pub width: i32,
    pub height: i32,
    pub rgba: Vec<u8>,
}
pub fn image_data(
    width: i32,
    height: i32,
    stride: i32,
    alpha: bool,
    bits: i32,
    channels: i32,
    data: &[u8],
) -> Option<Image> {
    if !(1..=2048).contains(&width)
        || !(1..=2048).contains(&height)
        || bits != 8
        || channels != if alpha { 4 } else { 3 }
    {
        return None;
    }
    let (width_usize, height_usize, stride_usize) = (
        width as usize,
        height as usize,
        usize::try_from(stride).ok()?,
    );
    let row = width_usize.checked_mul(channels as usize)?;
    let needed = (height_usize - 1)
        .checked_mul(stride_usize)?
        .checked_add(row)?;
    if stride_usize < row || needed > data.len() || needed > 16_777_216 {
        return None;
    }
    let mut rgba = Vec::with_capacity(width_usize * height_usize * 4);
    for y in 0..height_usize {
        for pixel in data[y * stride_usize..y * stride_usize + row].chunks_exact(channels as usize)
        {
            rgba.extend_from_slice(&[
                pixel[0],
                pixel[1],
                pixel[2],
                if alpha { pixel[3] } else { 255 },
            ]);
        }
    }
    Some(Image {
        width,
        height,
        rgba,
    })
}

#[derive(Clone, Debug)]
pub struct Notification {
    pub id: u32,
    pub app: String,
    pub icon: String,
    pub summary: String,
    pub body: String,
    pub actions: Vec<(String, String)>,
    pub critical: bool,
    pub transient: bool,
    pub resident: bool,
    pub desktop_entry: Option<String>,
    pub category: Option<String>,
    pub image: Option<Image>,
    pub timeout_ms: Option<u32>,
    pub unread: bool,
}
pub fn application_key(desktop: Option<&str>, name: &str) -> String {
    match desktop.filter(|id| !id.is_empty()) {
        Some(id) => format!("desktop:{}", id.trim_end_matches(".desktop")),
        None => format!("name:{}", crate::text::fold(name)),
    }
}
#[derive(Default)]
pub struct History {
    pub items: VecDeque<Notification>,
    next_id: u32,
}
impl History {
    pub fn allocate(&mut self) -> Option<u32> {
        self.next_id = self.next_id.checked_add(1)?;
        Some(self.next_id)
    }
    pub fn insert(&mut self, item: Notification) {
        self.items.retain(|old| old.id != item.id);
        if !item.transient {
            self.items.push_front(item);
        }
        self.items.truncate(100);
    }
    pub fn remove(&mut self, id: u32) {
        self.items.retain(|item| item.id != id);
    }
}

#[cfg(test)]
#[allow(clippy::expect_used)]
mod tests {
    use super::*;
    #[test]
    fn only_safe_markup_and_uri_survive() {
        assert_eq!(
            sanitize_markup("<b>kalın</b> &amp; <i>ince</i>"),
            "<b>kalın</b> &amp; <i>ince</i>"
        );
        let unsafe_body = sanitize_markup("<script>x</script><a href=\"javascript:bad\">link</a>");
        assert!(!unsafe_body.contains("<script>"));
        assert!(!unsafe_body.contains("<a "));
        assert!(sanitize_markup("<a href=\"https://example.org\">link</a>").starts_with("<a href="));
        assert!(!sanitize_markup("<b>open").contains("<b>"));
        assert!(!sanitize_markup("<b foreground=\"red\">x</b>").contains("<b "));
    }
    #[test]
    fn respects_row_padding_and_rejects_bad_images() {
        let image = image_data(1, 2, 4, false, 8, 3, &[1, 2, 3, 99, 4, 5, 6]).expect("valid image");
        assert_eq!(image.rgba, [1, 2, 3, 255, 4, 5, 6, 255]);
        for (w, h, s) in [
            (0, 2, 4),
            (4096, 1, 16384),
            (2, 2, 3),
            (1, 2, -1),
            (1, 3, 4),
        ] {
            assert!(image_data(w, h, s, false, 8, 3, &[0; 8]).is_none());
        }
        assert!(image_data(1, 1, 4, true, 16, 4, &[0; 4]).is_none());
    }
    #[test]
    fn history_bounds_replacements_and_transient_privacy() {
        let mut history = History::default();
        let item = |id, transient| Notification {
            id,
            transient,
            app: String::new(),
            icon: String::new(),
            summary: id.to_string(),
            body: String::new(),
            actions: vec![],
            critical: false,
            resident: false,
            desktop_entry: None,
            category: None,
            image: None,
            timeout_ms: Some(5000),
            unread: true,
        };
        for _ in 0..110 {
            let id = history.allocate().expect("identifier");
            history.insert(item(id, false));
        }
        assert_eq!(history.items.len(), 100);
        assert_eq!(history.items.back().map(|n| n.id), Some(11));
        history.insert(item(110, false));
        assert_eq!(history.items.len(), 100);
        history.insert(item(111, true));
        assert_eq!(history.items.front().map(|n| n.id), Some(110));
        history.remove(110);
        assert_eq!(history.items.len(), 99);
    }
}
