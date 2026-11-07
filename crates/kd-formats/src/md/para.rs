//! Paragraph formatting: alignment, line spacing, spacing before and after, indents, shading,
//! text direction, borders, padding and page breaks.
//!
//! It is kept as inline CSS on a `<div>` around the paragraphs, so other Markdown viewers and
//! the web page export show it too:
//!
//! ```markdown
//! <div style="text-align: center; line-height: 1.8; margin-top: 0.28cm">
//!
//! The paragraph, still **Markdown** inside.
//!
//! </div>
//! ```

use std::ops::Range;

use kd_doc::{Change, Selection};

use super::Edit;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ParaAlign {
    Left,
    Center,
    Right,
    Justify,
}

impl ParaAlign {
    pub fn css(self) -> &'static str {
        match self {
            ParaAlign::Left => "left",
            ParaAlign::Center => "center",
            ParaAlign::Right => "right",
            ParaAlign::Justify => "justify",
        }
    }
}

/// Line spacing as Word shows it.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum LineSpacing {
    /// 1.0 = single, 1.5, 2.0 … (CSS `line-height` is 1.2 × this: single spacing in Word is
    /// about 1.2 lines of the font size).
    Multiple(f32),
    /// Exactly this many points.
    Exactly(f32),
}

pub const SINGLE: f32 = 1.2;

/// The first line's indent.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Special {
    /// The first line further in by this many cm.
    FirstLine(f32),
    /// The other lines further in by this many cm.
    Hanging(f32),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BorderStyle {
    Solid,
    Dashed,
    Dotted,
    Double,
}

impl BorderStyle {
    pub fn css(self) -> &'static str {
        match self {
            BorderStyle::Solid => "solid",
            BorderStyle::Dashed => "dashed",
            BorderStyle::Dotted => "dotted",
            BorderStyle::Double => "double",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Border {
    /// Points.
    pub width: f32,
    pub style: BorderStyle,
    pub color: u32,
}

/// A paragraph's formatting. Lengths are centimetres.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct ParaStyle {
    pub align: Option<ParaAlign>,
    pub line: Option<LineSpacing>,
    pub before: Option<f32>,
    pub after: Option<f32>,
    pub left: Option<f32>,
    pub right: Option<f32>,
    pub special: Option<Special>,
    /// Shading (`RRGGBB`).
    pub background: Option<u32>,
    /// Right to left.
    pub rtl: bool,
    pub border: Option<Border>,
    /// Padding inside the border or shading.
    pub padding: Option<f32>,
    pub page_break_before: bool,
    pub keep_with_next: bool,
    pub keep_together: bool,
    /// CSS this does not know, kept as it was.
    pub other: Vec<(String, String)>,
}

/// A length in CSS → cm.
pub fn css_cm(v: &str) -> Option<f32> {
    let v = v.trim();
    let num_end = v.find(|c: char| !(c.is_ascii_digit() || c == '.' || c == '-')).unwrap_or(v.len());
    let n: f32 = v[..num_end].parse().ok()?;
    Some(match v[num_end..].trim() {
        "cm" => n,
        "mm" => n / 10.0,
        "in" => n * 2.54,
        "pt" => n * 2.54 / 72.0,
        "px" | "" => n * 2.54 / 96.0,
        "em" | "rem" => n * 12.0 * 2.54 / 72.0,
        _ => return None,
    })
}

fn cm(v: f32) -> String {
    let r = (v * 100.0).round() / 100.0;
    format!("{r}cm")
}

/// `#rgb`, `#rrggbb` → `RRGGBB`.
pub fn css_color(v: &str) -> Option<u32> {
    let h = v.trim().strip_prefix('#')?;
    match h.len() {
        6 => u32::from_str_radix(h, 16).ok(),
        3 => {
            let n = u32::from_str_radix(h, 16).ok()?;
            let (r, g, b) = ((n >> 8) & 15, (n >> 4) & 15, n & 15);
            Some(((r * 17) << 16) | ((g * 17) << 8) | (b * 17))
        }
        _ => None,
    }
}

impl ParaStyle {
    pub fn is_empty(&self) -> bool {
        *self == ParaStyle::default()
    }

    /// From a `style` attribute.
    pub fn from_css(css: &str) -> ParaStyle {
        let mut s = ParaStyle::default();
        for decl in css.split(';') {
            let Some((k, v)) = decl.split_once(':') else { continue };
            let (k, v) = (k.trim().to_ascii_lowercase(), v.trim());
            let known = match k.as_str() {
                "text-align" => {
                    s.align = match v {
                        "left" | "start" => Some(ParaAlign::Left),
                        "center" => Some(ParaAlign::Center),
                        "right" | "end" => Some(ParaAlign::Right),
                        "justify" => Some(ParaAlign::Justify),
                        _ => None,
                    };
                    s.align.is_some()
                }
                "line-height" => {
                    s.line = match v.parse::<f32>() {
                        Ok(n) => Some(LineSpacing::Multiple(((n / SINGLE) * 100.0).round() / 100.0)),
                        Err(_) => css_cm(v).map(|c| LineSpacing::Exactly((c / 2.54 * 72.0 * 10.0).round() / 10.0)),
                    };
                    s.line.is_some()
                }
                "margin-top" => set(&mut s.before, v),
                "margin-bottom" => set(&mut s.after, v),
                "margin-left" => set(&mut s.left, v),
                "margin-right" => set(&mut s.right, v),
                "text-indent" => match css_cm(v) {
                    Some(n) if n < 0.0 => {
                        s.special = Some(Special::Hanging(-n));
                        true
                    }
                    Some(n) => {
                        s.special = Some(Special::FirstLine(n));
                        true
                    }
                    None => false,
                },
                "background" | "background-color" => {
                    s.background = css_color(v);
                    s.background.is_some()
                }
                "direction" => {
                    s.rtl = v == "rtl";
                    true
                }
                "padding" => set(&mut s.padding, v),
                "border" => {
                    // 1pt solid #000000
                    let mut b = Border { width: 1.0, style: BorderStyle::Solid, color: 0 };
                    for part in v.split_whitespace() {
                        if let Some(c) = css_color(part) {
                            b.color = c;
                        } else if let Some(w) = css_cm(part) {
                            b.width = (w / 2.54 * 72.0 * 10.0).round() / 10.0;
                        } else {
                            b.style = match part {
                                "dashed" => BorderStyle::Dashed,
                                "dotted" => BorderStyle::Dotted,
                                "double" => BorderStyle::Double,
                                _ => BorderStyle::Solid,
                            };
                        }
                    }
                    s.border = Some(b);
                    true
                }
                "break-before" | "page-break-before" => {
                    s.page_break_before = v == "page" || v == "always";
                    true
                }
                "break-after" | "page-break-after" => {
                    s.keep_with_next = v == "avoid";
                    true
                }
                "break-inside" | "page-break-inside" => {
                    s.keep_together = v == "avoid";
                    true
                }
                _ => false,
            };
            if !known {
                s.other.push((k, v.to_string()));
            }
        }
        // a hanging indent is written as a negative text-indent with a larger left margin
        if let Some(Special::Hanging(h)) = s.special {
            s.left = Some((s.left.unwrap_or(0.0) - h).max(0.0)).filter(|l| *l > 0.001);
        }
        s
    }

    /// As a `style` attribute.
    pub fn to_css(&self) -> String {
        let mut d: Vec<String> = Vec::new();
        if let Some(a) = self.align {
            d.push(format!("text-align: {}", a.css()));
        }
        match self.line {
            Some(LineSpacing::Multiple(m)) => d.push(format!("line-height: {}", ((m * SINGLE) * 100.0).round() / 100.0)),
            Some(LineSpacing::Exactly(pt)) => d.push(format!("line-height: {pt}pt")),
            None => {}
        }
        if let Some(v) = self.before {
            d.push(format!("margin-top: {}", cm(v)));
        }
        if let Some(v) = self.after {
            d.push(format!("margin-bottom: {}", cm(v)));
        }
        let hanging = match self.special {
            Some(Special::Hanging(h)) => h,
            _ => 0.0,
        };
        if self.left.is_some() || hanging > 0.0 {
            d.push(format!("margin-left: {}", cm(self.left.unwrap_or(0.0) + hanging)));
        }
        if let Some(v) = self.right {
            d.push(format!("margin-right: {}", cm(v)));
        }
        match self.special {
            Some(Special::FirstLine(v)) => d.push(format!("text-indent: {}", cm(v))),
            Some(Special::Hanging(v)) => d.push(format!("text-indent: -{}", cm(v))),
            None => {}
        }
        if let Some(c) = self.background {
            d.push(format!("background: #{c:06x}"));
        }
        if let Some(b) = self.border {
            d.push(format!("border: {}pt {} #{:06x}", b.width, b.style.css(), b.color));
        }
        if let Some(p) = self.padding {
            d.push(format!("padding: {}", cm(p)));
        } else if self.background.is_some() || self.border.is_some() {
            d.push("padding: 0.15cm 0.25cm".into());
        }
        if self.rtl {
            d.push("direction: rtl".into());
        }
        if self.page_break_before {
            d.push("break-before: page".into());
        }
        if self.keep_with_next {
            d.push("break-after: avoid".into());
        }
        if self.keep_together {
            d.push("break-inside: avoid".into());
        }
        for (k, v) in &self.other {
            if !(k == "padding" && (self.background.is_some() || self.border.is_some())) {
                d.push(format!("{k}: {v}"));
            }
        }
        d.join("; ")
    }

    /// The left margin of the paragraph's other lines (cm), whatever the first line does.
    pub fn left_total(&self) -> f32 {
        self.left.unwrap_or(0.0)
            + match self.special {
                Some(Special::Hanging(h)) => h,
                _ => 0.0,
            }
    }
}

fn set(field: &mut Option<f32>, v: &str) -> bool {
    *field = css_cm(v);
    field.is_some()
}

/// `<div style="…">` on its own: the style.
pub fn div_style(html: &str) -> Option<ParaStyle> {
    let t = html.trim();
    if !t.starts_with("<div") || !t.ends_with('>') || t.contains('\n') || t.ends_with("/>") {
        return None;
    }
    let lower = t.to_ascii_lowercase();
    let p = lower.find("style=")?;
    let rest = &t[p + 6..];
    let q = rest.chars().next()?;
    if q != '"' && q != '\'' {
        return None;
    }
    let end = rest[1..].find(q)?;
    // other attributes than style: not ours
    let others = lower[4..p].trim();
    if !others.is_empty() {
        return None;
    }
    Some(ParaStyle::from_css(&rest[1..1 + end]))
}

/// A formatted part of the text: the `<div>` line, the `</div>` line and the style.
#[derive(Debug, Clone)]
struct Wrapper {
    /// Byte range of the `<div …>` line (without its newline).
    open: Range<usize>,
    /// Byte range of the `</div>` line.
    close: Range<usize>,
    style: ParaStyle,
}

/// The lines of `text` with their byte ranges (without newline).
fn lines(text: &str) -> Vec<Range<usize>> {
    let mut out = Vec::new();
    let mut start = 0;
    for (i, c) in text.char_indices() {
        if c == '\n' {
            out.push(start..i);
            start = i + 1;
        }
    }
    out.push(start..text.len());
    out
}

/// Every `<div style>` … `</div>` of the text (outside code blocks).
fn wrappers(text: &str) -> Vec<Wrapper> {
    let mut out = Vec::new();
    let mut stack: Vec<Option<(Range<usize>, ParaStyle)>> = Vec::new();
    let mut fence: Option<&str> = None;
    for r in lines(text) {
        let t = text[r.clone()].trim();
        if let Some(f) = fence {
            if t.starts_with(f) {
                fence = None;
            }
            continue;
        }
        if t.starts_with("```") || t.starts_with("~~~") {
            fence = Some(&t[..3]);
            continue;
        }
        if t.starts_with("<div") && t.ends_with('>') {
            stack.push(div_style(t).map(|s| (r.clone(), s)));
        } else if t == "</div>"
            && let Some(top) = stack.pop()
            && let Some((open, style)) = top
        {
            out.push(Wrapper { open, close: r.clone(), style });
        }
    }
    out
}

/// The paragraphs the selection is in: from the blank line before its first line to the one
/// after its last (byte range of whole lines).
fn paragraph_span(text: &str, sel: Range<usize>) -> Range<usize> {
    let ls = lines(text);
    let first = ls.iter().position(|l| l.end >= sel.start).unwrap_or(0);
    let last = ls.iter().position(|l| l.end >= sel.end).unwrap_or(ls.len() - 1);
    let blank = |i: usize| text[ls[i].clone()].trim().is_empty();
    // on a blank line or a <div>/</div> line: no paragraph
    if first == last && (blank(first) || is_div_line(&text[ls[first].clone()])) {
        return ls[first].start..ls[first].start;
    }
    let mut a = first;
    while a > 0 && !blank(a - 1) && !is_div_line(&text[ls[a - 1].clone()]) {
        a -= 1;
    }
    let mut b = last.max(a);
    while b + 1 < ls.len() && !blank(b + 1) && !is_div_line(&text[ls[b + 1].clone()]) {
        b += 1;
    }
    ls[a].start..ls[b].end
}

fn is_div_line(l: &str) -> bool {
    let t = l.trim();
    (t.starts_with("<div") && t.ends_with('>')) || t == "</div>"
}

/// The formatting of the paragraph at `pos` (the innermost `<div style>` around it).
pub fn style_at(text: &str, pos: usize) -> ParaStyle {
    innermost(text, pos..pos).map(|w| w.style).unwrap_or_default()
}

fn innermost(text: &str, sel: Range<usize>) -> Option<Wrapper> {
    let span = paragraph_span(text, sel);
    wrappers(text).into_iter().filter(|w| w.open.end < span.start && w.close.start > span.end).min_by_key(|w| w.close.start - w.open.end)
}

/// Change the formatting of the paragraphs in `sel`: edits their `<div style>` or puts one
/// around them; removes it when nothing is left.
pub fn apply(text: &str, sel: Selection, f: impl FnOnce(&mut ParaStyle)) -> Option<Edit> {
    let range = sel.start()..sel.end();
    let span = paragraph_span(text, range);
    if text[span.clone()].trim().is_empty() || text[span.clone()].lines().all(|l| l.trim().is_empty() || is_div_line(l)) {
        return None;
    }
    match innermost(text, sel.start()..sel.end()) {
        Some(w) => {
            let mut style = w.style.clone();
            f(&mut style);
            if style == w.style {
                return None;
            }
            if style.is_empty() {
                // remove the div: its line and the blank line after it, the </div> and the blank before
                // up to the paragraphs' first character, and from after their last one to the
                // end of the </div> line: the reverse of wrapping
                let open_to = w.open.end + (text[w.open.end..].len() - text[w.open.end..].trim_start().len());
                let close_from = text[..w.close.start].trim_end().len().max(open_to);
                let removed = open_to - w.open.start;
                let changes = vec![Change::new(w.open.start, open_to, ""), Change::new(close_from, w.close.end, "")];
                let shift = |p: usize| if p >= open_to { p - removed } else { p.min(w.open.start) };
                return Some(Edit { changes, selection: Selection::new(shift(sel.anchor), shift(sel.head)) });
            }
            let line = format!("<div style=\"{}\">", style.to_css());
            let delta = line.len() as isize - w.open.len() as isize;
            let shift = |p: usize| if p > w.open.end { (p as isize + delta) as usize } else { p };
            Some(Edit {
                changes: vec![Change::new(w.open.start, w.open.end, line)],
                selection: Selection::new(shift(sel.anchor), shift(sel.head)),
            })
        }
        None => {
            let mut style = ParaStyle::default();
            f(&mut style);
            if style.is_empty() {
                return None;
            }
            let open = format!("<div style=\"{}\">\n\n", style.to_css());
            let close = "\n\n</div>".to_string();
            let n = open.len();
            Some(Edit {
                changes: vec![Change::insert(span.start, open), Change::insert(span.end, close)],
                selection: Selection::new(sel.anchor + n, sel.head + n),
            })
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn run(text: &str, at: usize, f: impl FnOnce(&mut ParaStyle)) -> String {
        let e = apply(text, Selection::cursor(at), f).expect("an edit");
        let mut d = kd_doc::Document::new(text);
        d.apply(e.changes, e.selection, kd_doc::EditKind::Other);
        d.text().to_string()
    }

    #[test]
    fn css_round_trip() {
        let s = ParaStyle {
            align: Some(ParaAlign::Center),
            line: Some(LineSpacing::Multiple(1.5)),
            before: Some(0.28),
            after: Some(0.14),
            special: Some(Special::Hanging(0.5)),
            left: Some(1.0),
            background: Some(0xfff2cc),
            rtl: true,
            ..ParaStyle::default()
        };
        let css = s.to_css();
        assert!(css.contains("line-height: 1.8") && css.contains("text-indent: -0.5cm") && css.contains("margin-left: 1.5cm"), "{css}");
        let back = ParaStyle::from_css(&css);
        assert_eq!(back.align, s.align);
        assert_eq!(back.line, s.line);
        assert_eq!(back.special, s.special);
        assert_eq!(back.left, Some(1.0));
        assert_eq!(back.background, s.background);
        assert!(back.rtl);
    }

    #[test]
    fn wraps_edits_and_unwraps() {
        let text = "# Title\n\nFirst line\nsecond line\n\nOther\n";
        let at = text.find("second").unwrap();
        let t1 = run(text, at, |s| s.align = Some(ParaAlign::Center));
        assert_eq!(t1, "# Title\n\n<div style=\"text-align: center\">\n\nFirst line\nsecond line\n\n</div>\n\nOther\n");
        assert_eq!(style_at(&t1, t1.find("second").unwrap()).align, Some(ParaAlign::Center));
        assert_eq!(style_at(&t1, t1.find("Other").unwrap()).align, None);
        let t2 = run(&t1, t1.find("First").unwrap(), |s| s.line = Some(LineSpacing::Multiple(2.0)));
        assert!(t2.contains("<div style=\"text-align: center; line-height: 2.4\">"), "{t2}");
        let t3 = run(&t2, t2.find("First").unwrap(), |s| *s = ParaStyle::default());
        assert_eq!(t3, text);
        // the </div> line and blank lines are not paragraphs
        assert!(apply(&t1, Selection::cursor(t1.find("</div>").unwrap() + 2), |s| s.rtl = true).is_none());
        assert!(apply(&t1, Selection::cursor(t1.find("\n\nOther").unwrap() + 1), |s| s.rtl = true).is_none());
        // the parser gives the paragraphs inside with their style
        let p = crate::md::parse(&t1);
        assert!(p.blocks.iter().any(
            |b| matches!(b, crate::md::Block::Styled { style, blocks, .. } if style.align == Some(ParaAlign::Center) && blocks.len() == 1)
        ));
    }
}
