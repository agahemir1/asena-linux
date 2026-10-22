//! Text document helpers that do not touch GTK widgets.

/// Files above this size are not opened in the editor.
pub const MAX_BYTES: usize = 32 * 1024 * 1024;

#[derive(Debug, PartialEq, Eq)]
pub enum DecodeError {
    TooLarge,
    NotText,
}

/// Accepts UTF-8 text, with or without a byte order mark.
pub fn decode(bytes: &[u8]) -> Result<String, DecodeError> {
    if bytes.len() > MAX_BYTES {
        return Err(DecodeError::TooLarge);
    }
    let bytes = bytes.strip_prefix(&[0xEF, 0xBB, 0xBF]).unwrap_or(bytes);
    if bytes.contains(&0) {
        return Err(DecodeError::NotText);
    }
    String::from_utf8(bytes.to_vec()).map_err(|_| DecodeError::NotText)
}

/// Tab title: the file name with a dot while there are unsaved changes.
pub fn tab_title(name: Option<&str>, untitled: &str, modified: bool) -> String {
    let name = name.filter(|name| !name.is_empty()).unwrap_or(untitled);
    if modified {
        format!("• {name}")
    } else {
        name.to_owned()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn decodes_utf8_and_rejects_binary() {
        assert_eq!(
            decode("Merhaba dünya".as_bytes()).as_deref(),
            Ok("Merhaba dünya")
        );
        assert_eq!(decode(&[0xEF, 0xBB, 0xBF, b'a']).as_deref(), Ok("a"));
        assert_eq!(decode(&[0xFF, 0xFE, 0x00]), Err(DecodeError::NotText));
        assert_eq!(decode(b"a\0b"), Err(DecodeError::NotText));
    }

    #[test]
    fn titles_mark_unsaved_changes() {
        assert_eq!(
            tab_title(Some("notlar.txt"), "Untitled", false),
            "notlar.txt"
        );
        assert_eq!(tab_title(None, "Untitled", true), "• Untitled");
        assert_eq!(tab_title(Some(""), "Adsız", false), "Adsız");
    }
}
