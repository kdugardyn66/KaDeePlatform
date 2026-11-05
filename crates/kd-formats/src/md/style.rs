//! Fonts and colours for part of a document, as MdCraft writes them: a
//! `<span style="font-family: Arial; color: #C00000">` around the words.

use kd_doc::{Change, Selection};

use super::edit::{Edit, word_at};

const OPEN: &str = "<span style=\"";

/// The span around `from..to` on one line: `(open_start, open_end, close_start, style)`.
fn span_around(text: &str, from: usize, to: usize) -> Option<(usize, usize, usize, String)> {
    let line_start = text[..from].rfind('\n').map(|i| i + 1).unwrap_or(0);
    let line_end = text[to..].find('\n').map(|i| to + i).unwrap_or(text.len());
    let mut search = from;
    while let Some(i) = text[line_start..search].rfind(OPEN) {
        let open = line_start + i;
        let style_start = open + OPEN.len();
        let q = text[style_start..line_end].find('"')? + style_start;
        let open_end = text[q..line_end].find('>')? + q + 1;
        if open_end <= from
            && let Some(c) = text[open_end..line_end].find("</span>")
        {
            let close = open_end + c;
            if close >= to {
                return Some((open, open_end, close, text[style_start..q].to_string()));
            }
        }
        search = open;
    }
    None
}

fn parse_style(style: &str) -> Vec<(String, String)> {
    style
        .split(';')
        .filter_map(|d| d.split_once(':'))
        .map(|(k, v)| (k.trim().to_lowercase(), v.trim().to_string()))
        .filter(|(k, v)| !k.is_empty() && !v.is_empty())
        .collect()
}

fn write_style(decls: &[(String, String)]) -> String {
    decls.iter().map(|(k, v)| format!("{k}: {v}")).collect::<Vec<_>>().join("; ")
}

/// The style properties around the cursor (`font-family`, `font-size`, `color` …).
pub fn style_at(text: &str, pos: usize) -> Vec<(String, String)> {
    span_around(text, pos, pos).map(|(_, _, _, s)| parse_style(&s)).unwrap_or_default()
}

/// One property around the cursor.
pub fn style_value(text: &str, pos: usize, prop: &str) -> Option<String> {
    style_at(text, pos).into_iter().find(|(k, _)| k == prop).map(|(_, v)| v)
}

/// Set (or with `None` remove) a style property on the selection (or the word at the cursor):
/// changes the span around it when there is one, else wraps it in a new span.
pub fn apply_style(text: &str, sel: Selection, prop: &str, value: Option<&str>) -> Option<Edit> {
    let (mut from, mut to) = (sel.start(), sel.end());
    if let Some((open, open_end, close, style)) = span_around(text, from, to) {
        let mut decls = parse_style(&style);
        decls.retain(|(k, _)| k != prop);
        if let Some(v) = value {
            decls.push((prop.to_string(), v.to_string()));
        }
        let (changes, shift_open) = if decls.is_empty() {
            (vec![Change::new(open, open_end, ""), Change::new(close, close + "</span>".len(), "")], open_end - open)
        } else {
            let tag = format!("{OPEN}{}\">", write_style(&decls));
            let delta = tag.len() as isize - (open_end - open) as isize;
            (vec![Change::new(open, open_end, tag)], (-delta) as usize)
        };
        let back = |p: usize| p.wrapping_sub(shift_open);
        let selection = Selection::new(back(sel.anchor), back(sel.head));
        return Some(Edit { changes, selection });
    }
    let value = value?;
    if from == to
        && let Some((a, b)) = word_at(text, from)
    {
        from = a;
        to = b;
    }
    let tag = format!("{OPEN}{prop}: {value}\">");
    let inner = if from == to { "text" } else { &text[from..to] };
    let ins = format!("{tag}{inner}</span>");
    let start = from + tag.len();
    Some(Edit { changes: vec![Change::new(from, to, ins)], selection: Selection::new(start, start + inner.len()) })
}

/// MdCraft's font sizes (points).
pub const SIZES: [f32; 17] = [8.0, 9.0, 10.0, 10.5, 11.0, 12.0, 14.0, 16.0, 18.0, 20.0, 22.0, 24.0, 26.0, 28.0, 36.0, 48.0, 72.0];

/// The next size up or down from `cur` points.
pub fn step_size(cur: f32, up: bool) -> f32 {
    if up {
        SIZES.iter().copied().find(|s| *s > cur).unwrap_or(96.0)
    } else {
        SIZES.iter().rev().copied().find(|s| *s < cur).unwrap_or(6.0)
    }
}

/// `"12pt"` → 12.0.
pub fn points(v: &str) -> Option<f32> {
    v.trim().trim_end_matches("pt").trim().parse().ok()
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

    #[test]
    fn wraps_then_changes_then_removes() {
        let t = "Some red words";
        let e = apply_style(t, Selection::new(5, 8), "color", Some("#C00000")).expect("wrap");
        let t2 = apply(t, &e);
        assert_eq!(t2, "Some <span style=\"color: #C00000\">red</span> words");
        assert_eq!(&t2[e.selection.range()], "red");
        let pos = t2.find("red").expect("red");
        assert_eq!(style_value(&t2, pos + 1, "color").as_deref(), Some("#C00000"));
        let e = apply_style(&t2, Selection::cursor(pos + 1), "font-size", Some("14pt")).expect("add");
        let t3 = apply(&t2, &e);
        assert_eq!(t3, "Some <span style=\"color: #C00000; font-size: 14pt\">red</span> words");
        let pos = t3.find("red").expect("red");
        let t4 = apply(&t3, &apply_style(&t3, Selection::cursor(pos), "color", None).expect("rm"));
        let pos = t4.find("red").expect("red");
        let t5 = apply(&t4, &apply_style(&t4, Selection::cursor(pos), "font-size", None).expect("rm"));
        assert_eq!(t5, t);
        assert!(apply_style(t, Selection::cursor(0), "color", None).is_none());
    }

    #[test]
    fn sizes_step() {
        assert_eq!(step_size(12.0, true), 14.0);
        assert_eq!(step_size(12.0, false), 11.0);
        assert_eq!(points("10.5pt"), Some(10.5));
    }
}
