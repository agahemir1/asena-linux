//! PDF documents through the poppler command line tools (`poppler-utils`):
//! `pdfinfo` for pages and sizes, `pdftoppm` for page images, `pdftotext` for search.
use std::{path::Path, process::Command};

#[derive(Clone, Debug, PartialEq)]
pub struct Info {
    pub title: Option<String>,
    /// Page sizes in points, in page order.
    pub sizes: Vec<(f64, f64)>,
}

#[derive(Debug, PartialEq, Eq)]
pub enum Error {
    /// `poppler-utils` is not installed.
    Missing,
    /// The document needs a password.
    Encrypted,
    Unreadable(String),
}

/// `612 x 792 pts (letter)` → (612, 792).
fn size(text: &str) -> Option<(f64, f64)> {
    let mut parts = text.split_whitespace();
    let width: f64 = parts.next()?.parse().ok()?;
    if parts.next()? != "x" {
        return None;
    }
    let height: f64 = parts.next()?.parse().ok()?;
    (width > 0.0 && height > 0.0).then_some((width, height))
}

/// Output of `pdfinfo -f 1 -l <last>`, which lists every page's size.
pub fn parse_info(text: &str) -> Option<Info> {
    let mut title = None;
    let mut pages = 0usize;
    let mut sizes = Vec::new();
    let mut fallback = None;
    for line in text.lines() {
        let Some((key, value)) = line.split_once(':') else {
            continue;
        };
        let value = value.trim();
        match key.trim() {
            "Title" if !value.is_empty() => title = Some(value.to_owned()),
            "Pages" => pages = value.parse().ok()?,
            "Page size" => fallback = size(value),
            key if key.starts_with("Page") && key.ends_with("size") => {
                if let Some(size) = size(value) {
                    sizes.push(size);
                }
            }
            _ => {}
        }
    }
    if pages == 0 {
        return None;
    }
    // Pages missing from the listing take the first page's size.
    let first = sizes
        .first()
        .copied()
        .or(fallback)
        .unwrap_or((612.0, 792.0));
    sizes.resize(pages, first);
    Some(Info { title, sizes })
}

fn run(command: &mut Command) -> Result<Vec<u8>, Error> {
    let output = command.env("LC_ALL", "C").output().map_err(|error| {
        if error.kind() == std::io::ErrorKind::NotFound {
            Error::Missing
        } else {
            Error::Unreadable(error.to_string())
        }
    })?;
    if output.status.success() {
        return Ok(output.stdout);
    }
    let message = String::from_utf8_lossy(&output.stderr).into_owned();
    if message.contains("Incorrect password") {
        Err(Error::Encrypted)
    } else {
        Err(Error::Unreadable(message))
    }
}

/// Paths are absolute, so they never look like an option to the tools.
pub fn info(path: &Path) -> Result<Info, Error> {
    let output = run(Command::new("pdfinfo")
        .args(["-f", "1", "-l", "100000"])
        .arg(path))?;
    parse_info(&String::from_utf8_lossy(&output))
        .ok_or_else(|| Error::Unreadable("no pages".into()))
}

/// PNG image of the page (numbered from 1) at `dpi`.
pub fn render(path: &Path, page: usize, dpi: u32) -> Result<Vec<u8>, Error> {
    let page = page.to_string();
    run(Command::new("pdftoppm")
        .args(["-png", "-singlefile", "-r", &dpi.to_string()])
        .args(["-f", &page, "-l", &page])
        .arg(path))
}

/// The text of every page; `pdftotext` ends each page with a form feed.
pub fn text(path: &Path) -> Result<Vec<String>, Error> {
    let output = run(Command::new("pdftotext")
        .args(["-layout", "-enc", "UTF-8"])
        .arg(path)
        .arg("-"))?;
    Ok(split_pages(&String::from_utf8_lossy(&output)))
}

pub fn split_pages(text: &str) -> Vec<String> {
    let mut pages: Vec<String> = text.split('\u{c}').map(str::to_owned).collect();
    if pages.last().is_some_and(|page| page.trim().is_empty()) {
        pages.pop();
    }
    pages
}

/// Zero-based indexes of the pages that contain `query`, matched with Turkish folding.
pub fn find(pages: &[String], query: &str) -> Vec<usize> {
    let query = asena_core::text::fold(query.trim());
    if query.is_empty() {
        return Vec::new();
    }
    pages
        .iter()
        .enumerate()
        .filter(|(_, text)| asena_core::text::fold(text).contains(&query))
        .map(|(index, _)| index)
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    const INFO: &str = "Title:          Asena Vizyon\nProducer:       LibreOffice\nPages:          3\nEncrypted:      no\nPage    1 size: 595.276 x 841.89 pts (A4)\nPage    1 rot:  0\nPage    2 size: 841.89 x 595.276 pts (A4)\nPage    2 rot:  0\nFile size:      12345 bytes\n";

    #[test]
    fn info_lists_every_page() {
        let info = parse_info(INFO);
        let Some(info) = info else {
            panic!("the sample has pages");
        };
        assert_eq!(info.title.as_deref(), Some("Asena Vizyon"));
        assert_eq!(info.sizes.len(), 3);
        assert_eq!(info.sizes[1], (841.89, 595.276));
        assert_eq!(info.sizes[2], (595.276, 841.89));
    }

    #[test]
    fn info_without_pages_is_rejected() {
        assert_eq!(parse_info("Title: x\n"), None);
        let single = parse_info("Pages: 1\nPage size:      612 x 792 pts (letter)\n");
        assert_eq!(single.map(|info| info.sizes), Some(vec![(612.0, 792.0)]));
    }

    #[test]
    fn text_splits_on_form_feeds() {
        let pages = split_pages("Birinci sayfa\n\u{c}İkinci SAYFA\n\u{c}");
        assert_eq!(pages.len(), 2);
        assert_eq!(find(&pages, "ikinci"), [1]);
        assert_eq!(find(&pages, "SAYFA"), [0, 1]);
        assert!(find(&pages, "  ").is_empty());
    }
}
