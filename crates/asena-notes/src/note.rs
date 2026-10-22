//! Notes are Markdown files; these helpers derive titles and names from them.
use asena_core::text::fold;

/// Notes larger than this are listed but not loaded into the editor.
pub const MAX_BYTES: u64 = 4 * 1024 * 1024;

/// Title from the first non-empty line, without Markdown heading marks.
pub fn title_of(text: &str) -> Option<String> {
    text.lines()
        .map(|line| line.trim().trim_start_matches('#').trim())
        .find(|line| !line.is_empty())
        .map(|line| line.chars().take(80).collect())
}

/// Second non-empty line, shown under the title in the note list.
pub fn preview_of(text: &str) -> String {
    text.lines()
        .map(str::trim)
        .filter(|line| !line.is_empty())
        .nth(1)
        .map(|line| {
            line.trim_start_matches(['#', '-', '*', ' '])
                .chars()
                .take(100)
                .collect()
        })
        .unwrap_or_default()
}

/// File name for a new note created at `unix_micros`.
pub fn file_name(unix_micros: i64) -> String {
    format!("note-{unix_micros}.md")
}

/// True for files this application manages.
pub fn is_note(name: &str) -> bool {
    name.ends_with(".md") && !name.starts_with('.')
}

pub fn matches(text: &str, query: &str) -> bool {
    let query = fold(query.trim());
    query.is_empty() || fold(text).contains(&query)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn titles_skip_blank_lines_and_heading_marks() {
        assert_eq!(
            title_of("\n\n# Alışveriş\n- süt").as_deref(),
            Some("Alışveriş")
        );
        assert_eq!(
            title_of("Toplantı notları").as_deref(),
            Some("Toplantı notları")
        );
        assert_eq!(title_of("   \n  "), None);
        assert_eq!(preview_of("# Liste\n\n- ekmek\n- su"), "ekmek");
        assert_eq!(preview_of("Tek satır"), "");
    }

    #[test]
    fn names_and_search() {
        assert_eq!(file_name(42), "note-42.md");
        assert!(is_note("note-42.md"));
        assert!(!is_note(".note-42.md"));
        assert!(!is_note("note-42.md.tmp"));
        assert!(matches("İstanbul gezisi", "istanbul"));
        assert!(!matches("Ankara", "izmir"));
    }
}
