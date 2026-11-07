//! How tables and pictures look, kept in the Markdown:
//!
//! - a table's look on a `<div data-table="…">` around it (`no-header; total; banded-rows;
//!   first-column; template: blue; border: all 1pt #4472c4; fill: #ffffff; width: 14cm;
//!   row-height: 0.8cm; fit; keep-rows`), so the table itself stays a plain Markdown table;
//! - a picture's size, opacity, rotation, flip, wrapping and crop as CSS on an `<img>`
//!   (`<img src="a.png" alt="…" style="width: 8cm; transform: rotate(90deg); float: left">`),
//!   which other viewers and the web page export show too.

use std::ops::Range;

use kd_doc::{Change, Selection};

use super::Edit;
use super::para::{css_cm, css_color};

// ---- tables -----------------------------------------------------------------------------------

/// Ready-made table looks: (id, name, header background, header text, band, border, total).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct TableTemplate {
    pub id: &'static str,
    pub name: &'static str,
    pub header_bg: Option<u32>,
    pub header_fg: Option<u32>,
    pub band: u32,
    pub border: u32,
    /// The first and last columns when they stand out.
    pub accent_bg: Option<u32>,
}

pub const TEMPLATES: [TableTemplate; 8] = [
    TableTemplate { id: "plain", name: "Plain", header_bg: None, header_fg: None, band: 0xF6F8FA, border: 0xD0D7DE, accent_bg: None },
    TableTemplate {
        id: "grid",
        name: "Grid",
        header_bg: Some(0xF2F2F2),
        header_fg: None,
        band: 0xF2F2F2,
        border: 0x000000,
        accent_bg: Some(0xF2F2F2),
    },
    TableTemplate {
        id: "light",
        name: "Light",
        header_bg: None,
        header_fg: Some(0x2F5496),
        band: 0xEEF3FA,
        border: 0x8EAADB,
        accent_bg: None,
    },
    TableTemplate {
        id: "blue",
        name: "Blue",
        header_bg: Some(0x4472C4),
        header_fg: Some(0xFFFFFF),
        band: 0xD9E2F3,
        border: 0x8EAADB,
        accent_bg: Some(0xB4C6E7),
    },
    TableTemplate {
        id: "green",
        name: "Green",
        header_bg: Some(0x70AD47),
        header_fg: Some(0xFFFFFF),
        band: 0xE2EFDA,
        border: 0xA9D08E,
        accent_bg: Some(0xC5E0B3),
    },
    TableTemplate {
        id: "orange",
        name: "Orange",
        header_bg: Some(0xED7D31),
        header_fg: Some(0xFFFFFF),
        band: 0xFBE4D5,
        border: 0xF4B183,
        accent_bg: Some(0xF7CAAC),
    },
    TableTemplate {
        id: "gray",
        name: "Gray",
        header_bg: Some(0x595959),
        header_fg: Some(0xFFFFFF),
        band: 0xEDEDED,
        border: 0xA5A5A5,
        accent_bg: Some(0xDBDBDB),
    },
    TableTemplate {
        id: "dark",
        name: "Dark",
        header_bg: Some(0x1F3864),
        header_fg: Some(0xFFFFFF),
        band: 0xDEEAF6,
        border: 0x1F3864,
        accent_bg: Some(0xBDD6EE),
    },
];

pub fn template(id: &str) -> Option<&'static TableTemplate> {
    TEMPLATES.iter().find(|t| t.id == id)
}

/// Which lines a table's border draws.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Borders {
    All,
    Outer,
    Inner,
    /// Lines between the rows, and above and below the table.
    Horizontal,
    None,
}

impl Borders {
    pub fn css(self) -> &'static str {
        match self {
            Borders::All => "all",
            Borders::Outer => "outer",
            Borders::Inner => "inner",
            Borders::Horizontal => "horizontal",
            Borders::None => "none",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct TableBorder {
    pub which: Borders,
    /// Points.
    pub width: f32,
    pub color: u32,
}

#[derive(Debug, Clone, PartialEq)]
pub struct TableLook {
    pub header: bool,
    pub total: bool,
    pub banded_rows: bool,
    pub first_col: bool,
    pub last_col: bool,
    pub banded_cols: bool,
    pub template: Option<String>,
    pub border: Option<TableBorder>,
    /// The cells' background.
    pub fill: Option<u32>,
    /// The table's width (cm); `fit` makes it as wide as its content instead.
    pub width: Option<f32>,
    pub row_height: Option<f32>,
    pub fit: bool,
    /// Rows do not break across pages.
    pub keep_rows: bool,
}

impl Default for TableLook {
    fn default() -> Self {
        TableLook {
            header: true,
            total: false,
            banded_rows: false,
            first_col: false,
            last_col: false,
            banded_cols: false,
            template: None,
            border: None,
            fill: None,
            width: None,
            row_height: None,
            fit: false,
            keep_rows: false,
        }
    }
}

/// How one cell looks.
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct CellLook {
    pub bg: Option<u32>,
    pub fg: Option<u32>,
    pub bold: bool,
}

impl TableLook {
    pub fn is_default(&self) -> bool {
        *self == TableLook::default()
    }

    /// From a `data-table` attribute.
    pub fn parse(attr: &str) -> TableLook {
        let mut t = TableLook::default();
        for part in attr.split(';').map(str::trim).filter(|p| !p.is_empty()) {
            match part.split_once(':') {
                Some((k, v)) => {
                    let v = v.trim();
                    match k.trim() {
                        "template" => t.template = Some(v.to_string()),
                        "fill" => t.fill = css_color(v),
                        "width" => t.width = css_cm(v),
                        "row-height" => t.row_height = css_cm(v),
                        "border" => {
                            let mut b = TableBorder { which: Borders::All, width: 1.0, color: 0xD0D7DE };
                            for w in v.split_whitespace() {
                                match w {
                                    "all" => b.which = Borders::All,
                                    "outer" => b.which = Borders::Outer,
                                    "inner" => b.which = Borders::Inner,
                                    "horizontal" => b.which = Borders::Horizontal,
                                    "none" => b.which = Borders::None,
                                    w => {
                                        if let Some(c) = css_color(w) {
                                            b.color = c;
                                        } else if let Some(cm) = css_cm(w) {
                                            b.width = (cm / 2.54 * 72.0 * 10.0).round() / 10.0;
                                        }
                                    }
                                }
                            }
                            t.border = Some(b);
                        }
                        _ => {}
                    }
                }
                None => match part {
                    "no-header" => t.header = false,
                    "total" => t.total = true,
                    "banded-rows" => t.banded_rows = true,
                    "first-column" => t.first_col = true,
                    "last-column" => t.last_col = true,
                    "banded-columns" => t.banded_cols = true,
                    "fit" => t.fit = true,
                    "keep-rows" => t.keep_rows = true,
                    _ => {}
                },
            }
        }
        t
    }

    /// As a `data-table` attribute.
    pub fn to_attr(&self) -> String {
        let mut v: Vec<String> = Vec::new();
        if !self.header {
            v.push("no-header".into());
        }
        for (on, name) in [
            (self.total, "total"),
            (self.banded_rows, "banded-rows"),
            (self.first_col, "first-column"),
            (self.last_col, "last-column"),
            (self.banded_cols, "banded-columns"),
            (self.fit, "fit"),
            (self.keep_rows, "keep-rows"),
        ] {
            if on {
                v.push(name.into());
            }
        }
        if let Some(t) = &self.template {
            v.push(format!("template: {t}"));
        }
        if let Some(b) = self.border {
            v.push(format!("border: {} {}pt #{:06x}", b.which.css(), b.width, b.color));
        }
        if let Some(c) = self.fill {
            v.push(format!("fill: #{c:06x}"));
        }
        if let Some(w) = self.width {
            v.push(format!("width: {}cm", (w * 100.0).round() / 100.0));
        }
        if let Some(h) = self.row_height {
            v.push(format!("row-height: {}cm", (h * 100.0).round() / 100.0));
        }
        v.join("; ")
    }

    pub fn tpl(&self) -> &'static TableTemplate {
        self.template.as_deref().and_then(template).unwrap_or(&TEMPLATES[0])
    }

    /// The look of the cell at row `r` (0 = the header row) and column `c` of a table with
    /// `rows` rows (header included) and `cols` columns.
    pub fn cell(&self, r: usize, c: usize, rows: usize, cols: usize) -> CellLook {
        let t = self.tpl();
        let mut out = CellLook { bg: self.fill, ..CellLook::default() };
        let body_r = r.saturating_sub(1);
        if self.banded_rows && r > 0 && body_r % 2 == 1 {
            out.bg = Some(t.band);
        }
        if self.banded_cols && c % 2 == 1 && !(self.header && r == 0) {
            out.bg = Some(t.band);
        }
        if (self.first_col && c == 0) || (self.last_col && c + 1 == cols) {
            out.bold = true;
            if let Some(a) = t.accent_bg {
                out.bg = Some(a);
            }
        }
        if self.total && r + 1 == rows && r > 0 {
            out.bold = true;
            out.bg = t.accent_bg.or(out.bg);
        }
        if self.header && r == 0 {
            out.bold = true;
            out.bg = t.header_bg.or(self.fill);
            out.fg = t.header_fg;
        }
        out
    }

    /// Style rules for a table inside the element `sel` (the web page export), in the
    /// order `cell()` decides: fill, banded rows, banded columns, first and last column,
    /// total row, header row.
    pub fn css(&self, sel: &str) -> String {
        let t = self.tpl();
        let hex = |c: u32| format!("#{c:06x}");
        let mut css = String::new();
        let mut rule = |s: &str, d: &str| css.push_str(&format!("{sel} {s} {{ {d} }}\n"));
        let width = if self.fit {
            "width: auto".to_string()
        } else {
            self.width.map(|w| format!("width: {w}cm; max-width: 100%")).unwrap_or_else(|| "width: 100%".into())
        };
        rule("table", &format!("display: table; border-collapse: collapse; {width}"));
        let fill = self.fill.map(hex).unwrap_or_else(|| "transparent".into());
        rule("table tr:nth-child(n)", "background: transparent; border: none");
        rule("table td, table th", &format!("border: none; background: {fill}"));
        let b = self.border();
        let line = format!("{}pt solid {}", b.width, hex(b.color));
        match b.which {
            Borders::All => rule("table td, table th", &format!("border: {line}")),
            Borders::Outer => rule("table", &format!("border: {line}")),
            Borders::Inner => {
                rule("table", "border-style: hidden");
                rule("table td, table th", &format!("border: {line}"));
            }
            Borders::Horizontal => rule("table td, table th", &format!("border-top: {line}; border-bottom: {line}")),
            Borders::None => {}
        }
        if let Some(h) = self.row_height {
            rule("table tr", &format!("height: {h}cm"));
        }
        if self.keep_rows {
            rule("table tr", "break-inside: avoid");
        }
        let band = hex(t.band);
        if self.banded_rows {
            rule("tbody tr:nth-child(2n) td", &format!("background: {band}"));
        }
        if self.banded_cols {
            rule("tbody tr td:nth-child(2n)", &format!("background: {band}"));
        }
        let accent = t.accent_bg.map(|a| format!("; background: {}", hex(a))).unwrap_or_default();
        if self.first_col {
            rule("tbody tr td:first-child", &format!("font-weight: 600{accent}"));
        }
        if self.last_col {
            rule("tbody tr td:last-child", &format!("font-weight: 600{accent}"));
        }
        if self.total {
            rule("tbody tr:last-child td", &format!("font-weight: 600{accent}"));
        }
        if self.header {
            let mut d = String::from("font-weight: 600");
            if let Some(bg) = t.header_bg {
                d.push_str(&format!("; background: {}", hex(bg)));
            }
            if let Some(fg) = t.header_fg {
                d.push_str(&format!("; color: {}", hex(fg)));
            }
            rule("thead tr th", &d);
        } else {
            rule("thead tr th", "font-weight: normal");
        }
        css
    }

    /// The border: (which, width in points, colour).
    pub fn border(&self) -> TableBorder {
        self.border.unwrap_or(TableBorder { which: Borders::All, width: 0.75, color: self.tpl().border })
    }

    /// Which sides of the cell get a line: (top, right, bottom, left).
    pub fn sides(&self, r: usize, c: usize, rows: usize, cols: usize) -> [bool; 4] {
        let (first_r, last_r, first_c, last_c) = (r == 0, r + 1 == rows, c == 0, c + 1 == cols);
        match self.border().which {
            Borders::All => [true; 4],
            Borders::None => [false; 4],
            Borders::Outer => [first_r, last_c, last_r, first_c],
            Borders::Inner => [!first_r, !last_c, !last_r, !first_c],
            Borders::Horizontal => [true, false, true, false],
        }
    }
}

/// `<div data-table="…">` on its own: the look.
pub fn div_table(html: &str) -> Option<TableLook> {
    let t = html.trim();
    if !t.starts_with("<div") || !t.ends_with('>') || t.contains('\n') {
        return None;
    }
    let lower = t.to_ascii_lowercase();
    let p = lower.find("data-table=")?;
    let rest = &t[p + 11..];
    let q = rest.chars().next()?;
    let end = rest[1..].find(q)?;
    Some(TableLook::parse(&rest[1..1 + end]))
}

/// The lines of the text: byte ranges without the newline.
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

/// The `<div data-table>` line and the `</div>` line around the table at `pos`.
fn table_wrapper(text: &str, from: usize, to: usize) -> Option<(Range<usize>, Range<usize>, TableLook)> {
    let ls = lines(text);
    let first = ls.iter().position(|l| l.start == from)?;
    let last = ls.iter().position(|l| l.end == to)?;
    let mut a = first;
    while a > 0 && text[ls[a - 1].clone()].trim().is_empty() {
        a -= 1;
    }
    let mut b = last;
    while b + 1 < ls.len() && text[ls[b + 1].clone()].trim().is_empty() {
        b += 1;
    }
    let open = ls.get(a.checked_sub(1)?)?.clone();
    let close = ls.get(b + 1)?.clone();
    let look = div_table(&text[open.clone()])?;
    (text[close.clone()].trim() == "</div>").then_some((open, close, look))
}

/// The look of the table at `pos` (`None` when the cursor is not in a table).
pub fn table_look_at(text: &str, pos: usize) -> Option<TableLook> {
    let (from, to, ..) = super::edit::table_at(text, pos)?;
    Some(table_wrapper(text, from, to).map(|(_, _, l)| l).unwrap_or_default())
}

/// Change the look of the table at the cursor (adds, edits or removes its `<div>`).
pub fn set_table_look(text: &str, sel: Selection, f: impl FnOnce(&mut TableLook)) -> Option<Edit> {
    let (from, to, ..) = super::edit::table_at(text, sel.head)?;
    match table_wrapper(text, from, to) {
        Some((open, close, look)) => {
            let mut new = look.clone();
            f(&mut new);
            if new == look {
                return None;
            }
            if new.is_default() {
                let open_to = open.end + (text[open.end..].len() - text[open.end..].trim_start().len());
                let close_from = text[..close.start].trim_end().len();
                let removed = open_to - open.start;
                let shift = |p: usize| if p >= open_to { p - removed } else { p };
                return Some(Edit {
                    changes: vec![Change::new(open.start, open_to, ""), Change::new(close_from, close.end, "")],
                    selection: Selection::new(shift(sel.anchor), shift(sel.head)),
                });
            }
            let line = format!("<div data-table=\"{}\">", new.to_attr());
            let delta = line.len() as isize - open.len() as isize;
            let shift = |p: usize| if p > open.end { (p as isize + delta) as usize } else { p };
            Some(Edit {
                changes: vec![Change::new(open.start, open.end, line)],
                selection: Selection::new(shift(sel.anchor), shift(sel.head)),
            })
        }
        None => {
            let mut new = TableLook::default();
            f(&mut new);
            if new.is_default() {
                return None;
            }
            let open = format!("<div data-table=\"{}\">\n\n", new.to_attr());
            let n = open.len();
            Some(Edit {
                changes: vec![Change::insert(from, open), Change::insert(to, "\n\n</div>")],
                selection: Selection::new(sel.anchor + n, sel.head + n),
            })
        }
    }
}

/// The column of the cursor in a table row (0-based).
pub fn table_column_at(text: &str, pos: usize) -> usize {
    let start = text[..pos].rfind('\n').map(|i| i + 1).unwrap_or(0);
    let before = &text[start..pos];
    let pipes = before.matches('|').count() - before.matches("\\|").count();
    if before.trim_start().starts_with('|') { pipes.saturating_sub(1) } else { pipes }
}

/// Rows and columns: insert above / below / left / right, delete a row, a column or the table.
pub fn table_op(text: &str, sel: Selection, op: &str) -> Option<Edit> {
    let col = table_column_at(text, sel.head);
    if op == "delete-table" {
        let (from, to, ..) = super::edit::table_at(text, sel.head)?;
        let (from, to) = match table_wrapper(text, from, to) {
            Some((open, close, _)) => (open.start, close.end),
            None => (from, to),
        };
        return Some(Edit { changes: vec![Change::new(from, to, "")], selection: Selection::cursor(from) });
    }
    super::edit::format_table(text, sel, |mut rows, cur| {
        let n = rows.iter().map(Vec::len).max().unwrap_or(1);
        let empty = vec![String::new(); n];
        match op {
            "row-above" => rows.insert(cur.max(2), empty),
            "row-below" => rows.insert((cur + 1).max(2), empty),
            "delete-row" => {
                if cur >= 2 && rows.len() > 3 {
                    rows.remove(cur);
                }
            }
            "col-left" | "col-right" => {
                let at = if op == "col-left" { col } else { col + 1 }.min(n);
                for (j, r) in rows.iter_mut().enumerate() {
                    r.insert(
                        at.min(r.len()),
                        if j == 1 {
                            "---".into()
                        } else if j == 0 {
                            "New".into()
                        } else {
                            String::new()
                        },
                    );
                }
            }
            "delete-col" if n > 1 => {
                for r in rows.iter_mut() {
                    if col < r.len() {
                        r.remove(col);
                    }
                }
            }
            _ => {}
        }
        rows
    })
}

// ---- pictures ---------------------------------------------------------------------------------

/// How a picture sits among the text.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Wrap {
    /// In the line, like a letter.
    #[default]
    Inline,
    /// On the left, the text flowing round it on the right.
    Left,
    Right,
    /// On its own, the text above and below.
    Block,
}

#[derive(Debug, Clone, PartialEq, Default)]
pub struct ImgStyle {
    /// Width in cm (the height follows).
    pub width: Option<f32>,
    /// 0–1.
    pub opacity: Option<f32>,
    /// Degrees, clockwise.
    pub rotate: f32,
    pub flip_h: bool,
    pub flip_v: bool,
    pub wrap: Wrap,
    /// Cut to this width : height (the middle is kept).
    pub crop: Option<(u32, u32)>,
    pub other: Vec<(String, String)>,
}

impl ImgStyle {
    pub fn is_default(&self) -> bool {
        *self == ImgStyle::default()
    }

    pub fn from_css(css: &str) -> ImgStyle {
        let mut s = ImgStyle::default();
        for decl in css.split(';') {
            let Some((k, v)) = decl.split_once(':') else { continue };
            let (k, v) = (k.trim().to_ascii_lowercase(), v.trim());
            let known = match k.as_str() {
                "width" => {
                    s.width = css_cm(v);
                    s.width.is_some()
                }
                "opacity" => {
                    s.opacity = v.parse().ok().map(|o: f32| o.clamp(0.0, 1.0));
                    s.opacity.is_some()
                }
                "transform" => {
                    for part in v.split(')') {
                        let part = part.trim();
                        if let Some(a) = part.strip_prefix("rotate(") {
                            s.rotate = a.trim_end_matches("deg").trim().parse().unwrap_or(0.0);
                        } else if part.starts_with("scaleX(-1") {
                            s.flip_h = true;
                        } else if part.starts_with("scaleY(-1") {
                            s.flip_v = true;
                        }
                    }
                    true
                }
                "float" => {
                    s.wrap = match v {
                        "left" => Wrap::Left,
                        "right" => Wrap::Right,
                        _ => s.wrap,
                    };
                    true
                }
                "display" => {
                    if v == "block" && s.wrap == Wrap::Inline {
                        s.wrap = Wrap::Block;
                    }
                    true
                }
                "aspect-ratio" => {
                    let mut it = v.split('/').map(|x| x.trim().parse::<u32>().ok());
                    if let (Some(Some(a)), Some(Some(b))) = (it.next(), it.next()) {
                        s.crop = Some((a, b));
                    }
                    true
                }
                // written with the others; not kept on their own
                "margin" | "object-fit" | "height" => true,
                _ => false,
            };
            if !known {
                s.other.push((k, v.to_string()));
            }
        }
        s
    }

    pub fn to_css(&self) -> String {
        let mut d: Vec<String> = Vec::new();
        if let Some(w) = self.width {
            d.push(format!("width: {}cm", (w * 100.0).round() / 100.0));
        }
        if let Some(o) = self.opacity.filter(|o| *o < 0.999) {
            d.push(format!("opacity: {}", (o * 100.0).round() / 100.0));
        }
        let mut t: Vec<String> = Vec::new();
        if self.rotate.rem_euclid(360.0) != 0.0 {
            t.push(format!("rotate({}deg)", self.rotate.rem_euclid(360.0)));
        }
        if self.flip_h {
            t.push("scaleX(-1)".into());
        }
        if self.flip_v {
            t.push("scaleY(-1)".into());
        }
        if !t.is_empty() {
            d.push(format!("transform: {}", t.join(" ")));
        }
        match self.wrap {
            Wrap::Inline => {}
            Wrap::Left => d.push("float: left; margin: 0 0.4cm 0.2cm 0".into()),
            Wrap::Right => d.push("float: right; margin: 0 0 0.2cm 0.4cm".into()),
            Wrap::Block => d.push("display: block; margin: 0.2cm auto".into()),
        }
        if let Some((a, b)) = self.crop {
            d.push(format!("aspect-ratio: {a} / {b}; object-fit: cover"));
        }
        for (k, v) in &self.other {
            d.push(format!("{k}: {v}"));
        }
        d.join("; ")
    }

    /// The rotation as a multiple of 90° (0–3), when it is one.
    pub fn quarter_turns(&self) -> Option<u32> {
        let r = self.rotate.rem_euclid(360.0);
        (r % 90.0 == 0.0).then_some((r / 90.0) as u32)
    }
}

/// A picture in the text.
#[derive(Debug, Clone, PartialEq)]
pub struct ImageRef {
    pub range: Range<usize>,
    pub url: String,
    pub alt: String,
    pub style: ImgStyle,
}

/// An attribute's value in a tag.
fn attr(tag: &str, name: &str) -> Option<String> {
    let lower = tag.to_ascii_lowercase();
    let mut from = 0;
    while let Some(p) = lower[from..].find(name).map(|p| p + from) {
        from = p + name.len();
        let before = lower[..p].chars().last().unwrap_or(' ');
        let rest = tag[p + name.len()..].trim_start();
        if !before.is_whitespace() || !rest.starts_with('=') {
            continue;
        }
        let v = rest[1..].trim_start();
        let q = v.chars().next()?;
        if q == '"' || q == '\'' {
            let end = v[1..].find(q)?;
            return Some(v[1..1 + end].to_string());
        }
        return Some(v.split(|c: char| c.is_whitespace() || c == '>' || c == '/').next().unwrap_or("").to_string());
    }
    None
}

/// The pictures on the line around `pos`: Markdown `![alt](url)` and `<img>` tags.
fn images_on_line(text: &str, pos: usize) -> Vec<ImageRef> {
    let start = text[..pos.min(text.len())].rfind('\n').map(|i| i + 1).unwrap_or(0);
    let end = text[pos.min(text.len())..].find('\n').map(|i| pos + i).unwrap_or(text.len());
    let line = &text[start..end];
    let mut out = Vec::new();
    let mut i = 0;
    while i < line.len() {
        let rest = &line[i..];
        if rest.starts_with("![") {
            if let Some(close) = rest.find("](")
                && let Some(paren) = rest[close + 2..].find(')')
            {
                let alt = rest[2..close].trim_matches('\u{1}').to_string();
                let inner = &rest[close + 2..close + 2 + paren];
                let url = inner.split_whitespace().next().unwrap_or("").to_string();
                let mut len = close + 2 + paren + 1;
                // {width=…} after it
                let mut style = ImgStyle::default();
                if rest[len..].starts_with('{')
                    && let Some(e) = rest[len..].find('}')
                {
                    for kv in rest[len + 1..len + e].split_whitespace() {
                        if let Some(w) = kv.strip_prefix("width=") {
                            style.width =
                                css_cm(w.trim_matches('"')).or_else(|| w.trim_matches('"').parse::<f32>().ok().map(|px| px * 2.54 / 96.0));
                        }
                    }
                    len += e + 1;
                }
                out.push(ImageRef { range: start + i..start + i + len, url, alt, style });
                i += len;
                continue;
            }
        } else if rest.to_ascii_lowercase().starts_with("<img")
            && let Some(e) = rest.find('>')
        {
            let tag = &rest[..=e];
            out.push(ImageRef {
                range: start + i..start + i + e + 1,
                url: attr(tag, "src").unwrap_or_default(),
                alt: attr(tag, "alt").unwrap_or_default(),
                style: attr(tag, "style").map(|s| ImgStyle::from_css(&s)).unwrap_or_default(),
            });
            i += e + 1;
            continue;
        }
        i += rest.chars().next().map(char::len_utf8).unwrap_or(1);
    }
    out
}

/// The picture at the cursor: the one it is in, or the only one on its line.
pub fn image_at(text: &str, pos: usize) -> Option<ImageRef> {
    let all = images_on_line(text, pos);
    all.iter().find(|r| r.range.start <= pos && pos <= r.range.end).cloned().or_else(|| (all.len() == 1).then(|| all[0].clone()))
}

fn esc(s: &str) -> String {
    s.replace('&', "&amp;").replace('"', "&quot;").replace('<', "&lt;")
}

/// The picture written back: Markdown when it has no style, else an `<img>`.
pub fn image_markup(url: &str, alt: &str, style: &ImgStyle) -> String {
    if style.is_default() {
        let url = if url.contains(' ') && !url.starts_with('<') { format!("<{url}>") } else { url.to_string() };
        return format!("![{alt}]({url})");
    }
    let url = url.trim_start_matches('<').trim_end_matches('>');
    format!("<img src=\"{}\" alt=\"{}\" style=\"{}\">", esc(url), esc(alt), style.to_css())
}

/// Change the picture at the cursor: its style and/or its address.
pub fn set_image(text: &str, sel: Selection, f: impl FnOnce(&mut String, &mut ImgStyle)) -> Option<Edit> {
    let img = image_at(text, sel.head)?;
    let (mut url, mut style) = (img.url.clone(), img.style.clone());
    f(&mut url, &mut style);
    let new = image_markup(&url, &img.alt, &style);
    if new == text[img.range.clone()] {
        return None;
    }
    let at = img.range.start;
    Some(Edit { changes: vec![Change::new(img.range.start, img.range.end, new)], selection: Selection::cursor(at) })
}

/// The pixels as the style shows them: cut to the aspect ratio (the middle kept), turned,
/// mirrored and see-through. A turn that is not a quarter leaves transparent corners.
pub fn styled_pixels(mut img: image::RgbaImage, s: &ImgStyle) -> image::RgbaImage {
    use image::imageops;
    if let Some((a, b)) = s.crop.filter(|(a, b)| *a > 0 && *b > 0) {
        let (w, h) = img.dimensions();
        let want = a as f32 / b as f32;
        let (cw, ch) = if w as f32 / h as f32 > want {
            (((h as f32 * want).round() as u32).clamp(1, w), h)
        } else {
            (w, ((w as f32 / want).round() as u32).clamp(1, h))
        };
        img = imageops::crop_imm(&img, (w - cw) / 2, (h - ch) / 2, cw, ch).to_image();
    }
    if s.flip_h {
        imageops::flip_horizontal_in_place(&mut img);
    }
    if s.flip_v {
        imageops::flip_vertical_in_place(&mut img);
    }
    match s.quarter_turns() {
        Some(1) => img = imageops::rotate90(&img),
        Some(2) => img = imageops::rotate180(&img),
        Some(3) => img = imageops::rotate270(&img),
        Some(_) => {}
        None => img = rotate_any(&img, s.rotate),
    }
    if let Some(o) = s.opacity.filter(|o| *o < 0.999) {
        for px in img.pixels_mut() {
            px.0[3] = (px.0[3] as f32 * o).round() as u8;
        }
    }
    img
}

/// Turned clockwise by any angle, on a canvas that holds all of it (bilinear).
fn rotate_any(img: &image::RgbaImage, deg: f32) -> image::RgbaImage {
    let (w, h) = (img.width() as f32, img.height() as f32);
    let (sin, cos) = deg.to_radians().sin_cos();
    let nw = (w * cos.abs() + h * sin.abs()).ceil().max(1.0) as u32;
    let nh = (w * sin.abs() + h * cos.abs()).ceil().max(1.0) as u32;
    let (cx, cy, ncx, ncy) = (w / 2.0, h / 2.0, nw as f32 / 2.0, nh as f32 / 2.0);
    let sample = |x: i64, y: i64| -> [f32; 4] {
        if x < 0 || y < 0 || x >= img.width() as i64 || y >= img.height() as i64 {
            return [0.0; 4];
        }
        let p = img.get_pixel(x as u32, y as u32).0;
        [p[0] as f32, p[1] as f32, p[2] as f32, p[3] as f32]
    };
    image::RgbaImage::from_fn(nw, nh, |x, y| {
        let (dx, dy) = (x as f32 + 0.5 - ncx, y as f32 + 0.5 - ncy);
        // the inverse turn
        let sx = dx * cos + dy * sin + cx - 0.5;
        let sy = -dx * sin + dy * cos + cy - 0.5;
        let (x0, y0) = (sx.floor(), sy.floor());
        let (fx, fy) = (sx - x0, sy - y0);
        let (x0, y0) = (x0 as i64, y0 as i64);
        let (a, b, c, d) = (sample(x0, y0), sample(x0 + 1, y0), sample(x0, y0 + 1), sample(x0 + 1, y0 + 1));
        let mut out = [0u8; 4];
        for i in 0..4 {
            let v = a[i] * (1.0 - fx) * (1.0 - fy) + b[i] * fx * (1.0 - fy) + c[i] * (1.0 - fx) * fy + d[i] * fx * fy;
            out[i] = v.round().clamp(0.0, 255.0) as u8;
        }
        image::Rgba(out)
    })
}

/// The size the style gives a picture of `w` × `h` (any unit): the width set (in the same
/// unit as `width_set`), the bounding box of a turn.
pub fn styled_size(w: f32, h: f32, s: &ImgStyle, width_set: Option<f32>) -> (f32, f32) {
    let (mut w, mut h) = (w, h);
    if let Some((a, b)) = s.crop.filter(|(a, b)| *a > 0 && *b > 0) {
        let want = a as f32 / b as f32;
        if w / h > want { w = h * want } else { h = w / want }
    }
    if let Some(ws) = width_set {
        h *= ws / w;
        w = ws;
    }
    let (sin, cos) = s.rotate.to_radians().sin_cos();
    (w * cos.abs() + h * sin.abs(), w * sin.abs() + h * cos.abs())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn run(text: &str, at: usize, e: Option<Edit>) -> String {
        let e = e.expect("an edit");
        let mut d = kd_doc::Document::new(text);
        d.apply(e.changes, e.selection, kd_doc::EditKind::Other);
        let _ = at;
        d.text().to_string()
    }

    #[test]
    fn table_look_round_trip_and_cells() {
        let mut t =
            TableLook { banded_rows: true, first_col: true, template: Some("blue".into()), width: Some(14.0), ..TableLook::default() };
        t.border = Some(TableBorder { which: Borders::Outer, width: 1.5, color: 0x4472c4 });
        let back = TableLook::parse(&t.to_attr());
        assert_eq!(back, t);
        assert_eq!(t.cell(0, 1, 4, 3), CellLook { bg: Some(0x4472C4), fg: Some(0xFFFFFF), bold: true });
        assert_eq!(t.cell(2, 1, 4, 3).bg, Some(0xD9E2F3));
        assert!(t.cell(1, 0, 4, 3).bold);
        assert_eq!(t.sides(0, 0, 4, 3), [true, false, false, true]);
    }

    #[test]
    fn wraps_and_edits_a_table() {
        let text = "Intro\n\n| a | b |\n|---|---|\n| 1 | 2 |\n\nEnd\n";
        let at = text.find("1 |").unwrap();
        let t1 = run(text, at, set_table_look(text, Selection::cursor(at), |l| l.banded_rows = true));
        assert!(t1.contains("<div data-table=\"banded-rows\">\n\n| a | b |"), "{t1}");
        assert_eq!(table_look_at(&t1, t1.find("1 |").unwrap()).map(|l| l.banded_rows), Some(true));
        let p = crate::md::parse(&t1);
        assert!(p.blocks.iter().any(|b| matches!(b, crate::md::Block::Table { look: Some(l), .. } if l.banded_rows)));
        let at = t1.find("1 |").unwrap();
        let t2 = run(&t1, at, set_table_look(&t1, Selection::cursor(at), |l| l.banded_rows = false));
        assert_eq!(t2, text);
        let t3 = run(text, at, table_op(text, Selection::cursor(text.find("2 |").unwrap()), "col-left"));
        assert!(t3.contains("| a   | New | b   |"), "{t3}");
    }

    #[test]
    fn pictures_get_and_lose_a_style() {
        let text = "See ![Logo](img/logo.png) here\n";
        let at = text.find("Logo").unwrap();
        let t1 = run(
            text,
            at,
            set_image(text, Selection::cursor(at), |_, s| {
                s.width = Some(5.0);
                s.rotate = 90.0;
                s.flip_h = true;
                s.wrap = Wrap::Left;
            }),
        );
        assert!(
            t1.contains("<img src=\"img/logo.png\" alt=\"Logo\" style=\"width: 5cm; transform: rotate(90deg) scaleX(-1); float: left;"),
            "{t1}"
        );
        let img = image_at(&t1, t1.find("img src").unwrap()).expect("found");
        assert_eq!(img.style.quarter_turns(), Some(1));
        assert_eq!(img.style.wrap, Wrap::Left);
        let at = t1.find("img src").unwrap();
        let t2 = run(&t1, at, set_image(&t1, Selection::cursor(at), |_, s| *s = ImgStyle::default()));
        assert_eq!(t2, text);
        let p = crate::md::parse(&t1);
        assert!(
            matches!(&p.blocks[0], crate::md::Block::Paragraph { inlines, .. } if inlines.iter().any(|i| matches!(i, crate::md::Inline::Image { style: Some(s), .. } if s.width == Some(5.0))))
        );
    }

    #[test]
    fn styles_pixels_and_sizes() {
        let mut img = image::RgbaImage::from_pixel(40, 20, image::Rgba([0, 0, 255, 255]));
        img.put_pixel(0, 0, image::Rgba([255, 0, 0, 255]));
        let s = ImgStyle { rotate: 90.0, flip_h: true, opacity: Some(0.5), ..ImgStyle::default() };
        let out = styled_pixels(img.clone(), &s);
        assert_eq!(out.dimensions(), (20, 40));
        // flipped to the top right, turned to the bottom right
        assert_eq!(out.get_pixel(19, 39).0, [255, 0, 0, 128]);
        let sq = styled_pixels(img, &ImgStyle { crop: Some((1, 1)), ..ImgStyle::default() });
        assert_eq!(sq.dimensions(), (20, 20));
        let (w, h) = styled_size(400.0, 200.0, &ImgStyle { rotate: 90.0, ..ImgStyle::default() }, Some(100.0));
        assert!((w - 50.0).abs() < 0.01 && (h - 100.0).abs() < 0.01);
    }

    #[test]
    fn a_table_look_as_css() {
        let l = TableLook::parse("banded-rows; template: blue; border: inner 1pt #000000; fit");
        let css = l.css(".t");
        assert!(css.contains(".t table { display: table; border-collapse: collapse; width: auto }"), "{css}");
        assert!(css.contains("border-style: hidden") && css.contains("#d9e2f3") && css.contains("#4472c4"));
    }
}
