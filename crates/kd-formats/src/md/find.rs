//! Find and Replace.

use std::ops::Range;

use kd_doc::{Change, Selection};

use super::edit::Edit;

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct FindOptions {
    pub match_case: bool,
    pub whole_word: bool,
}

fn is_word(c: char) -> bool {
    c.is_alphanumeric() || c == '_'
}

/// Every match of `query` in `text` (byte ranges, in order).
pub fn find_all(text: &str, query: &str, opts: FindOptions) -> Vec<Range<usize>> {
    if query.is_empty() {
        return Vec::new();
    }
    let mut out = Vec::new();
    if opts.match_case {
        for (i, m) in text.match_indices(query) {
            out.push(i..i + m.len());
        }
    } else {
        // compare char by char so offsets stay those of `text`
        let q: Vec<char> = query.chars().flat_map(char::to_lowercase).collect();
        let chars: Vec<(usize, char)> = text.char_indices().collect();
        let mut i = 0;
        while i < chars.len() {
            let mut j = i;
            let mut k = 0;
            while k < q.len() && j < chars.len() {
                let lower: Vec<char> = chars[j].1.to_lowercase().collect();
                if q[k..].starts_with(&lower) {
                    k += lower.len();
                    j += 1;
                } else {
                    break;
                }
            }
            if k == q.len() {
                let end = chars.get(j).map(|c| c.0).unwrap_or(text.len());
                out.push(chars[i].0..end);
                i = j.max(i + 1);
            } else {
                i += 1;
            }
        }
    }
    if opts.whole_word {
        out.retain(|r| {
            let before = text[..r.start].chars().next_back().is_none_or(|c| !is_word(c));
            let after = text[r.end..].chars().next().is_none_or(|c| !is_word(c));
            before && after
        });
    }
    out
}

/// The match at or after `from` (wrapping around), or before it when `back`.
pub fn next_match(matches: &[Range<usize>], from: usize, back: bool) -> Option<usize> {
    if matches.is_empty() {
        return None;
    }
    if back {
        Some(matches.iter().rposition(|m| m.start < from).unwrap_or(matches.len() - 1))
    } else {
        Some(matches.iter().position(|m| m.start >= from).unwrap_or(0))
    }
}

/// Replace every match (one undo step).
pub fn replace_all(text: &str, query: &str, with: &str, opts: FindOptions, sel: Selection) -> Option<(Edit, usize)> {
    let m = find_all(text, query, opts);
    if m.is_empty() {
        return None;
    }
    let changes: Vec<Change> = m.iter().map(|r| Change::new(r.start, r.end, with)).collect();
    let selection = super::edit::shift_selection(sel, &changes);
    Some((Edit { changes, selection }, m.len()))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn finds_with_options() {
        let t = "Cat cat concat CAT";
        assert_eq!(find_all(t, "cat", FindOptions::default()).len(), 4);
        assert_eq!(find_all(t, "cat", FindOptions { match_case: true, whole_word: false }).len(), 2);
        assert_eq!(find_all(t, "cat", FindOptions { match_case: false, whole_word: true }).len(), 3);
        assert_eq!(find_all("Ünïcode ünïcode", "ÜNÏ", FindOptions::default()), vec![0..5, 10..15]);
        assert!(find_all(t, "", FindOptions::default()).is_empty());
    }

    #[test]
    fn next_wraps() {
        let m = vec![0..1, 5..6, 9..10];
        assert_eq!(next_match(&m, 6, false), Some(2));
        assert_eq!(next_match(&m, 11, false), Some(0));
        assert_eq!(next_match(&m, 5, true), Some(0));
        assert_eq!(next_match(&m, 0, true), Some(2));
    }

    #[test]
    fn replaces_all() {
        let (e, n) = replace_all("a b a", "a", "xy", FindOptions::default(), Selection::cursor(5)).expect("some");
        assert_eq!(n, 2);
        let mut s = "a b a".to_string();
        for c in e.changes.iter().rev() {
            s.replace_range(c.from..c.to, &c.insert);
        }
        assert_eq!(s, "xy b xy");
    }
}
