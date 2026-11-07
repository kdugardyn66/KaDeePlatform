//! The document as a flow of paragraphs, tables and pictures: what the slide writers (PPTX,
//! ODP) and the PDF writer lay out themselves.

use super::{Doc, Picture, Run, runs};
use crate::md::{Align, Block, Inline, ListItem, Style};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Kind {
    Heading(u8),
    Body,
    Quote,
    Code,
    /// A callout with its colour (`RRGGBB`); its title is bold in the runs.
    Callout(u32),
    Small,
    Center,
}

#[derive(Debug, Clone)]
pub(crate) struct Para {
    pub runs: Vec<Run>,
    pub kind: Kind,
    /// List nesting (0 = not in a list).
    pub level: usize,
    /// "•", "1." … for list items.
    pub bullet: Option<String>,
    /// Paragraph formatting (`<div style>` around it).
    pub fmt: Option<crate::md::para::ParaStyle>,
}

impl Para {
    /// Centred: a centred kind or a centred paragraph format.
    pub fn centred(&self) -> bool {
        self.kind == Kind::Center || self.fmt.as_ref().is_some_and(|f| f.align == Some(crate::md::para::ParaAlign::Center))
    }
}

#[derive(Debug, Clone)]
pub(crate) enum Item {
    Paras(Vec<Para>),
    Table { aligns: Vec<Align>, header: Vec<Vec<Run>>, rows: Vec<Vec<Vec<Run>>> },
    Picture(Picture),
    Rule,
    PageBreak,
}

fn text(t: &str, s: Style) -> Run {
    Run::Text(t.to_string(), s)
}

struct Builder<'a> {
    doc: &'a Doc,
    items: Vec<Item>,
    /// The formatting of the `<div style>` the blocks are in.
    fmt: Option<crate::md::para::ParaStyle>,
}

impl Builder<'_> {
    fn para(&mut self, mut p: Para) {
        if p.fmt.is_none() {
            p.fmt = self.fmt.clone();
        }
        match self.items.last_mut() {
            Some(Item::Paras(v)) => v.push(p),
            _ => self.items.push(Item::Paras(vec![p])),
        }
    }

    /// A paragraph's runs, with its own pictures pulled out as picture items.
    fn inline_para(&mut self, inl: &[Inline], kind: Kind, level: usize, bullet: Option<String>) {
        if let [Inline::Image { url, alt, .. }] = inl {
            match self.doc.picture(url) {
                Some(p) => self.items.push(Item::Picture(p)),
                None => self.para(Para { fmt: None, runs: vec![text(&format!("[{alt}]"), Style::default())], kind, level, bullet }),
            }
            return;
        }
        self.para(Para { fmt: None, runs: runs(inl), kind, level, bullet });
    }

    fn blocks(&mut self, blocks: &[Block], kind: Kind, level: usize) {
        for b in blocks {
            self.block(b, kind, level);
        }
    }

    fn block(&mut self, b: &Block, kind: Kind, level: usize) {
        match b {
            Block::Heading { level: l, inlines, .. } => self.inline_para(inlines, Kind::Heading(*l), 0, None),
            Block::Paragraph { inlines, .. } => self.inline_para(inlines, kind, level, None),
            Block::List { ordered, start, items, .. } => {
                for (i, it) in items.iter().enumerate() {
                    let bullet = if *ordered { format!("{}.", start + i) } else { ["•", "◦", "▪"][level % 3].to_string() };
                    self.item(it, bullet, kind, level + 1);
                }
            }
            Block::Quote { blocks, .. } => self.blocks(blocks, Kind::Quote, level),
            Block::Styled { style, blocks, .. } => {
                let outer = self.fmt.replace(style.clone());
                self.blocks(blocks, kind, level);
                self.fmt = outer;
            }
            Block::Callout { kind: k, title, blocks, .. } => {
                let color = match k.to_lowercase().as_str() {
                    "tip" => 0x1A7F37,
                    "important" => 0x8250DF,
                    "warning" => 0x9A6700,
                    "caution" => 0xD1242F,
                    _ => 0x0969DA,
                };
                let t = title.clone().unwrap_or_else(|| {
                    let k = k.to_lowercase();
                    let mut c = k.chars();
                    c.next().map(|f| f.to_uppercase().collect::<String>() + c.as_str()).unwrap_or_default()
                });
                self.para(Para {
                    fmt: None,
                    runs: vec![text(&t, Style { bold: true, color: Some(format!("#{color:06X}")), ..Style::default() })],
                    kind: Kind::Callout(color),
                    level,
                    bullet: None,
                });
                self.blocks(blocks, Kind::Callout(color), level);
            }
            Block::Code { info, text: code, .. } => match Picture::diagram(info, code) {
                Some(p) => self.items.push(Item::Picture(p)),
                None => self.para(Para {
                    fmt: None,
                    runs: vec![text(code.trim_end_matches('\n'), Style::default())],
                    kind: Kind::Code,
                    level,
                    bullet: None,
                }),
            },
            Block::Math { text: m, .. } => self.para(Para {
                fmt: None,
                runs: vec![text(&crate::md::math::to_unicode(m), Style { italic: true, ..Style::default() })],
                kind: Kind::Center,
                level,
                bullet: None,
            }),
            Block::Table { aligns, header, rows, .. } => self.items.push(Item::Table {
                aligns: aligns.clone(),
                header: header.iter().map(|c| runs(c)).collect(),
                rows: rows.iter().map(|r| r.iter().map(|c| runs(c)).collect()).collect(),
            }),
            Block::Rule { .. } => self.items.push(Item::Rule),
            Block::PageBreak { .. } | Block::SlideBreak { .. } => self.items.push(Item::PageBreak),
            Block::Toc { .. } => {
                for h in crate::md::outline(&self.doc.parsed).iter().filter(|h| h.level <= 3) {
                    self.para(Para {
                        fmt: None,
                        runs: vec![text(&h.text, Style { color: Some("#0969DA".into()), ..Style::default() })],
                        kind: Kind::Body,
                        level: h.level as usize,
                        bullet: Some(String::new()),
                    });
                }
            }
            Block::Html { text: html, .. } => {
                let mut t = String::new();
                let mut tag = false;
                for c in html.chars() {
                    match c {
                        '<' => tag = true,
                        '>' if tag => tag = false,
                        c if !tag => t.push(c),
                        _ => {}
                    }
                }
                if !t.trim().is_empty() {
                    self.para(Para { fmt: None, runs: vec![text(t.trim(), Style::default())], kind, level, bullet: None });
                }
            }
            Block::Footnote { label, blocks, .. } => {
                let mut sub = Builder { doc: self.doc, items: Vec::new(), fmt: self.fmt.clone() };
                sub.blocks(blocks, Kind::Small, level);
                if let Some(Item::Paras(v)) = sub.items.first_mut()
                    && let Some(p) = v.first_mut()
                {
                    p.runs.insert(0, text(&format!("[{label}] "), Style { sup: true, ..Style::default() }));
                }
                for it in sub.items {
                    match it {
                        Item::Paras(v) => {
                            for p in v {
                                self.para(p);
                            }
                        }
                        other => self.items.push(other),
                    }
                }
            }
        }
    }

    fn item(&mut self, it: &ListItem, bullet: String, kind: Kind, level: usize) {
        for (k, b) in it.blocks.iter().enumerate() {
            match b {
                Block::Paragraph { inlines, .. } if k == 0 => {
                    let mut rs = runs(inlines);
                    if let Some(done) = it.task {
                        rs.insert(0, text(if done { "☑ " } else { "☐ " }, Style::default()));
                    }
                    let bullet = if it.task.is_some() { String::new() } else { bullet.clone() };
                    self.para(Para { fmt: None, runs: rs, kind, level, bullet: Some(bullet) });
                }
                Block::List { .. } => self.block(b, kind, level),
                other => self.block(other, kind, level),
            }
        }
    }
}

/// The blocks as items.
pub(crate) fn flow(doc: &Doc, blocks: &[Block]) -> Vec<Item> {
    let mut b = Builder { doc, items: Vec::new(), fmt: None };
    b.blocks(blocks, Kind::Body, 0);
    b.items
}

/// The plain text of runs.
pub(crate) fn runs_text(rs: &[Run]) -> String {
    rs.iter()
        .map(|r| match r {
            Run::Text(t, _) => t.clone(),
            Run::Break => "\n".into(),
            Run::Image(_, alt) => format!("[{alt}]"),
        })
        .collect()
}

/// Slide size in points (16:9).
pub(crate) const SLIDE_W: f32 = 960.0;
pub(crate) const SLIDE_H: f32 = 540.0;

/// A positioned box on a slide (points from the top left).
#[derive(Debug, Clone)]
pub(crate) enum SlideBox {
    Text { x: f32, y: f32, w: f32, h: f32, paras: Vec<Para>, size: f32, center: bool, title: bool },
    Table { x: f32, y: f32, w: f32, h: f32, item: Item, size: f32 },
    Picture { x: f32, y: f32, w: f32, h: f32, picture: Picture },
}

/// The size of a paragraph's text relative to the body.
pub(crate) fn scale(kind: Kind) -> f32 {
    match kind {
        Kind::Heading(1) => 1.9,
        Kind::Heading(2) => 1.5,
        Kind::Heading(3) => 1.25,
        Kind::Heading(_) => 1.1,
        Kind::Code => 0.8,
        Kind::Small => 0.75,
        _ => 1.0,
    }
}

/// Estimated height of paragraphs in a box `w` points wide at body size `size`.
pub(crate) fn text_height(paras: &[Para], w: f32, size: f32) -> f32 {
    paras
        .iter()
        .map(|p| {
            let s = size * scale(p.kind);
            let t = runs_text(&p.runs);
            let indent = p.level as f32 * s * 1.2;
            let per_line = ((w - indent) / (s * 0.52)).max(8.0);
            let lines: f32 = t.split('\n').map(|l| (l.chars().count() as f32 / per_line).ceil().max(1.0)).sum();
            lines * s * 1.25 + s * 0.45
        })
        .sum()
}

/// Lay out a slide: the title on top (or everything centred for title, section, end, stat
/// and quote slides), then text, tables and pictures down the slide, shrunk to fit.
pub(crate) fn slide_layout(doc: &Doc, s: &super::SlideDoc) -> Vec<SlideBox> {
    let items = flow(doc, &s.parsed.blocks);
    let centred = matches!(s.class.as_str(), "title" | "section" | "end" | "stat" | "quote" | "center");
    let pad = if s.class == "blank" { 0.0 } else { 48.0 };
    let (w, top_y, bottom) = (SLIDE_W - 2.0 * pad, pad * 0.75, SLIDE_H - pad * 0.75);
    let only_text = items.iter().all(|i| matches!(i, Item::Paras(_)));
    if centred && only_text {
        let paras: Vec<Para> = items.into_iter().flat_map(|i| if let Item::Paras(v) = i { v } else { Vec::new() }).collect();
        let mut size = 26.0f32;
        while size > 12.0 && text_height(&paras, w, size) > bottom - top_y {
            size -= 1.0;
        }
        return vec![SlideBox::Text { x: pad, y: top_y, w, h: bottom - top_y, paras, size, center: true, title: false }];
    }
    let mut out = Vec::new();
    let mut rest = items;
    let mut y = top_y;
    // the title: the first paragraph when it is a heading
    if let Some(Item::Paras(v)) = rest.first_mut()
        && v.first().is_some_and(|p| matches!(p.kind, Kind::Heading(_)))
    {
        let t = v.remove(0);
        if v.is_empty() {
            rest.remove(0);
        }
        out.push(SlideBox::Text { x: pad, y, w, h: 64.0, paras: vec![t], size: 20.0, center: false, title: true });
        y += 80.0;
    }
    // natural heights at the body size
    let body = 22.0f32;
    let room = bottom - y;
    let need = |it: &Item, size: f32| -> f32 {
        match it {
            Item::Paras(v) => text_height(v, w, size),
            Item::Table { rows, .. } => (rows.len() as f32 + 1.0) * size * 1.7,
            // pictures fill the width (or the height left), charts read better large
            Item::Picture(p) => (w * p.height.max(1) as f32 / p.width.max(1) as f32).min(room),
            Item::Rule => 12.0,
            Item::PageBreak => 0.0,
        }
    };
    let avail = bottom - y;
    let total: f32 = rest.iter().map(|i| need(i, body)).sum();
    let k = if total > avail && total > 0.0 { (avail / total).max(0.35) } else { 1.0 };
    let size = (body * k).max(11.0);
    for it in rest {
        let h = need(&it, body) * k;
        match it {
            Item::Paras(paras) => {
                let h = text_height(&paras, w, size).max(h);
                out.push(SlideBox::Text { x: pad, y, w, h, paras, size, center: centred, title: false });
                y += h;
            }
            Item::Table { .. } => {
                out.push(SlideBox::Table { x: pad, y, w, h, item: it, size: (size * 0.8).max(10.0) });
                y += h + 10.0;
            }
            Item::Picture(p) => {
                let ratio = p.width.max(1) as f32 / p.height.max(1) as f32;
                let (pw, ph) = ((h * ratio).min(w), h);
                let x = pad + (w - pw) / 2.0;
                out.push(SlideBox::Picture { x, y, w: pw, h: ph, picture: p });
                y += ph + 8.0;
            }
            Item::Rule | Item::PageBreak => y += h,
        }
    }
    out
}
