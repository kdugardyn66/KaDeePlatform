//! PDF, written directly: the document laid out on the front matter's paper size and margins
//! (decks: one 16:9 page per slide) with the standard PDF fonts (Helvetica, Courier), pictures,
//! charts and diagrams, tables, links, page numbers and the watermark. The standard fonts cover
//! Western European text (Windows-1252); other characters are replaced.
#![allow(clippy::write_with_newline)] // content streams are one operator per line

use std::fmt::Write as _;
use std::io::Write as _;

use super::flow::{self, Item, Kind, Para, SLIDE_H, SLIDE_W, SlideBox};
use super::{Doc, Picture, Run};
use crate::md::{self, Align, Style};

// ---- fonts -----------------------------------------------------------------------------------------

#[rustfmt::skip]
const HELVETICA: [u16; 95] = [
    278,278,355,556,556,889,667,191,333,333,389,584,278,333,278,278,556,556,556,556,556,556,556,556,556,556,278,278,584,584,584,556,
    1015,667,667,722,722,667,611,778,722,278,500,667,556,833,722,778,667,778,722,667,611,722,667,944,667,667,611,278,278,278,469,556,
    333,556,556,500,556,556,278,556,556,222,222,500,222,833,556,556,556,556,333,500,278,556,500,722,500,500,500,334,260,334,584,
];
#[rustfmt::skip]
const HELVETICA_BOLD: [u16; 95] = [
    278,333,474,556,556,889,722,238,333,333,389,584,278,333,278,278,556,556,556,556,556,556,556,556,556,556,333,333,584,584,584,611,
    975,722,722,722,722,667,611,778,722,278,556,722,611,833,722,778,667,778,722,667,611,722,667,944,667,667,611,333,278,333,584,556,
    333,556,611,556,611,556,333,611,611,278,278,556,278,889,611,611,611,611,389,556,333,611,556,778,556,556,500,389,280,389,584,
];

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Font {
    Regular,
    Bold,
    Italic,
    BoldItalic,
    Mono,
}

impl Font {
    fn of(s: &Style, mono: bool) -> Font {
        match (mono || s.code, s.bold, s.italic) {
            (true, _, _) => Font::Mono,
            (_, true, true) => Font::BoldItalic,
            (_, true, false) => Font::Bold,
            (_, false, true) => Font::Italic,
            _ => Font::Regular,
        }
    }

    fn name(self) -> &'static str {
        match self {
            Font::Regular => "F1",
            Font::Bold => "F2",
            Font::Italic => "F3",
            Font::BoldItalic => "F4",
            Font::Mono => "F5",
        }
    }

    /// Width of a Windows-1252 byte in 1/1000 em.
    fn width(self, b: u8) -> f32 {
        if self == Font::Mono {
            return 600.0;
        }
        let bold = matches!(self, Font::Bold | Font::BoldItalic);
        let w = match b {
            32..=126 => (if bold { HELVETICA_BOLD } else { HELVETICA })[(b - 32) as usize],
            0x95 => 350,
            0x85 | 0x97 | 0x89 => 1000,
            0x91 | 0x92 => {
                if bold {
                    278
                } else {
                    222
                }
            }
            0x93 | 0x94 => {
                if bold {
                    500
                } else {
                    333
                }
            }
            0x99 => 1000,
            0xA0 => 278,
            _ => {
                if bold {
                    611
                } else {
                    556
                }
            }
        };
        w as f32
    }
}

/// A character in Windows-1252, or a stand-in.
fn win(c: char) -> Vec<u8> {
    let b = match c {
        ' '..='~' => c as u8,
        '\u{A0}'..='\u{FF}' => c as u32 as u8,
        '€' => 0x80,
        '‚' => 0x82,
        'ƒ' => 0x83,
        '„' => 0x84,
        '…' => 0x85,
        '†' => 0x86,
        '‡' => 0x87,
        'ˆ' => 0x88,
        '‰' => 0x89,
        'Š' => 0x8A,
        '‹' => 0x8B,
        'Œ' => 0x8C,
        'Ž' => 0x8E,
        '‘' => 0x91,
        '’' => 0x92,
        '“' => 0x93,
        '”' => 0x94,
        '•' | '▪' | '◦' | '●' => 0x95,
        '–' => 0x96,
        '—' => 0x97,
        '˜' => 0x98,
        '™' => 0x99,
        'š' => 0x9A,
        '›' => 0x9B,
        'œ' => 0x9C,
        'ž' => 0x9E,
        'Ÿ' => 0x9F,
        '−' => b'-',
        '\t' => b' ',
        '☑' | '✓' | '✔' => return b"[x]".to_vec(),
        '☐' => return b"[ ]".to_vec(),
        '→' | '⟶' => return b"->".to_vec(),
        '←' => return b"<-".to_vec(),
        '⇒' => return b"=>".to_vec(),
        '≤' => return b"<=".to_vec(),
        '≥' => return b">=".to_vec(),
        '≠' => return b"!=".to_vec(),
        '≈' => return b"~".to_vec(),
        '⁄' => b'/',
        '\u{200B}' | '\u{FE0F}' | '\u{20D7}' | '\u{0302}' | '\u{0303}' | '\u{0305}' | '\u{0307}' => return Vec::new(),
        c if (c as u32) >= 0x1F000 => return Vec::new(),
        _ => b'?',
    };
    vec![b]
}

fn encode(s: &str) -> Vec<u8> {
    s.chars().flat_map(win).collect()
}

fn text_width(bytes: &[u8], font: Font, size: f32) -> f32 {
    bytes.iter().map(|b| font.width(*b)).sum::<f32>() * size / 1000.0
}

fn pdf_string(bytes: &[u8]) -> String {
    let mut s = String::from("(");
    for b in bytes {
        match b {
            b'(' | b')' | b'\\' => {
                s.push('\\');
                s.push(*b as char);
            }
            32..=126 => s.push(*b as char),
            _ => {
                let _ = write!(s, "\\{b:03o}");
            }
        }
    }
    s.push(')');
    s
}

fn rgb(hex: &str) -> (f32, f32, f32) {
    let c = super::parse_hex(&format!("#{}", hex.trim_start_matches('#'))).unwrap_or(0);
    (((c >> 16) & 255) as f32 / 255.0, ((c >> 8) & 255) as f32 / 255.0, (c & 255) as f32 / 255.0)
}

// ---- layout -------------------------------------------------------------------------------------

/// A piece of a line: text in one style.
#[derive(Debug, Clone)]
struct Piece {
    bytes: Vec<u8>,
    font: Font,
    size: f32,
    color: String,
    style: Style,
    width: f32,
}

#[derive(Default)]
struct Page {
    ops: String,
    links: Vec<(f32, f32, f32, f32, String)>,
    images: Vec<usize>,
}

struct Writer {
    pages: Vec<Page>,
    images: Vec<Picture>,
    ph: f32,
    /// Top margin (the page's text starts there).
    margin: f32,
    bottom: f32,
    y: f32,
    base: f32,
    fg: String,
    head: String,
    deck: bool,
}

impl Writer {
    fn page(&mut self) -> &mut Page {
        if self.pages.is_empty() {
            self.pages.push(Page::default());
        }
        self.pages.last_mut().expect("a page")
    }

    fn new_page(&mut self) {
        self.pages.push(Page::default());
        self.y = self.margin;
    }

    fn ensure(&mut self, h: f32) {
        if !self.deck && self.y + h > self.ph - self.bottom && self.y > self.margin + 1.0 {
            self.new_page();
        }
    }

    fn rect(&mut self, x: f32, y: f32, w: f32, h: f32, color: &str) {
        let (r, g, b) = rgb(color);
        let py = self.ph - y - h;
        let _ = write!(self.page().ops, "{r:.3} {g:.3} {b:.3} rg {x:.2} {py:.2} {w:.2} {h:.2} re f\n");
    }

    fn line(&mut self, x1: f32, y1: f32, x2: f32, y2: f32, color: &str, width: f32) {
        let (r, g, b) = rgb(color);
        let ph = self.ph;
        let _ = write!(self.page().ops, "{r:.3} {g:.3} {b:.3} RG {width:.2} w {x1:.2} {:.2} m {x2:.2} {:.2} l S\n", ph - y1, ph - y2);
    }

    /// Runs as pieces (words keep their trailing space so lines break between them).
    fn pieces(&self, runs: &[Run], size: f32, color: &str, extra: &Style, mono: bool) -> Vec<Option<Piece>> {
        let mut out = Vec::new();
        for r in runs {
            match r {
                Run::Text(t, s) => {
                    let mut st = s.clone();
                    st.bold |= extra.bold;
                    st.italic |= extra.italic;
                    let font = Font::of(&st, mono);
                    let sz =
                        if st.sup || st.sub { size * 0.7 } else { st.size.as_deref().and_then(crate::md::style::points).unwrap_or(size) };
                    let col = st
                        .color
                        .as_deref()
                        .and_then(super::parse_hex)
                        .map(|c| format!("{c:06X}"))
                        .unwrap_or_else(|| if st.link.is_some() { "0969DA".into() } else { color.to_string() });
                    for (i, line) in t.split('\n').enumerate() {
                        if i > 0 {
                            out.push(None);
                        }
                        let mut word = String::new();
                        let flush = |word: &mut String, out: &mut Vec<Option<Piece>>| {
                            if !word.is_empty() {
                                let bytes = encode(word);
                                let width = text_width(&bytes, font, sz);
                                out.push(Some(Piece { bytes, font, size: sz, color: col.clone(), style: st.clone(), width }));
                                word.clear();
                            }
                        };
                        for c in line.chars() {
                            word.push(c);
                            if c == ' ' && !mono {
                                flush(&mut word, &mut out);
                            }
                        }
                        flush(&mut word, &mut out);
                    }
                }
                Run::Break => out.push(None),
                Run::Image(_, alt) => {
                    let bytes = encode(&format!("[{alt}] "));
                    let width = text_width(&bytes, Font::Italic, size);
                    out.push(Some(Piece { bytes, font: Font::Italic, size, color: color.to_string(), style: Style::default(), width }));
                }
            }
        }
        out
    }

    /// Break pieces into lines `w` wide.
    fn lines(pieces: Vec<Option<Piece>>, w: f32) -> Vec<Vec<Piece>> {
        let mut lines: Vec<Vec<Piece>> = vec![Vec::new()];
        let mut x = 0.0;
        for p in pieces {
            match p {
                None => {
                    lines.push(Vec::new());
                    x = 0.0;
                }
                Some(mut p) => {
                    let trimmed = {
                        let n = p.bytes.iter().rev().take_while(|b| **b == b' ').count();
                        p.width - n as f32 * p.font.width(b' ') * p.size / 1000.0
                    };
                    if x + trimmed > w && x > 0.0 {
                        lines.push(Vec::new());
                        x = 0.0;
                    }
                    // a word longer than the line: cut it
                    while p.width > w && p.bytes.len() > 1 {
                        let mut cut = 0;
                        let mut acc = 0.0;
                        for (i, b) in p.bytes.iter().enumerate() {
                            acc += p.font.width(*b) * p.size / 1000.0;
                            if acc > w {
                                cut = i.max(1);
                                break;
                            }
                        }
                        if cut == 0 {
                            break;
                        }
                        let rest = p.bytes.split_off(cut);
                        let head = Piece { width: text_width(&p.bytes, p.font, p.size), ..p.clone() };
                        if let Some(l) = lines.last_mut() {
                            l.push(head);
                        }
                        lines.push(Vec::new());
                        p.bytes = rest;
                        p.width = text_width(&p.bytes, p.font, p.size);
                    }
                    x += p.width;
                    if let Some(l) = lines.last_mut() {
                        l.push(p);
                    }
                }
            }
        }
        lines
    }

    /// Draw one line at (x, baseline y).
    fn draw_line(&mut self, line: &[Piece], x0: f32, base_y: f32) {
        let mut x = x0;
        let ph = self.ph;
        for p in line {
            let text_w = p.width - p.bytes.iter().rev().take_while(|b| **b == b' ').count() as f32 * p.font.width(b' ') * p.size / 1000.0;
            if p.style.highlight || p.style.background.is_some() || p.style.code {
                let bg = if p.style.highlight {
                    "FFF3A3".to_string()
                } else if let Some(c) = p.style.background.as_deref().and_then(super::parse_hex) {
                    format!("{c:06X}")
                } else {
                    "EFF1F3".into()
                };
                self.rect(x - 0.5, base_y - p.size * 0.85, text_w + 1.0, p.size * 1.15, &bg);
            }
            let rise = if p.style.sup {
                p.size * 0.6
            } else if p.style.sub {
                -p.size * 0.25
            } else {
                0.0
            };
            let (r, g, b) = rgb(&p.color);
            let _ = write!(
                self.page().ops,
                "BT /{} {:.2} Tf {r:.3} {g:.3} {b:.3} rg {x:.2} {:.2} Td {} Tj ET\n",
                p.font.name(),
                p.size,
                ph - base_y + rise,
                pdf_string(&p.bytes)
            );
            if p.style.underline || p.style.link.is_some() {
                self.line(x, base_y + 1.5, x + text_w, base_y + 1.5, &p.color, 0.6);
            }
            if p.style.strike {
                self.line(x, base_y - p.size * 0.3, x + text_w, base_y - p.size * 0.3, &p.color, 0.6);
            }
            if let Some(u) = &p.style.link
                && !u.starts_with('#')
            {
                let rect = (x, ph - base_y - 2.0, x + text_w, ph - base_y + p.size * 0.8);
                self.page().links.push((rect.0, rect.1, rect.2, rect.3, u.clone()));
            }
            x += p.width;
        }
    }

    /// A paragraph in a box `w` wide at x, from the current y; returns its height.
    #[allow(clippy::too_many_arguments)]
    fn para(&mut self, p: &Para, x: f32, w: f32, size: f32, center: bool, measure_only: bool) -> f32 {
        let s = size * flow::scale(p.kind);
        let (color, extra) = match p.kind {
            Kind::Heading(_) => (self.head.clone(), Style { bold: true, ..Style::default() }),
            Kind::Quote => ("59636E".to_string(), Style { italic: true, ..Style::default() }),
            Kind::Small => ("59636E".to_string(), Style::default()),
            _ => (self.fg.clone(), Style::default()),
        };
        use crate::md::para::{LineSpacing, ParaAlign, Special};
        const CM: f32 = 72.0 / 2.54;
        let fmt = p.fmt.as_ref();
        let left = fmt.map(|f| f.left_total() * CM).unwrap_or(0.0);
        let right = fmt.and_then(|f| f.right).unwrap_or(0.0) * CM;
        let first = match fmt.and_then(|f| f.special) {
            Some(Special::FirstLine(v)) => v * CM,
            Some(Special::Hanging(v)) => -v * CM,
            None => 0.0,
        };
        let boxed = fmt.is_some_and(|f| f.background.is_some() || f.border.is_some());
        let fpad = if boxed { fmt.and_then(|f| f.padding).unwrap_or(0.2) * CM } else { 0.0 };
        let indent = p.level as f32 * s * 1.4 + if matches!(p.kind, Kind::Quote | Kind::Callout(_)) { s } else { 0.0 } + left;
        let mono = p.kind == Kind::Code;
        let pad = if mono { 6.0 } else { fpad };
        // a first-line indent narrows all lines a little (the lines are measured alike)
        let inner_w = w - indent - right - 2.0 * pad - first.max(0.0);
        let lines = Self::lines(self.pieces(&p.runs, s, &color, &extra, mono), inner_w);
        let lh = match fmt.and_then(|f| f.line) {
            Some(LineSpacing::Multiple(m)) => s * crate::md::para::SINGLE * m,
            Some(LineSpacing::Exactly(pt)) => pt * s / size.max(1.0),
            None => s * if mono { 1.3 } else { 1.35 },
        };
        let before = match (p.kind, fmt.and_then(|f| f.before)) {
            (_, Some(b)) => b * CM,
            (Kind::Heading(1), _) => s * 0.8,
            (Kind::Heading(_), _) => s * 0.7,
            _ if p.level > 0 => s * 0.15,
            _ => s * 0.35,
        };
        let after = fmt.and_then(|f| f.after).map(|a| a * CM).unwrap_or(s * 0.35);
        let total = before + lines.len() as f32 * lh + 2.0 * pad + after;
        if measure_only {
            return total;
        }
        if !self.deck && fmt.is_some_and(|f| f.page_break_before) && self.y > self.margin + 1.0 {
            self.new_page();
        }
        if !self.deck {
            // headings stay with what follows
            let keep = if matches!(p.kind, Kind::Heading(_)) || fmt.is_some_and(|f| f.keep_together) {
                (lh * 3.0).max(total - before)
            } else {
                lh.min(total)
            };
            self.ensure(before + keep.min(self.ph - self.margin - self.bottom));
        }
        self.y += before;
        let top = self.y;
        let box_h = lines.len() as f32 * lh + 2.0 * pad;
        if mono {
            self.rect(x + indent, top, w - indent, box_h, "F6F8FA");
        }
        if let Some(f) = fmt {
            let bx = x + indent - first.min(0.0).abs();
            let bw = w - indent - right + first.min(0.0).abs();
            if let Some(bg) = f.background {
                self.rect(bx, top, bw, box_h, &format!("{bg:06X}"));
            }
            if let Some(b) = f.border {
                let c = format!("{:06X}", b.color);
                let (x2, y2) = (bx + bw, top + box_h);
                self.line(bx, top, x2, top, &c, b.width);
                self.line(bx, y2, x2, y2, &c, b.width);
                self.line(bx, top, bx, y2, &c, b.width);
                self.line(x2, top, x2, y2, &c, b.width);
            }
        }
        let align = fmt.and_then(|f| f.align).or(if fmt.is_some_and(|f| f.rtl) { Some(ParaAlign::Right) } else { None });
        for (i, line) in lines.iter().enumerate() {
            if !self.deck && self.y + lh > self.ph - self.bottom {
                self.new_page();
            }
            let lw: f32 = line.iter().map(|q| q.width).sum();
            let shift = if i == 0 { first } else { 0.0 };
            let avail = w - indent - right - 2.0 * pad;
            let lx = if center || p.kind == Kind::Center || align == Some(ParaAlign::Center) {
                x + indent + pad + (avail - lw) / 2.0
            } else if align == Some(ParaAlign::Right) {
                x + indent + pad + avail - lw
            } else {
                x + indent + pad + shift
            };
            self.draw_line(line, lx, self.y + pad + s * 1.0 + (lh - s * 1.35).max(0.0) / 2.0);
            self.y += lh;
        }
        self.y += 2.0 * pad;
        // bullet, quote bar, callout bar
        if let Some(b) = p.bullet.as_ref().filter(|b| !b.is_empty()) {
            let bytes = encode(b);
            let bw = text_width(&bytes, Font::Regular, s);
            let (r, g, bl) = rgb(&color);
            let ph = self.ph;
            let _ = write!(
                self.page().ops,
                "BT /F1 {s:.2} Tf {r:.3} {g:.3} {bl:.3} rg {:.2} {:.2} Td {} Tj ET\n",
                x + indent - bw - s * 0.4,
                ph - (top + s),
                pdf_string(&bytes)
            );
        }
        match p.kind {
            Kind::Quote => self.line(x + indent - s * 0.6, top, x + indent - s * 0.6, self.y, "D0D7DE", 2.5),
            Kind::Callout(c) => self.line(x + indent - s * 0.6, top - before * 0.5, x + indent - s * 0.6, self.y, &format!("{c:06X}"), 3.0),
            Kind::Heading(1) | Kind::Heading(2) if !self.deck => {
                let y = self.y + 2.0;
                self.line(x, y, x + w, y, "D0D7DE", 0.6);
                self.y += 4.0;
            }
            _ => {}
        }
        self.y += after;
        total
    }

    fn picture(&mut self, p: &Picture, x: f32, y: f32, w: f32, h: f32) {
        let n = self.images.len();
        self.images.push(p.clone());
        let ph = self.ph;
        let page = self.page();
        page.images.push(n);
        let _ = write!(page.ops, "q {w:.2} 0 0 {h:.2} {x:.2} {:.2} cm /Im{n} Do Q\n", ph - y - h);
    }

    fn flow_picture(&mut self, p: &Picture, x: f32, w: f32) {
        let (mut pw, mut ph) = p.points(w);
        let max_h = self.ph - self.margin - self.bottom;
        if ph > max_h {
            pw *= max_h / ph;
            ph = max_h;
        }
        self.ensure(ph + 8.0);
        let px = x + (w - pw) / 2.0;
        let y = self.y + 4.0;
        self.picture(p, px, y, pw, ph);
        self.y = y + ph + 10.0;
    }

    #[allow(clippy::too_many_arguments)]
    fn table(&mut self, aligns: &[Align], header: &[Vec<Run>], rows: &[Vec<Vec<Run>>], x: f32, w: f32, size: f32) {
        let n = header.len().max(1);
        let col = w / n as f32;
        let pad = 4.0;
        let fg = self.fg.clone();
        let all: Vec<(bool, Vec<Vec<Run>>)> =
            std::iter::once((true, header.to_vec())).chain(rows.iter().map(|r| (false, r.clone()))).collect();
        self.y += size * 0.3;
        for (head, cells) in all {
            let extra = Style { bold: head, ..Style::default() };
            let lines: Vec<Vec<Vec<Piece>>> = (0..n)
                .map(|i| {
                    Self::lines(self.pieces(cells.get(i).map(Vec::as_slice).unwrap_or(&[]), size, &fg, &extra, false), col - 2.0 * pad)
                })
                .collect();
            let lh = size * 1.3;
            let rh = lines.iter().map(|l| l.len()).max().unwrap_or(1) as f32 * lh + 2.0 * pad;
            self.ensure(rh);
            let top = self.y;
            if head {
                self.rect(x, top, w, rh, "F2F4F7");
            }
            for (i, cell) in lines.iter().enumerate() {
                let cx = x + i as f32 * col;
                for (k, line) in cell.iter().enumerate() {
                    let lw: f32 = line.iter().map(|q| q.width).sum();
                    let lx = match aligns.get(i) {
                        Some(Align::Center) => cx + (col - lw) / 2.0,
                        Some(Align::Right) => cx + col - pad - lw,
                        _ => cx + pad,
                    };
                    self.draw_line(line, lx, top + pad + k as f32 * lh + size);
                }
            }
            let bottom = top + rh;
            self.line(x, top, x + w, top, "D0D7DE", 0.5);
            self.line(x, bottom, x + w, bottom, "D0D7DE", 0.5);
            for i in 0..=n {
                let cx = x + i as f32 * col;
                self.line(cx, top, cx, bottom, "D0D7DE", 0.5);
            }
            self.y = bottom;
        }
        self.y += size * 0.8;
    }

    fn items(&mut self, items: &[Item], x: f32, w: f32, size: f32, center: bool) {
        for it in items {
            match it {
                Item::Paras(v) => {
                    for p in v {
                        self.para(p, x, w, size, center, false);
                    }
                }
                Item::Table { aligns, header, rows } => self.table(aligns, header, rows, x, w, size * 0.92),
                Item::Picture(p) => self.flow_picture(p, x, w),
                Item::Rule => {
                    self.ensure(12.0);
                    let y = self.y + 6.0;
                    self.line(x, y, x + w, y, "D0D7DE", 1.5);
                    self.y += 14.0;
                }
                Item::PageBreak => {
                    if !self.deck {
                        self.new_page();
                    }
                }
            }
        }
    }
}

// ---- the file ----------------------------------------------------------------------------------------

fn deflate(data: &[u8]) -> Vec<u8> {
    let mut e = flate2::write::ZlibEncoder::new(Vec::new(), flate2::Compression::default());
    let _ = e.write_all(data);
    e.finish().unwrap_or_default()
}

/// A picture as an image XObject: its dictionary (without the closing `>>` and length), its
/// stream, and a soft mask object when it has transparency.
struct ImageObj {
    dict: String,
    stream: Vec<u8>,
    smask: Option<Vec<u8>>,
}

fn stream_obj(dict: &str, data: &[u8]) -> Vec<u8> {
    let mut o = format!("{dict} /Length {} >>\nstream\n", data.len()).into_bytes();
    o.extend_from_slice(data);
    o.extend_from_slice(b"\nendstream");
    o
}

fn image_objects(p: &Picture) -> Option<ImageObj> {
    let img = image::load_from_memory(&p.data).ok()?;
    let (w, h) = (img.width(), img.height());
    if p.ext == "jpeg" {
        let cs = if img.color().channel_count() == 1 { "/DeviceGray" } else { "/DeviceRGB" };
        return Some(ImageObj {
            dict: format!(
                "<< /Type /XObject /Subtype /Image /Width {w} /Height {h} /ColorSpace {cs} /BitsPerComponent 8 /Filter /DCTDecode"
            ),
            stream: p.data.clone(),
            smask: None,
        });
    }
    let rgba = img.to_rgba8();
    let has_alpha = rgba.pixels().any(|px| px.0[3] < 255);
    let rgb: Vec<u8> = rgba.pixels().flat_map(|px| [px.0[0], px.0[1], px.0[2]]).collect();
    let smask = has_alpha.then(|| {
        let a: Vec<u8> = rgba.pixels().map(|px| px.0[3]).collect();
        stream_obj(
            &format!(
                "<< /Type /XObject /Subtype /Image /Width {w} /Height {h} /ColorSpace /DeviceGray /BitsPerComponent 8 /Filter /FlateDecode"
            ),
            &deflate(&a),
        )
    });
    Some(ImageObj {
        dict: format!(
            "<< /Type /XObject /Subtype /Image /Width {w} /Height {h} /ColorSpace /DeviceRGB /BitsPerComponent 8 /Filter /FlateDecode"
        ),
        stream: deflate(&rgb),
        smask,
    })
}

pub fn write(doc: &Doc) -> Result<Vec<u8>, String> {
    let deck = !doc.slides.is_empty();
    let (pw, ph) = if deck { (SLIDE_W, SLIDE_H) } else { doc.page_size() };
    let m = if deck { md::page::Margins::all(0.0) } else { doc.margins() };
    let margin = m.top;
    let mut w = Writer {
        pages: Vec::new(),
        images: Vec::new(),
        ph,
        margin,
        bottom: m.bottom,
        y: margin,
        base: doc.font_size().clamp(7.0, 20.0),
        fg: "1F2328".into(),
        head: doc.heading_color(),
        deck,
    };
    if deck {
        for s in &doc.slides {
            let (bg, fg, head) = doc.slide_colors(s);
            w.pages.push(Page::default());
            w.rect(0.0, 0.0, pw, ph, &bg);
            w.fg = fg;
            w.head = head;
            for b in flow::slide_layout(doc, s) {
                match b {
                    SlideBox::Text { x, y, w: bw, h, paras, size, center, title } => {
                        let size = if title { size * 1.8 } else { size };
                        let paras: Vec<Para> = if title {
                            paras.into_iter().map(|p| Para { kind: Kind::Body, runs: bold(&p.runs), ..p }).collect()
                        } else {
                            paras
                        };
                        let head_color = w.head.clone();
                        let next = if title { head_color } else { w.fg.clone() };
                        let saved = std::mem::replace(&mut w.fg, next);
                        let total: f32 = paras.iter().map(|p| w.para(p, x, bw, size, center, true)).sum();
                        w.y = if center { y + ((h - total) / 2.0).max(0.0) } else { y };
                        for p in &paras {
                            w.para(p, x, bw, size, center, false);
                        }
                        w.fg = saved;
                    }
                    SlideBox::Picture { x, y, w: bw, h, picture } => w.picture(&picture, x, y, bw, h),
                    SlideBox::Table { x, y, w: bw, item: Item::Table { aligns, header, rows }, size, .. } => {
                        w.y = y;
                        w.table(&aligns, &header, &rows, x, bw, size);
                    }
                    SlideBox::Table { .. } => {}
                }
            }
        }
    } else {
        w.pages.push(Page::default());
        let items = flow::flow(doc, &doc.parsed.blocks);
        let (x, tw, size) = (m.left, pw - m.left - m.right, w.base);
        w.items(&items, x, tw, size, false);
    }
    // page numbers and the watermark
    let count = w.pages.len();
    let watermark = doc.meta("watermark").filter(|m| !m.is_empty()).map(encode);
    for i in 0..count {
        let mut ops = std::mem::take(&mut w.pages[i].ops);
        if let Some(wm) = &watermark {
            let size = 72.0f32;
            let tw = text_width(wm, Font::Bold, size);
            let (c, s) = (std::f32::consts::FRAC_1_SQRT_2, std::f32::consts::FRAC_1_SQRT_2);
            let (cx, cy) = (pw / 2.0, ph / 2.0);
            let _ = write!(
                ops,
                "q 0.85 0.85 0.85 rg BT /F2 {size} Tf {c:.4} {s:.4} {:.4} {c:.4} {:.2} {:.2} Tm {} Tj ET Q\n",
                -s,
                cx - c * tw / 2.0 + s * size / 3.0,
                cy - s * tw / 2.0 - c * size / 3.0,
                pdf_string(wm)
            );
        }
        if !deck && count > 1 {
            let n = encode(&format!("{}", i + 1));
            let nw = text_width(&n, Font::Regular, 9.0);
            let _ = write!(ops, "BT /F1 9 Tf 0.55 0.58 0.62 rg {:.2} {:.2} Td {} Tj ET\n", (pw - nw) / 2.0, m.bottom / 2.0, pdf_string(&n));
        }
        w.pages[i].ops = ops;
    }

    // objects: 1 catalog, 2 pages, 3–7 fonts, 8 info, then pages, contents, images
    let mut objs: Vec<Vec<u8>> = Vec::new();
    objs.push(b"<< /Type /Catalog /Pages 2 0 R >>".to_vec());
    objs.push(Vec::new()); // the page tree, filled in below
    for base in ["Helvetica", "Helvetica-Bold", "Helvetica-Oblique", "Helvetica-BoldOblique", "Courier"] {
        objs.push(format!("<< /Type /Font /Subtype /Type1 /BaseFont /{base} /Encoding /WinAnsiEncoding >>").into_bytes());
    }
    let lit = |s: &str| pdf_string(&encode(s));
    objs.push(
        format!(
            "<< /Title {} /Author {} /Keywords {} /Producer (Kadee Markdown) /Creator (Kadee Markdown) >>",
            lit(&doc.title),
            lit(&doc.author),
            lit(&doc.keywords())
        )
        .into_bytes(),
    );
    // images
    let mut image_ids = Vec::new();
    for p in &w.images {
        match image_objects(p) {
            Some(img) => {
                let mut dict = img.dict;
                if let Some(m) = img.smask {
                    objs.push(m);
                    let _ = write!(dict, " /SMask {} 0 R", objs.len());
                }
                objs.push(stream_obj(&dict, &img.stream));
                image_ids.push(Some(objs.len()));
            }
            None => image_ids.push(None),
        }
    }
    let fonts = "/Font << /F1 3 0 R /F2 4 0 R /F3 5 0 R /F4 6 0 R /F5 7 0 R >>";
    let mut kids = Vec::new();
    for page in &w.pages {
        let content = deflate(page.ops.as_bytes());
        objs.push(stream_obj("<< /Filter /FlateDecode", &content));
        let content_id = objs.len();
        let mut annots = Vec::new();
        for (x1, y1, x2, y2, url) in &page.links {
            objs.push(
                format!(
                    "<< /Type /Annot /Subtype /Link /Rect [{x1:.2} {y1:.2} {x2:.2} {y2:.2}] /Border [0 0 0] /A << /S /URI /URI {} >> >>",
                    lit(url)
                )
                .into_bytes(),
            );
            annots.push(format!("{} 0 R", objs.len()));
        }
        let xobjects: Vec<String> =
            page.images.iter().filter_map(|i| image_ids.get(*i).copied().flatten().map(|id| format!("/Im{i} {id} 0 R"))).collect();
        let res = format!("<< {fonts} /XObject << {} >> >>", xobjects.join(" "));
        let annots = if annots.is_empty() { String::new() } else { format!(" /Annots [{}]", annots.join(" ")) };
        objs.push(
            format!("<< /Type /Page /Parent 2 0 R /MediaBox [0 0 {pw:.2} {ph:.2}] /Contents {content_id} 0 R /Resources {res}{annots} >>")
                .into_bytes(),
        );
        kids.push(format!("{} 0 R", objs.len()));
    }
    objs[1] = format!("<< /Type /Pages /Kids [{}] /Count {} >>", kids.join(" "), kids.len()).into_bytes();
    let mut out = b"%PDF-1.7\n%\xE2\xE3\xCF\xD3\n".to_vec();
    let mut offsets = Vec::new();
    for (i, o) in objs.iter().enumerate() {
        offsets.push(out.len());
        out.extend_from_slice(format!("{} 0 obj\n", i + 1).as_bytes());
        out.extend_from_slice(o);
        out.extend_from_slice(b"\nendobj\n");
    }
    let xref = out.len();
    out.extend_from_slice(format!("xref\n0 {}\n0000000000 65535 f \n", objs.len() + 1).as_bytes());
    for o in offsets {
        out.extend_from_slice(format!("{o:010} 00000 n \n").as_bytes());
    }
    out.extend_from_slice(
        format!("trailer\n<< /Size {} /Root 1 0 R /Info 8 0 R >>\nstartxref\n{xref}\n%%EOF\n", objs.len() + 1).as_bytes(),
    );
    let _ = w.base;
    Ok(out)
}

fn bold(runs: &[Run]) -> Vec<Run> {
    runs.iter()
        .map(|r| match r {
            Run::Text(t, s) => Run::Text(t.clone(), Style { bold: true, ..s.clone() }),
            o => o.clone(),
        })
        .collect()
}
