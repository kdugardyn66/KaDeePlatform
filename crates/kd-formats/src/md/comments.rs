//! Comments in a Markdown document, kept as HTML comments right after the text they are about:
//!
//! ```markdown
//! The budget grows by 4 %<!-- comment (Kris Dugardyn, 2026-11-07 21:10): Check with finance -->.
//! ```
//!
//! Other Markdown viewers and every export leave them out, so the document reads the same.

use std::ops::Range;

const OPEN: &str = "<!-- comment (";
const CLOSE: &str = "-->";

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Comment {
    /// The whole `<!-- comment … -->` in the text.
    pub range: Range<usize>,
    /// 1-based line of the comment.
    pub line: usize,
    pub author: String,
    pub date: String,
    pub text: String,
    /// The text just before the comment on its line (what it is about).
    pub context: String,
}

/// Every comment in `text`, in order.
pub fn comments(text: &str) -> Vec<Comment> {
    let mut out = Vec::new();
    let mut from = 0;
    while let Some(p) = text[from..].find(OPEN).map(|p| p + from) {
        let Some(end) = text[p..].find(CLOSE).map(|e| p + e + CLOSE.len()) else { break };
        from = end;
        let inner = &text[p + OPEN.len()..end - CLOSE.len()];
        let Some((who, body)) = inner.split_once("):") else { continue };
        let (author, date) = match who.rsplit_once(',') {
            Some((a, d)) => (a.trim().to_string(), d.trim().to_string()),
            None => (who.trim().to_string(), String::new()),
        };
        let line_start = text[..p].rfind('\n').map(|i| i + 1).unwrap_or(0);
        let before = strip_comments(&text[line_start..p]);
        let context = tail(&plain(&before), 70);
        out.push(Comment { range: p..end, line: text[..p].matches('\n').count() + 1, author, date, text: unescape(body.trim()), context });
    }
    out
}

/// The text of a new comment.
pub fn marker(author: &str, date: &str, text: &str) -> String {
    let author = if author.trim().is_empty() { "Me" } else { author.trim() };
    let author = author.replace([',', '(', ')'], " ");
    format!("{OPEN}{author}, {date}): {} {CLOSE}", escape(text.trim()))
}

/// The document without its comments (for exports, so the HTML source has none either).
pub fn strip(text: &str) -> std::borrow::Cow<'_, str> {
    if text.contains(OPEN) { std::borrow::Cow::Owned(strip_comments(text)) } else { std::borrow::Cow::Borrowed(text) }
}

/// The text without any comments.
fn strip_comments(s: &str) -> String {
    let mut out = String::new();
    let mut rest = s;
    while let Some(p) = rest.find(OPEN) {
        out.push_str(&rest[..p]);
        match rest[p..].find(CLOSE) {
            Some(e) => rest = &rest[p + e + CLOSE.len()..],
            None => {
                rest = "";
            }
        }
    }
    out.push_str(rest);
    out
}

/// Markdown markers out of a line, roughly (`**`, `#`, `>`, list bullets, links' addresses).
fn plain(s: &str) -> String {
    let mut t = s.trim_start().trim_start_matches('#').trim_start_matches('>').trim_start().to_string();
    for p in ["- [ ] ", "- [x] ", "- ", "* ", "+ "] {
        if let Some(r) = t.strip_prefix(p) {
            t = r.to_string();
            break;
        }
    }
    // [text](address) → text
    let mut out = String::new();
    let mut chars = t.chars().peekable();
    let mut skip_paren = false;
    while let Some(c) = chars.next() {
        match c {
            ']' if chars.peek() == Some(&'(') => skip_paren = true,
            '(' if skip_paren => {
                for c in chars.by_ref() {
                    if c == ')' {
                        break;
                    }
                }
                skip_paren = false;
            }
            '*' | '_' | '`' | '[' | ']' => {}
            c => out.push(c),
        }
    }
    out.trim().to_string()
}

/// The last `n` characters, cut at a word.
fn tail(s: &str, n: usize) -> String {
    let count = s.chars().count();
    if count <= n {
        return s.to_string();
    }
    let cut: String = s.chars().skip(count - n).collect();
    let cut = cut.split_once(' ').map(|(_, r)| r).unwrap_or(&cut);
    format!("…{cut}")
}

/// `-->` cannot be inside an HTML comment.
fn escape(s: &str) -> String {
    s.replace("-->", "--&gt;").replace('\n', " ")
}

fn unescape(s: &str) -> String {
    s.replace("--&gt;", "-->")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn finds_and_writes_comments() {
        let m = marker("Kris Dugardyn", "2026-11-07 21:10", "Check with --> finance");
        let text = format!("# Plan\n\nThe **budget** grows by 4 %{m}.\n\nOther{}\n", marker("", "2026-11-08", "Second"));
        let cs = comments(&text);
        assert_eq!(cs.len(), 2);
        assert_eq!(cs[0].author, "Kris Dugardyn");
        assert_eq!(cs[0].date, "2026-11-07 21:10");
        assert_eq!(cs[0].text, "Check with --> finance");
        assert_eq!(cs[0].context, "The budget grows by 4 %");
        assert_eq!(cs[0].line, 3);
        assert_eq!(&text[cs[0].range.clone()], m);
        assert_eq!(cs[1].author, "Me");
        // comments leave the document as it was
        let html = crate::md::html::to_html(&text, &crate::md::html::HtmlOptions::default());
        assert!(!html.contains("finance") && !html.contains("Second"));
    }
}
