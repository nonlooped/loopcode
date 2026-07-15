//! Line-level diff stats (+added / -removed) for mutating workspace tools.

use similar::{ChangeTag, TextDiff};

/// Count added/removed lines between two text bodies.
pub fn line_counts(before: &str, after: &str) -> (u64, u64) {
    let diff = TextDiff::from_lines(before, after);
    let mut added = 0u64;
    let mut removed = 0u64;
    for change in diff.iter_all_changes() {
        match change.tag() {
            ChangeTag::Insert => added += 1,
            ChangeTag::Delete => removed += 1,
            ChangeTag::Equal => {}
        }
    }
    (added, removed)
}

/// Line count of a newly created file (all additions).
pub fn lines_of(content: &str) -> u64 {
    content.lines().count() as u64
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn counts_adds_and_removes() {
        let (a, r) = line_counts("a\nb\nc\n", "a\nB\nc\nd\n");
        assert_eq!((a, r), (2, 1)); // b→B is remove+add, plus d added
    }

    #[test]
    fn created_file_is_all_additions() {
        assert_eq!(lines_of("x\ny\n"), 2);
        assert_eq!(lines_of(""), 0);
    }
}
