//! Slide decks: a document with `<!-- slide … -->` separators (MdCraft's format). The content
//! before the first separator is a slide too, unless it holds only the front matter.
//!
//! ```markdown
//! <!-- slide class="title" data-background-color="#0b1220" -->
//!
//! # Title
//!
//! Note: speaker notes, to the end of the slide.
//! ```

use kd_doc::{Change, Selection};

use super::edit::{Edit, body_start, take_marks};

/// One slide: its byte range (from its separator line, or the body's start, up to the next
/// separator) and its separator's attributes.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Slide {
    pub start: usize,
    pub end: usize,
    /// The separator line (without its newline), when the slide has one.
    pub marker: Option<(usize, usize)>,
    pub attrs: Vec<(String, String)>,
}

impl Slide {
    pub fn attr(&self, key: &str) -> Option<&str> {
        self.attrs.iter().find(|(k, _)| k == key).map(|(_, v)| v.as_str())
    }

    /// The layout class (`title`, `section`, `two-col` …); "" for title and content.
    pub fn class(&self) -> &str {
        self.attr("class").unwrap_or("")
    }

    /// The slide's Markdown, without its separator.
    pub fn body<'a>(&self, text: &'a str) -> &'a str {
        let from = self.marker.map(|(_, e)| (e + 1).min(self.end)).unwrap_or(self.start);
        &text[from..self.end]
    }

    /// The content and the speaker notes (after a line starting with `Note:`).
    pub fn content_and_notes<'a>(&self, text: &'a str) -> (&'a str, Option<&'a str>) {
        split_notes(self.body(text))
    }
}

/// `body` split at its `Note:` line.
pub fn split_notes(body: &str) -> (&str, Option<&str>) {
    let mut at = 0;
    let mut fence = false;
    for line in body.split_inclusive('\n') {
        let t = line.trim_start();
        if t.starts_with("```") || t.starts_with("~~~") {
            fence = !fence;
        } else if !fence && (t.starts_with("Note:") || t.starts_with("Notes:")) {
            let notes = t.split_once(':').map(|(_, n)| n).unwrap_or("");
            let rest = &body[at + line.len()..];
            let n = &body[at + (line.len() - notes.len())..at + line.len() + rest.len()];
            return (&body[..at], Some(n.trim()).filter(|n| !n.is_empty()));
        }
        at += line.len();
    }
    (body, None)
}

/// Whether `line` is a slide separator.
pub fn is_marker(line: &str) -> bool {
    let t = line.trim();
    t.starts_with("<!--") && t[4..].trim_start().starts_with("slide") && t.ends_with("-->")
}

/// `key="value"` pairs of a separator.
pub fn marker_attrs(line: &str) -> Vec<(String, String)> {
    let t = line.trim();
    let inner = t.trim_start_matches("<!--").trim_end_matches("-->").trim();
    let inner = inner.strip_prefix("slide").unwrap_or(inner);
    let mut out = Vec::new();
    let mut rest = inner.trim();
    while let Some(eq) = rest.find('=') {
        let key = rest[..eq].trim().to_string();
        let after = rest[eq + 1..].trim_start();
        let (value, next) = if let Some(r) = after.strip_prefix('"') {
            match r.find('"') {
                Some(q) => (r[..q].to_string(), &r[q + 1..]),
                None => (r.to_string(), ""),
            }
        } else {
            let end = after.find(char::is_whitespace).unwrap_or(after.len());
            (after[..end].to_string(), &after[end..])
        };
        if !key.is_empty() {
            out.push((key, value));
        }
        rest = next.trim_start();
    }
    out
}

/// A separator line with these attributes.
pub fn marker_line(attrs: &[(String, String)]) -> String {
    let a: Vec<String> = attrs.iter().filter(|(_, v)| !v.is_empty()).map(|(k, v)| format!("{k}=\"{v}\"")).collect();
    if a.is_empty() { "<!-- slide -->".into() } else { format!("<!-- slide {} -->", a.join(" ")) }
}

/// The slides of `text`; empty when it has no separator (it is not a deck).
pub fn slides(text: &str) -> Vec<Slide> {
    let mut markers: Vec<(usize, usize)> = Vec::new();
    let mut fence = false;
    let mut pos = 0;
    for line in text.split_inclusive('\n') {
        let l = line.trim_end_matches(['\n', '\r']);
        let t = l.trim_start();
        if t.starts_with("```") || t.starts_with("~~~") {
            fence = !fence;
        } else if !fence && is_marker(l) {
            markers.push((pos, pos + l.len()));
        }
        pos += line.len();
    }
    if markers.is_empty() {
        return Vec::new();
    }
    let mut out = Vec::new();
    let mut body = body_start(text).min(markers[0].0);
    let lead = &text[body..markers[0].0];
    body += lead.len() - lead.trim_start_matches(['\n', '\r']).len();
    if !text[body..markers[0].0].trim().is_empty() {
        out.push(Slide { start: body, end: markers[0].0, marker: None, attrs: Vec::new() });
    }
    for (i, m) in markers.iter().enumerate() {
        let end = markers.get(i + 1).map(|n| n.0).unwrap_or(text.len());
        out.push(Slide { start: m.0, end, marker: Some(*m), attrs: marker_attrs(&text[m.0..m.1]) });
    }
    out
}

/// The slide the cursor is on.
pub fn current(slides: &[Slide], pos: usize) -> Option<usize> {
    if slides.is_empty() {
        return None;
    }
    Some(slides.iter().rposition(|s| s.start <= pos).unwrap_or(0))
}

/// Set (or with `None` remove) an attribute of the current slide's separator.
pub fn set_attr(text: &str, sel: Selection, key: &str, value: Option<&str>) -> Option<Edit> {
    let all = slides(text);
    let s = &all[current(&all, sel.head)?];
    let mut attrs = s.attrs.clone();
    attrs.retain(|(k, _)| k != key);
    if let Some(v) = value.filter(|v| !v.is_empty()) {
        // the class first, as MdCraft writes it
        if key == "class" {
            attrs.insert(0, (key.into(), v.into()));
        } else {
            attrs.push((key.into(), v.into()));
        }
    }
    let line = marker_line(&attrs);
    let change = match s.marker {
        Some((a, b)) => Change::new(a, b, line),
        None => Change::insert(s.start, format!("{line}\n\n")),
    };
    let selection = super::edit::shift_selection(sel, std::slice::from_ref(&change));
    Some(Edit { changes: vec![change], selection })
}

/// The slide as text with a separator, ending in exactly one blank line.
fn chunk(text: &str, s: &Slide) -> String {
    let body = &text[s.start..s.end];
    let body = if s.marker.is_none() { format!("<!-- slide -->\n\n{}", body.trim_start_matches('\n')) } else { body.to_string() };
    format!("{}\n\n", body.trim_end())
}

/// Insert a copy of the current slide after it.
pub fn duplicate(text: &str, sel: Selection) -> Option<Edit> {
    let all = slides(text);
    let s = &all[current(&all, sel.head)?];
    let copy = chunk(text, s);
    let at = s.end;
    let (prefix, at) =
        if at == text.len() && !text.ends_with("\n\n") { (if text.ends_with('\n') { "\n" } else { "\n\n" }, at) } else { ("", at) };
    let ins = format!("{prefix}{copy}");
    let cursor = at + prefix.len();
    Some(Edit { changes: vec![Change::insert(at, ins)], selection: Selection::cursor(cursor) })
}

/// Remove the current slide.
pub fn delete(text: &str, sel: Selection) -> Option<Edit> {
    let all = slides(text);
    let i = current(&all, sel.head)?;
    let s = &all[i];
    let cursor = all.get(i.wrapping_sub(1)).map(|p| p.start).unwrap_or(s.start);
    Some(Edit { changes: vec![Change::new(s.start, s.end, "")], selection: Selection::cursor(cursor.min(s.start)) })
}

/// Move the current slide up (`-1`) or down (`1`).
pub fn move_slide(text: &str, sel: Selection, dir: i32) -> Option<Edit> {
    let all = slides(text);
    let i = current(&all, sel.head)?;
    let j = if dir < 0 { i.checked_sub(1)? } else { i + 1 };
    let (a, b) = (i.min(j), i.max(j));
    let sb = all.get(b)?;
    let sa = &all[a];
    let last = sb.end == text.len();
    let mut swapped = format!("{}{}", chunk(text, sb), chunk(text, sa));
    if last {
        swapped = format!("{}\n", swapped.trim_end());
    }
    let new_cur = if dir < 0 { sa.start } else { sa.start + chunk(text, sb).len() };
    Some(Edit { changes: vec![Change::new(sa.start, sb.end, swapped)], selection: Selection::cursor(new_cur) })
}

/// Go to the current slide's notes, adding a `Note:` line when it has none.
pub fn add_notes(text: &str, sel: Selection) -> Option<Edit> {
    let all = slides(text);
    let s = &all[current(&all, sel.head)?];
    let body = &text[s.start..s.end];
    if let Some(i) = body.find("\nNote:") {
        let at = s.start + i + "\nNote: ".len();
        return Some(Edit { changes: vec![], selection: Selection::cursor(at.min(s.end)) });
    }
    let content_end = s.start + body.trim_end().len();
    let (ins, marks) = take_marks("\n\nNote: \u{1}Speaker notes\u{1}");
    let (a, b) = marks.unwrap_or((ins.len(), ins.len()));
    Some(Edit { changes: vec![Change::insert(content_end, ins)], selection: Selection::new(content_end + a, content_end + b) })
}

/// MdCraft's *Convert to slides*: a slide at every `##` heading (and every `#` after the
/// first), with a slide theme. `None` when the document already has slides.
pub fn from_headings(text: &str) -> Option<String> {
    if !slides(text).is_empty() {
        return None;
    }
    let body = body_start(text);
    let mut out = String::with_capacity(text.len() + 256);
    out.push_str(&text[..body]);
    let mut fence = false;
    let mut first_h1 = true;
    let mut any = false;
    for line in text[body..].split_inclusive('\n') {
        let t = line.trim_start();
        if t.starts_with("```") || t.starts_with("~~~") {
            fence = !fence;
        }
        if !fence {
            let h1 = t.starts_with("# ");
            let h2 = t.starts_with("## ");
            if h2 || (h1 && !first_h1) {
                if !out.ends_with("\n\n") && !out[body..].is_empty() {
                    out.push('\n');
                }
                out.push_str("<!-- slide -->\n\n");
                any = true;
            }
            if h1 {
                if first_h1 {
                    out.push_str("<!-- slide class=\"title\" -->\n\n");
                    any = true;
                }
                first_h1 = false;
            }
        }
        out.push_str(line);
    }
    if !any {
        return None;
    }
    // a slide theme unless one is set
    let theme = super::edit::front_matter(&out).into_iter().find(|(k, _)| k == "theme").map(|(_, v)| v);
    let deck_theme = theme.as_deref().and_then(super::library::theme).is_some_and(|t| t.deck);
    if !deck_theme {
        let e = super::edit::set_front_matter(&out, Selection::cursor(0), &[("theme", Some("paper"))]);
        let mut s = out.clone();
        for c in e.changes.iter().rev() {
            s.replace_range(c.from..c.to, &c.insert);
        }
        out = s;
    }
    Some(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn apply(text: &str, e: &Edit) -> String {
        let mut s = text.to_string();
        for c in e.changes.iter().rev() {
            s.replace_range(c.from..c.to, &c.insert);
        }
        s
    }

    const DECK: &str =
        "---\ntheme: paper\n---\n\n# Title\n\n<!-- slide class=\"section\" -->\n\n## Two\n\nNote: say hi\n\n<!-- slide -->\n\n## Three\n";

    #[test]
    fn slides_and_attrs() {
        let s = slides(DECK);
        assert_eq!(s.len(), 3);
        assert_eq!(s[0].marker, None);
        assert_eq!(s[1].class(), "section");
        assert_eq!(s[1].content_and_notes(DECK).1, Some("say hi"));
        assert!(slides("# No slides").is_empty());
        assert_eq!(
            marker_attrs("<!-- slide class=\"two-col\" data-background-color=\"#123\" -->"),
            vec![("class".into(), "two-col".into()), ("data-background-color".into(), "#123".into())]
        );
    }

    #[test]
    fn layout_and_background() {
        let at = DECK.find("## Three").expect("three");
        let e = set_attr(DECK, Selection::cursor(at), "class", Some("end")).expect("edit");
        assert!(apply(DECK, &e).contains("<!-- slide class=\"end\" -->\n\n## Three"));
        let e = set_attr(DECK, Selection::cursor(DECK.find("# Title").expect("t")), "data-background-color", Some("#111")).expect("edit");
        assert!(apply(DECK, &e).contains("---\n\n<!-- slide data-background-color=\"#111\" -->\n\n# Title"));
    }

    #[test]
    fn duplicate_delete_move() {
        let at = DECK.find("## Two").expect("two");
        let d = apply(DECK, &duplicate(DECK, Selection::cursor(at)).expect("dup"));
        assert_eq!(slides(&d).len(), 4);
        assert_eq!(d.matches("## Two").count(), 2);
        let x = apply(DECK, &delete(DECK, Selection::cursor(at)).expect("del"));
        assert_eq!(slides(&x).len(), 2);
        assert!(!x.contains("## Two"));
        let m = apply(DECK, &move_slide(DECK, Selection::cursor(at), 1).expect("move"));
        assert!(m.find("## Three").expect("3") < m.find("## Two").expect("2"));
        assert_eq!(slides(&m).len(), 3);
        let up = apply(DECK, &move_slide(DECK, Selection::cursor(at), -1).expect("up"));
        assert!(up.find("## Two").expect("2") < up.find("# Title").expect("1"));
        assert!(move_slide(DECK, Selection::cursor(DECK.len() - 2), 1).is_none());
    }

    #[test]
    fn notes_and_convert() {
        let at = DECK.find("## Three").expect("3");
        let n = add_notes(DECK, Selection::cursor(at)).expect("notes");
        assert!(apply(DECK, &n).ends_with("## Three\n\nNote: Speaker notes\n"));
        let c = from_headings("# Deck\n\nIntro\n\n## A\n\ntext\n\n## B\n").expect("convert");
        assert_eq!(slides(&c).len(), 3);
        assert!(c.contains("theme: paper"));
        assert!(from_headings(DECK).is_none());
    }
}
