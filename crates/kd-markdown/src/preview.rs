//! The preview, drawn natively: the document as MdCraft's page shows it (GitHub-like styles,
//! light and dark, the document theme's heading colour). Charts and Mermaid flowcharts, pie
//! charts, sequence diagrams and Gantt charts are drawn as SVG (kd-formats' `diagram`); math is
//! shown as Unicode; other diagram kinds show their source. Slide decks show one 16:9 frame per
//! slide with its notes.

use std::cell::RefCell;
use std::collections::HashMap;
use std::hash::{Hash, Hasher};
use std::path::{Path, PathBuf};
use std::rc::Rc;
use std::sync::Arc;

use gpui::{
    AnyElement, App, Context, ElementId, Entity, EventEmitter, FontStyle, FontWeight, Hsla, InteractiveText, IntoElement, MouseButton,
    MouseDownEvent, ObjectFit, ScrollHandle, SharedString, StrikethroughStyle, StyledText, TextRun, UnderlineStyle, Window, div, img,
    prelude::*, px, rgb,
};
use kd_formats::md::{self, Align, Block, Inline, ListItem, Parsed, library::ThemeInfo};
use kd_ui::{Theme, mix};

use crate::editor::mono_family;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PreviewEvent {
    /// Double-click on a block: show its source line (1-based).
    GoToLine(usize),
}

/// The document's colours (MdCraft's markdown.css).
#[derive(Clone, Copy)]
pub struct DocColors {
    pub fg: Hsla,
    pub muted: Hsla,
    pub canvas: Hsla,
    pub subtle: Hsla,
    pub border: Hsla,
    pub link: Hsla,
    pub mark: Hsla,
    pub code_bg: Hsla,
    pub note: Hsla,
    pub tip: Hsla,
    pub important: Hsla,
    pub warning: Hsla,
    pub caution: Hsla,
}

impl DocColors {
    pub fn new(dark: bool) -> DocColors {
        let c = |l: u32, d: u32| -> Hsla { rgb(if dark { d } else { l }).into() };
        DocColors {
            fg: c(0x1f2328, 0xf0f6fc),
            muted: c(0x59636e, 0x9198a1),
            canvas: c(0xffffff, 0x0d1117),
            subtle: c(0xf6f8fa, 0x151b23),
            border: c(0xd1d9e0, 0x3d444d),
            link: c(0x0969da, 0x4493f8),
            mark: if dark { Hsla::from(rgb(0xbb8009)).opacity(0.25) } else { rgb(0xfff8c5).into() },
            code_bg: if dark { Hsla::from(rgb(0x656c76)).opacity(0.2) } else { Hsla::from(rgb(0x818b98)).opacity(0.12) },
            note: c(0x0969da, 0x4493f8),
            tip: c(0x1a7f37, 0x3fb950),
            important: c(0x8250df, 0xab7df8),
            warning: c(0x9a6700, 0xd29922),
            caution: c(0xd1242f, 0xf85149),
        }
    }

    fn callout(&self, kind: &str) -> (Hsla, &'static str) {
        match kind.to_lowercase().as_str() {
            "tip" => (self.tip, "✓"),
            "important" => (self.important, "!"),
            "warning" => (self.warning, "⚠"),
            "caution" => (self.caution, "⛔"),
            _ => (self.note, "ℹ"),
        }
    }
}

/// The body font of the document: the system's on macOS and Windows; on Linux the first of
/// the usual sans-serif families that is installed.
pub fn body_family(cx: &App) -> SharedString {
    static LINUX: std::sync::OnceLock<SharedString> = std::sync::OnceLock::new();
    if cfg!(target_os = "macos") {
        ".SystemUIFont".into()
    } else if cfg!(windows) {
        "Segoe UI".into()
    } else {
        LINUX
            .get_or_init(|| {
                let names = cx.text_system().all_font_names();
                let f: SharedString = ["Inter", "Cantarell", "Ubuntu", "Noto Sans", "Open Sans", "Liberation Sans", "DejaVu Sans"]
                    .into_iter()
                    .find(|f| names.iter().any(|n| n == f))
                    .unwrap_or("DejaVu Sans")
                    .into();
                f
            })
            .clone()
    }
}

/// Code fences that MdCraft draws as diagrams or charts.
pub fn is_diagram(lang: &str) -> bool {
    matches!(
        lang,
        "mermaid"
            | "plantuml"
            | "puml"
            | "graphviz"
            | "dot"
            | "d2"
            | "vega"
            | "vega-lite"
            | "chart"
            | "chartjs"
            | "plotly"
            | "wavedrom"
            | "bpmn"
            | "nomnoml"
            | "svgbob"
            | "pikchr"
            | "excalidraw"
            | "smiles"
            | "abc"
            | "flowchart"
            | "sequence"
            | "echarts"
            | "markmap"
            | "molstar"
    )
}

pub struct Preview {
    pub parsed: Parsed,
    version: u64,
    pub zoom: f32,
    pub base_dir: Option<PathBuf>,
    pub scroll: ScrollHandle,
    /// In the dark interface, the page is dark too (View ▸ Dark page).
    pub dark_page: bool,
    /// View ▸ Page width: the page fills the pane.
    pub page_width: bool,
    /// The slides, when the document is a deck.
    pub deck: Vec<SlideDoc>,
    pub theme: Option<&'static ThemeInfo>,
    diagrams: Diagrams,
    /// The slide width of the last frame (a new width draws again).
    slide_w: f32,
}

/// Drawn diagrams by the hash of their source and colours.
type Diagrams = Rc<RefCell<HashMap<u64, Option<(Arc<gpui::RenderImage>, f32, f32)>>>>;

/// One slide of a deck, parsed.
pub struct SlideDoc {
    pub parsed: Parsed,
    pub notes: Option<Parsed>,
    pub class: String,
    pub background: Option<String>,
    /// Lines before the slide's content (to map its blocks to source lines).
    pub line_offset: usize,
    /// The slide's first line (1-based).
    pub line: usize,
}

/// `#rgb`, `#rrggbb` or a few colour names.
pub fn parse_color(c: &str) -> Option<Hsla> {
    let c = c.trim();
    if let Some(h) = c.strip_prefix('#') {
        let v = match h.len() {
            3 => {
                let n = u32::from_str_radix(h, 16).ok()?;
                (((n >> 8) & 0xf) * 0x11) << 16 | (((n >> 4) & 0xf) * 0x11) << 8 | ((n & 0xf) * 0x11)
            }
            6 => u32::from_str_radix(h, 16).ok()?,
            _ => return None,
        };
        return Some(rgb(v).into());
    }
    let v = match c.to_ascii_lowercase().as_str() {
        "red" => 0xff0000,
        "green" => 0x008000,
        "blue" => 0x0000ff,
        "yellow" => 0xffff00,
        "orange" => 0xffa500,
        "purple" => 0x800080,
        "gray" | "grey" => 0x808080,
        "black" => 0x000000,
        "white" => 0xffffff,
        "lime" => 0x00ff00,
        "cyan" | "aqua" => 0x00ffff,
        "magenta" | "fuchsia" => 0xff00ff,
        "navy" => 0x000080,
        "teal" => 0x008080,
        "maroon" => 0x800000,
        "olive" => 0x808000,
        "silver" => 0xc0c0c0,
        _ => return None,
    };
    Some(rgb(v).into())
}

fn is_dark(c: Hsla) -> bool {
    c.l < 0.45
}

impl EventEmitter<PreviewEvent> for Preview {}

impl Preview {
    pub fn new(_cx: &mut Context<Self>) -> Self {
        Preview {
            parsed: Parsed::default(),
            version: u64::MAX,
            zoom: 1.0,
            base_dir: None,
            scroll: ScrollHandle::new(),
            dark_page: true,
            page_width: false,
            deck: Vec::new(),
            theme: None,
            diagrams: Rc::default(),
            slide_w: 0.0,
        }
    }

    /// Parse `text` again if it changed (`version` is the document's).
    pub fn update_text(&mut self, text: &str, version: u64, cx: &mut Context<Self>) {
        if version != self.version {
            self.parsed = md::parse(text);
            self.theme = self.parsed.meta("theme").and_then(md::library::theme);
            self.deck = md::slides::slides(text)
                .iter()
                .map(|s| {
                    let (content, notes) = s.content_and_notes(text);
                    let content_start = content.as_ptr() as usize - text.as_ptr() as usize;
                    SlideDoc {
                        parsed: md::parse(content),
                        notes: notes.map(md::parse),
                        class: s.class().to_string(),
                        background: s.attr("data-background-color").map(str::to_string),
                        line_offset: text[..content_start].matches('\n').count(),
                        line: text[..s.start].matches('\n').count() + 1,
                    }
                })
                .collect();
            self.version = version;
            // drawn diagrams of older text are dropped now and then
            if self.diagrams.borrow().len() > 200 {
                self.diagrams.borrow_mut().clear();
            }
            cx.notify();
        }
    }

    /// The slide that holds source `line` (1-based).
    pub fn slide_at_line(&self, line: usize) -> Option<usize> {
        (!self.deck.is_empty()).then(|| self.deck.iter().rposition(|s| s.line <= line).unwrap_or(0))
    }

    /// Scroll to the block that holds source `line` (1-based): to the top with `top`, else
    /// only as far as needed to show it.
    pub fn reveal_line(&self, line: usize, top: bool) {
        // child 0 is the front matter; in a deck, one child per slide
        let ix = match self.slide_at_line(line) {
            Some(i) => i + 1,
            None => self.parsed.blocks.iter().rposition(|b| b.line() <= line).map(|i| i + 1).unwrap_or(0),
        };
        if top {
            self.scroll.scroll_to_top_of_item(ix);
        } else {
            self.scroll.scroll_to_item(ix);
        }
    }
}

/// What a render pass needs.
struct R {
    col: DocColors,
    /// Base font size (px).
    size: f32,
    body: SharedString,
    mono: SharedString,
    n: usize,
    base_dir: Option<PathBuf>,
    preview: Entity<Preview>,
    outline: Vec<md::Heading>,
    dark: bool,
    /// Added to block lines (a slide's blocks are parsed on their own).
    line_offset: usize,
    heading: Option<Hsla>,
    diagrams: Diagrams,
    /// Inside a slide: headings without rules or top margin.
    slide: bool,
}

impl R {
    fn id(&mut self, what: &str) -> ElementId {
        self.n += 1;
        ElementId::Name(format!("pv-{what}-{}", self.n).into())
    }

    fn font(&self, mono: bool) -> gpui::Font {
        gpui::font(if mono { self.mono.clone() } else { self.body.clone() })
    }

    /// A block that jumps to its source line on double-click.
    fn block(&self, line: usize) -> gpui::Div {
        let p = self.preview.clone();
        let line = line + self.line_offset;
        div().w_full().on_mouse_down(MouseButton::Left, move |e: &MouseDownEvent, _, cx: &mut App| {
            if e.click_count == 2 {
                p.update(cx, |_, cx| cx.emit(PreviewEvent::GoToLine(line)));
            }
        })
    }

    /// Inline content as one text element (links clickable).
    fn inlines(&mut self, inl: &[Inline], color: Hsla, weight: FontWeight, mono: bool) -> AnyElement {
        let mut text = String::new();
        let mut runs: Vec<TextRun> = Vec::new();
        let mut links: Vec<(std::ops::Range<usize>, String)> = Vec::new();
        let base = TextRun {
            len: 0,
            font: gpui::Font { weight, ..self.font(mono) },
            color,
            background_color: None,
            underline: None,
            strikethrough: None,
        };
        let push = |text: &mut String, runs: &mut Vec<TextRun>, s: &str, r: TextRun| {
            if s.is_empty() {
                return;
            }
            text.push_str(s);
            runs.push(TextRun { len: s.len(), ..r });
        };
        for i in inl {
            match i {
                Inline::Text(t, st) => {
                    let mut r = base.clone();
                    if st.code {
                        r.font = gpui::Font { weight, ..self.font(true) };
                        r.background_color = Some(self.col.code_bg);
                    }
                    if st.bold {
                        r.font.weight = FontWeight::BOLD;
                    }
                    if st.italic {
                        r.font.style = FontStyle::Italic;
                    }
                    if st.strike {
                        r.strikethrough = Some(StrikethroughStyle { thickness: px(1.0), color: Some(color) });
                    }
                    if st.highlight {
                        r.background_color = Some(self.col.mark);
                    }
                    if st.underline {
                        r.underline = Some(UnderlineStyle { thickness: px(1.0), color: Some(color), wavy: false });
                    }
                    if st.sup || st.sub {
                        r.color = mix_h(color, self.col.muted);
                    }
                    if let Some(c) = st.color.as_deref().and_then(parse_color) {
                        r.color = c;
                    }
                    if let Some(c) = st.background.as_deref().and_then(parse_color) {
                        r.background_color = Some(c.opacity(if self.dark { 0.45 } else { 1.0 }));
                    }
                    if let Some(f) = &st.font {
                        r.font.family = f.clone().into();
                    }
                    if let Some(url) = &st.link {
                        r.color = self.col.link;
                        let start = text.len();
                        push(&mut text, &mut runs, t, r);
                        links.push((start..text.len(), url.clone()));
                    } else {
                        push(&mut text, &mut runs, t, r);
                    }
                }
                Inline::Image { alt, .. } => {
                    let r = TextRun { color: self.col.muted, ..base.clone() };
                    push(&mut text, &mut runs, &format!("🖼 {alt}"), r);
                }
                Inline::Math { text: m, display } => {
                    let r = TextRun { font: gpui::Font { style: FontStyle::Italic, ..self.font(false) }, ..base.clone() };
                    let u = md::math::to_unicode(m);
                    if *display {
                        push(&mut text, &mut runs, &format!("\n{u}\n"), r);
                    } else {
                        push(&mut text, &mut runs, &u, r);
                    }
                }
                Inline::FootnoteRef(l) => {
                    let r = TextRun { color: self.col.link, ..base.clone() };
                    push(&mut text, &mut runs, &format!("[{l}]"), r);
                }
                Inline::Break => push(&mut text, &mut runs, "\n", base.clone()),
            }
        }
        if text.is_empty() {
            return div().into_any_element();
        }
        let styled = StyledText::new(text).with_runs(runs);
        if links.is_empty() {
            return styled.into_any_element();
        }
        let urls: Vec<String> = links.iter().map(|(_, u)| u.clone()).collect();
        let ranges = links.into_iter().map(|(r, _)| r).collect();
        let outline = self.outline.clone();
        let p = self.preview.clone();
        InteractiveText::new(self.id("link"), styled)
            .on_click(ranges, move |ix, _, cx| {
                let url = &urls[ix];
                if let Some(anchor) = url.strip_prefix('#') {
                    // a heading in this document
                    if let Some(h) = outline.iter().find(|h| slug(&h.text) == anchor) {
                        let line = h.line;
                        p.update(cx, |_, cx| cx.emit(PreviewEvent::GoToLine(line)));
                    }
                } else {
                    cx.open_url(url);
                }
            })
            .into_any_element()
    }

    fn blocks(&mut self, blocks: &[Block], depth: usize) -> Vec<AnyElement> {
        blocks.iter().filter_map(|b| self.render_block(b, depth)).collect()
    }

    fn render_block(&mut self, b: &Block, depth: usize) -> Option<AnyElement> {
        let s = self.size;
        let col = self.col;
        let gap = px(s);
        Some(match b {
            Block::Heading { level, inlines, line } => {
                let (k, rule) = match level {
                    1 => (2.0, true),
                    2 => (1.5, true),
                    3 => (1.25, false),
                    4 => (1.0, false),
                    5 => (0.875, false),
                    _ => (0.85, false),
                };
                let color = if *level >= 6 {
                    col.muted
                } else if *level <= 2 {
                    self.heading.unwrap_or(col.fg)
                } else {
                    col.fg
                };
                let text = self.inlines(inlines, color, FontWeight::SEMIBOLD, false);
                let rule = rule && !self.slide;
                self.block(*line)
                    .when(!self.slide, |d| d.mt(px(s * 1.5)))
                    .mb(gap)
                    .text_size(px(s * k))
                    .line_height(px(s * k * 1.25))
                    .when(rule, |d| d.pb(px(s * k * 0.3)).border_b_1().border_color(col.border.opacity(0.7)))
                    .child(text)
                    .into_any_element()
            }
            Block::Paragraph { inlines, line } => {
                // a picture on its own line is shown as a picture
                if let [Inline::Image { url, alt, .. }] = inlines.as_slice() {
                    return Some(self.block(*line).mb(gap).child(self.image(url, alt)).into_any_element());
                }
                // @import "file": its contents
                if let [Inline::Text(t, _)] = inlines.as_slice()
                    && let Some(path) =
                        t.trim().strip_prefix("@import").map(|r| r.trim().trim_start_matches('"')).map(|r| r.split('"').next().unwrap_or(r))
                {
                    let el = self.include(path, depth);
                    return Some(self.block(*line).mb(gap).child(el).into_any_element());
                }
                let text = self.inlines(inlines, col.fg, FontWeight::NORMAL, false);
                self.block(*line).mb(gap).child(text).into_any_element()
            }
            Block::List { ordered, start, tight, items, line } => {
                let rows: Vec<AnyElement> =
                    items.iter().enumerate().map(|(i, it)| self.item(it, *ordered, start + i, *tight, depth)).collect();
                self.block(*line)
                    .mb(if depth == 0 { gap } else { px(0.0) })
                    .flex()
                    .flex_col()
                    .gap(px(s * 0.25))
                    .children(rows)
                    .into_any_element()
            }
            Block::Quote { blocks, line } => {
                let inner = self.with_color(col.muted, |r| r.blocks(blocks, depth));
                self.block(*line)
                    .mb(gap)
                    .pl(px(s))
                    .border_l(px(s * 0.25))
                    .border_color(col.border)
                    .text_color(col.muted)
                    .child(last_flush(inner))
                    .into_any_element()
            }
            Block::Callout { kind, title, blocks, line } => {
                let (color, sym) = col.callout(kind);
                let title = title.clone().filter(|t| !t.eq_ignore_ascii_case(kind)).unwrap_or_else(|| capitalise(&kind.to_lowercase()));
                let inner = self.blocks(blocks, depth);
                self.block(*line)
                    .mb(gap)
                    .py(px(s * 0.5))
                    .px(px(s))
                    .border_l(px(s * 0.25))
                    .border_color(color)
                    .child(
                        div().flex().gap(px(6.0)).mb(px(8.0)).text_color(color).font_weight(FontWeight::SEMIBOLD).child(sym).child(title),
                    )
                    .child(last_flush(inner))
                    .into_any_element()
            }
            Block::Code { lang, info, text, line } => {
                if let Some((image, w, h)) = self.diagram(info, text) {
                    let el = img(image).w_full().max_w(px(w)).h(px(h)).object_fit(ObjectFit::Contain);
                    return Some(self.block(*line).mb(gap).flex().justify_center().child(el).into_any_element());
                }
                let diagram = is_diagram(lang);
                let mono = self.mono.clone();
                let body = text.trim_end_matches('\n').to_string();
                let code = div()
                    .p(px(s))
                    .bg(col.subtle)
                    .rounded(px(6.0))
                    .font_family(mono)
                    .text_size(px(s * 0.85))
                    .line_height(px(s * 0.85 * 1.45))
                    .text_color(if diagram { col.muted } else { col.fg })
                    .overflow_hidden()
                    .child(body);
                if diagram {
                    self.block(*line)
                        .mb(gap)
                        .flex()
                        .flex_col()
                        .gap(px(8.0))
                        .child(
                            div()
                                .p(px(10.0))
                                .rounded(px(6.0))
                                .border_1()
                                .border_dashed()
                                .border_color(col.border)
                                .text_color(col.muted)
                                .text_size(px(13.0))
                                .flex()
                                .justify_center()
                                .child(format!("{} — drawn in a later version", diagram_name(lang))),
                        )
                        .child(code.opacity(0.6))
                        .into_any_element()
                } else {
                    self.block(*line).mb(gap).child(code).into_any_element()
                }
            }
            Block::Math { text, line } => self
                .block(*line)
                .mb(gap)
                .flex()
                .flex_col()
                .items_center()
                .italic()
                .text_size(px(s * 1.1))
                .children(md::math::to_unicode(text).lines().map(|l| div().child(l.to_string())).collect::<Vec<_>>())
                .into_any_element(),
            Block::Table { aligns, header, rows, line } => self.table(aligns, header, rows, *line),
            Block::Rule { line } => self.block(*line).my(px(s * 1.5)).h(px(s * 0.25)).bg(col.border).into_any_element(),
            Block::PageBreak { line } => self.divider(*line, "Page break"),
            Block::SlideBreak { line } => self.divider(*line, "Slide"),
            Block::Toc { line } => {
                let rows: Vec<AnyElement> = self
                    .outline
                    .clone()
                    .into_iter()
                    .filter(|h| h.level <= 3)
                    .map(|h| {
                        let p = self.preview.clone();
                        let target = h.line;
                        div()
                            .id(self.id("toc"))
                            .pl(px((h.level as f32 - 1.0) * s * 1.2))
                            .text_color(col.link)
                            .cursor_pointer()
                            .on_click(move |_, _, cx| p.update(cx, |_, cx| cx.emit(PreviewEvent::GoToLine(target))))
                            .child(h.text)
                            .into_any_element()
                    })
                    .collect();
                self.block(*line).mb(gap).flex().flex_col().gap(px(2.0)).children(rows).into_any_element()
            }
            Block::Html { text, line } => {
                let t = text.trim();
                if t.starts_with("<!--") || t.is_empty() {
                    return None;
                }
                // layout tags (<div class="cover">, </div>, <details> …) wrap Markdown: their
                // content shows; the tags themselves do not
                let plain = strip_tags(t);
                if plain.trim().is_empty() {
                    return None;
                }
                if t.starts_with('<') && !t.starts_with("<pre") && !t.starts_with("<script") && !t.starts_with("<style") {
                    let bold = t.contains("<summary") || t.contains("<b>") || t.contains("<strong>");
                    return Some(
                        self.block(*line)
                            .mb(gap)
                            .when(bold, |d| d.font_weight(FontWeight::SEMIBOLD))
                            .children(plain.lines().map(|l| div().child(l.trim().to_string())).collect::<Vec<_>>())
                            .into_any_element(),
                    );
                }
                self.block(*line)
                    .mb(gap)
                    .font_family(self.mono.clone())
                    .text_size(px(s * 0.8))
                    .text_color(col.muted)
                    .child(t.to_string())
                    .into_any_element()
            }
            Block::Footnote { label, blocks, line } => {
                let inner = self.with_size(12.0f32.max(s * 0.75), |r| r.blocks(blocks, depth));
                self.block(*line)
                    .mt(px(s))
                    .pt(px(8.0))
                    .border_t_1()
                    .border_color(col.border)
                    .text_size(px(12.0f32.max(s * 0.75)))
                    .text_color(col.muted)
                    .flex()
                    .gap(px(6.0))
                    .child(div().text_color(col.link).child(format!("[{label}]")))
                    .child(div().flex_1().min_w_0().child(last_flush(inner)))
                    .into_any_element()
            }
        })
    }

    fn with_color<T>(&mut self, fg: Hsla, f: impl FnOnce(&mut R) -> T) -> T {
        let old = self.col.fg;
        self.col.fg = fg;
        let out = f(self);
        self.col.fg = old;
        out
    }

    fn with_size<T>(&mut self, size: f32, f: impl FnOnce(&mut R) -> T) -> T {
        let old = self.size;
        self.size = size;
        let out = f(self);
        self.size = old;
        out
    }

    fn item(&mut self, it: &ListItem, ordered: bool, n: usize, tight: bool, depth: usize) -> AnyElement {
        let s = self.size;
        let col = self.col;
        let marker: AnyElement = match it.task {
            Some(done) => div()
                .mt(px(s * 0.3))
                .size(px(s * 0.85))
                .rounded(px(3.0))
                .border_1()
                .border_color(if done { col.link } else { col.muted })
                .when(done, |d| {
                    d.bg(col.link).flex().items_center().justify_center().child(kd_ui::icon(kd_ui::Icon::Check, s * 0.7, col.canvas))
                })
                .into_any_element(),
            None if ordered => div().text_color(col.fg).child(format!("{n}.")).into_any_element(),
            None => div().text_color(col.fg).child(["•", "◦", "▪"][depth % 3]).into_any_element(),
        };
        let blocks: Vec<AnyElement> = it
            .blocks
            .iter()
            .filter_map(|b| {
                // tight lists: paragraphs without their margin
                if tight && let Block::Paragraph { inlines, line } = b {
                    let text = self.inlines(inlines, col.fg, FontWeight::NORMAL, false);
                    return Some(self.block(*line).child(text).into_any_element());
                }
                self.render_block(b, depth + 1)
            })
            .collect();
        let done = it.task == Some(true);
        div()
            .flex()
            .gap(px(s * 0.5))
            .child(div().w(px(s * 1.5)).flex().justify_end().flex_none().child(marker))
            .child(div().flex_1().min_w_0().flex().flex_col().when(done, |d| d.text_color(col.muted)).children(blocks))
            .into_any_element()
    }

    fn table(&mut self, aligns: &[Align], header: &[Vec<Inline>], rows: &[Vec<Vec<Inline>>], line: usize) -> AnyElement {
        let col = self.col;
        let s = self.size;
        let n = header.len().max(1);
        let mut cells: Vec<AnyElement> = Vec::new();
        let align = |d: gpui::Div, i: usize| match aligns.get(i) {
            Some(Align::Center) => d.flex().justify_center(),
            Some(Align::Right) => d.flex().justify_end(),
            _ => d,
        };
        for (i, h) in header.iter().enumerate() {
            let t = self.inlines(h, col.fg, FontWeight::SEMIBOLD, false);
            cells.push(
                align(div().px(px(13.0)).py(px(6.0)).min_h(px(s * 1.6 + 12.0)).border_1().border_color(col.border).bg(col.canvas), i)
                    .child(t)
                    .into_any_element(),
            );
        }
        for (r, row) in rows.iter().enumerate() {
            for i in 0..n {
                let t = row.get(i).map(|c| self.inlines(c, col.fg, FontWeight::NORMAL, false));
                let bg = if r % 2 == 0 { col.subtle } else { col.canvas };
                cells.push(
                    align(div().px(px(13.0)).py(px(6.0)).min_h(px(s * 1.6 + 12.0)).border_1().border_color(col.border).bg(bg), i)
                        .children(t)
                        .into_any_element(),
                );
            }
        }
        self.block(line).mb(px(s)).child(div().grid().grid_cols(n as u16).text_size(px(s * 0.95)).children(cells)).into_any_element()
    }

    fn divider(&mut self, line: usize, label: &str) -> AnyElement {
        let col = self.col;
        self.block(line)
            .my(px(self.size))
            .flex()
            .items_center()
            .gap(px(10.0))
            .text_size(px(11.0))
            .text_color(col.muted)
            .child(div().flex_1().h(px(1.0)).border_t_1().border_dashed().border_color(col.border))
            .child(label.to_string())
            .child(div().flex_1().h(px(1.0)).border_t_1().border_dashed().border_color(col.border))
            .into_any_element()
    }

    fn image(&mut self, url: &str, alt: &str) -> AnyElement {
        let col = self.col;
        let local = !url.contains("://") && !url.starts_with("data:");
        let path = local.then(|| resolve(self.base_dir.as_deref(), url)).flatten();
        match path {
            Some(p) => div().flex().justify_center().child(img(p).max_w_full()).into_any_element(),
            None => div()
                .p(px(10.0))
                .rounded(px(6.0))
                .border_1()
                .border_dashed()
                .border_color(col.border)
                .text_color(col.muted)
                .text_size(px(13.0))
                .child(format!("🖼 {}  ({url})", if alt.is_empty() { "Picture" } else { alt }))
                .into_any_element(),
        }
    }

    /// A chart or diagram drawn as SVG (cached), when its kind is drawn natively.
    fn diagram(&mut self, info: &str, code: &str) -> Option<(Arc<gpui::RenderImage>, f32, f32)> {
        let mut h = std::collections::hash_map::DefaultHasher::new();
        (info, code, self.dark).hash(&mut h);
        let key = h.finish();
        if let Some(v) = self.diagrams.borrow().get(&key) {
            return v.clone();
        }
        let v = md::diagram::render(info, code, self.dark)
            .and_then(|svg| crate::raster::svg_image(&svg.svg, 2.0).map(|i| (i, svg.width, svg.height)));
        self.diagrams.borrow_mut().insert(key, v.clone());
        v
    }

    /// `@import "file"`: a CSV file as a table, Markdown as its blocks, other text as code.
    fn include(&mut self, path: &str, depth: usize) -> AnyElement {
        let col = self.col;
        let note = |t: String| div().text_size(px(13.0)).text_color(col.muted).child(t).into_any_element();
        let Some(file) = resolve(self.base_dir.as_deref(), path) else {
            return note(format!("Included file not found: {path}"));
        };
        let Ok(text) = std::fs::read_to_string(&file) else {
            return note(format!("Included file is not text: {path}"));
        };
        if depth > 2 {
            return note(format!("Included too deep: {path}"));
        }
        let ext = file.extension().and_then(|e| e.to_str()).unwrap_or("").to_ascii_lowercase();
        let md_text = match ext.as_str() {
            "csv" | "tsv" => md::review::csv_to_table(&text),
            "md" | "markdown" | "mdd" | "mds" => text,
            _ => format!("```{ext}\n{}\n```", text.trim_end()),
        };
        let parsed = md::parse(&md_text);
        let saved = std::mem::replace(&mut self.line_offset, usize::MAX / 2);
        let children = self.blocks(&parsed.blocks, depth + 1);
        self.line_offset = saved;
        last_flush(children).into_any_element()
    }
}

/// The colours of a slide: its background, text and headings.
fn slide_colors(theme: Option<&'static ThemeInfo>, background: Option<&str>, class: &str, dark_page: bool) -> (Hsla, Hsla, Hsla) {
    let deck = theme.filter(|t| t.deck);
    let (mut bg, mut fg, mut head): (Hsla, Hsla, Hsla) = match deck {
        Some(t) => {
            let bg: Hsla = rgb(t.colors[0]).into();
            let fg: Hsla = if is_dark(bg) { rgb(0xf3f4f6).into() } else { rgb(0x1f2328).into() };
            // the theme colour that stands out most against the background
            let (a, b): (Hsla, Hsla) = (rgb(t.colors[1]).into(), rgb(t.colors[2]).into());
            let head = if (a.l - bg.l).abs() >= (b.l - bg.l).abs() { a } else { b };
            (bg, fg, head)
        }
        None if dark_page => (rgb(0x0d1117).into(), rgb(0xf0f6fc).into(), rgb(0xf0f6fc).into()),
        None => (gpui::white(), rgb(0x1f2328).into(), rgb(0x1f2328).into()),
    };
    if class == "section"
        && let Some(t) = deck
    {
        bg = rgb(t.colors[1]).into();
        fg = if is_dark(bg) { gpui::white() } else { rgb(0x111111).into() };
        head = fg;
    }
    if let Some(c) = background.and_then(parse_color) {
        bg = c;
        // keep the text readable on the chosen colour
        let fg_dark = is_dark(fg);
        if is_dark(bg) == fg_dark {
            fg = if is_dark(bg) { gpui::white() } else { rgb(0x1f2328).into() };
            head = fg;
        }
    }
    if is_dark(bg) == is_dark(head) {
        head = fg;
    }
    (bg, fg, head)
}

impl Preview {
    fn r(&self, size: f32, dark: bool, cx: &mut Context<Self>) -> R {
        R {
            col: DocColors::new(dark),
            size,
            body: body_family(cx),
            mono: mono_family(),
            n: 0,
            base_dir: self.base_dir.clone(),
            preview: cx.entity(),
            outline: md::outline(&self.parsed),
            dark,
            line_offset: 0,
            heading: None,
            diagrams: self.diagrams.clone(),
            slide: false,
        }
    }

    /// Slide `i` drawn `width` pixels wide (16:9), for the preview and the slide show.
    pub fn slide_frame(&mut self, i: usize, width: f32, cx: &mut Context<Self>) -> Option<AnyElement> {
        let pal = Theme::palette(cx);
        let s = self.deck.get(i)?;
        let (bg, fg, head) = slide_colors(self.theme, s.background.as_deref(), &s.class, pal.dark && self.dark_page);
        let size = width / 960.0 * 26.0;
        let mut r = self.r(size, is_dark(bg), cx);
        r.col.fg = fg;
        r.col.canvas = bg;
        r.heading = Some(head);
        r.line_offset = s.line_offset;
        r.slide = true;
        r.n = i * 10_000;
        let blocks = r.blocks(&s.parsed.blocks.clone(), 0);
        let centred = matches!(s.class.as_str(), "title" | "section" | "end" | "stat" | "quote" | "center");
        let pad = if s.class == "blank" { 0.0 } else { width / 960.0 * 48.0 };
        let p = cx.entity();
        let line = s.line;
        Some(
            div()
                .relative()
                .w(px(width))
                .h(px(width * 9.0 / 16.0))
                .flex_none()
                .overflow_hidden()
                .rounded(px(6.0))
                .border(px(1.5))
                .border_color(Hsla::from(pal.border_strong))
                .bg(bg)
                .text_color(fg)
                .text_size(px(size))
                .line_height(px(size * 1.45))
                .font_family(r.body.clone())
                .p(px(pad))
                .flex()
                .flex_col()
                .justify_center()
                .when(centred, |d| d.items_center().text_center())
                .on_mouse_down(MouseButton::Left, move |e: &MouseDownEvent, _, cx: &mut App| {
                    if e.click_count == 2 {
                        p.update(cx, |_, cx| cx.emit(PreviewEvent::GoToLine(line)));
                    }
                })
                .child(div().flex().flex_col().when(centred, |d| d.items_center()).children(blocks))
                .child(
                    div()
                        .absolute()
                        .right(px(12.0))
                        .bottom(px(6.0))
                        .text_size(px(12.0))
                        .text_color(fg.opacity(0.6))
                        .child((i + 1).to_string()),
                )
                .into_any_element(),
        )
    }

    /// The notes of slide `i`.
    pub fn slide_notes(&mut self, i: usize, size: f32, cx: &mut Context<Self>) -> Option<AnyElement> {
        let pal = Theme::palette(cx);
        let notes = self.deck.get(i)?.notes.as_ref()?.blocks.clone();
        let mut r = self.r(size, pal.dark && self.dark_page, cx);
        r.col.fg = r.col.muted;
        r.n = i * 10_000 + 5_000;
        let blocks = r.blocks(&notes, 0);
        Some(div().flex().flex_col().children(blocks).into_any_element())
    }
}

/// HTML without its tags (and with the common entities).
fn strip_tags(html: &str) -> String {
    let mut out = String::with_capacity(html.len());
    let mut in_tag = false;
    for c in html.chars() {
        match c {
            '<' => in_tag = true,
            '>' if in_tag => in_tag = false,
            c if !in_tag => out.push(c),
            _ => {}
        }
    }
    out.replace("&amp;", "&").replace("&lt;", "<").replace("&gt;", ">").replace("&nbsp;", " ").replace("&quot;", "\"")
}

fn mix_h(a: Hsla, b: Hsla) -> Hsla {
    let a = a.to_rgb();
    let b = b.to_rgb();
    mix(a, b, 0.4).into()
}

/// Remove the bottom margin of the last child, like `> :last-child { margin-bottom: 0 }`.
fn last_flush(children: Vec<AnyElement>) -> gpui::Div {
    div().flex().flex_col().mb(px(-16.0)).children(children)
}

fn capitalise(s: &str) -> String {
    let mut c = s.chars();
    c.next().map(|f| f.to_uppercase().collect::<String>() + c.as_str()).unwrap_or_default()
}

fn diagram_name(lang: &str) -> String {
    match lang {
        "mermaid" => "Mermaid diagram".into(),
        "chart" | "chartjs" | "vega" | "vega-lite" | "plotly" | "echarts" => "Chart".into(),
        l => format!("{} diagram", capitalise(l)),
    }
}

/// GitHub's heading anchors.
pub fn slug(text: &str) -> String {
    text.trim()
        .to_lowercase()
        .chars()
        .filter_map(|c| {
            if c.is_alphanumeric() || c == '-' || c == '_' {
                Some(c)
            } else if c == ' ' {
                Some('-')
            } else {
                None
            }
        })
        .collect()
}

fn resolve(base: Option<&Path>, url: &str) -> Option<PathBuf> {
    let url = url.split(['#', '?']).next().unwrap_or(url);
    let p = Path::new(url);
    let p = if p.is_absolute() { p.to_path_buf() } else { base?.join(p) };
    p.exists().then_some(p)
}

impl Render for Preview {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let pal = Theme::palette(cx);
        let dark = pal.dark && self.dark_page;
        let col = DocColors::new(dark);
        let size = 16.0 * self.zoom;
        let max_w = if self.page_width { 100_000.0 } else { 980.0 * self.zoom.max(1.0) };
        let root = div()
            .id("kd-preview")
            .size_full()
            .overflow_y_scroll()
            .track_scroll(&self.scroll)
            .bg(col.canvas)
            .font_family(body_family(cx))
            .text_size(px(size))
            .line_height(px(size * 1.6))
            .text_color(col.fg);
        if !self.deck.is_empty() {
            // a deck: one frame per slide with its notes
            let avail = f32::from(self.scroll.bounds().size.width);
            let width = if avail > 0.0 { (avail - 64.0).clamp(240.0, 960.0 * self.zoom) } else { 480.0 };
            if (width - self.slide_w).abs() > 0.5 {
                // the pane's width is known after layout: draw again with it
                self.slide_w = width;
                window.request_animation_frame();
            }
            let mut children = vec![div().h(px(24.0)).into_any_element()];
            for i in 0..self.deck.len() {
                let frame = self.slide_frame(i, width, cx);
                let notes = self.slide_notes(i, 14.0, cx);
                children.push(
                    div()
                        .w_full()
                        .flex()
                        .flex_col()
                        .items_center()
                        .pb(px(28.0))
                        .children(frame)
                        .when_some(notes, |d, n| {
                            d.child(
                                div()
                                    .w(px(width))
                                    .mt(px(10.0))
                                    .px(px(12.0))
                                    .py(px(8.0))
                                    .border_l(px(3.0))
                                    .border_color(col.border)
                                    .text_size(px(14.0))
                                    .line_height(px(20.0))
                                    .child(div().font_weight(FontWeight::SEMIBOLD).text_color(col.fg).child("Speaker notes"))
                                    .child(n),
                            )
                        })
                        .into_any_element(),
                );
            }
            return root.children(children).child(div().h(px(200.0)));
        }
        let mut r = self.r(size, dark, cx);
        r.heading = self.theme.filter(|t| !t.deck && !dark && t.id != "github").map(|t| rgb(t.colors[0]).into());
        let front: Option<AnyElement> = (!self.parsed.front_matter.is_empty()).then(|| {
            let rows: Vec<AnyElement> = self
                .parsed
                .front_matter
                .iter()
                .flat_map(|(k, v)| {
                    [
                        div()
                            .px(px(10.0))
                            .py(px(4.0))
                            .border_1()
                            .border_color(col.border)
                            .bg(col.subtle)
                            .font_weight(FontWeight::SEMIBOLD)
                            .child(k.clone())
                            .into_any_element(),
                        div().px(px(10.0)).py(px(4.0)).border_1().border_color(col.border).child(v.clone()).into_any_element(),
                    ]
                })
                .collect();
            div().mb(px(size)).text_size(px(13.0)).grid().grid_cols(2).children(rows).into_any_element()
        });
        // every top-level block is a child of the scroller, so the preview can scroll to one
        let blocks: Vec<AnyElement> = self
            .parsed
            .blocks
            .clone()
            .iter()
            .map(|b| column(r.render_block(b, 0).unwrap_or_else(|| div().into_any_element()), max_w))
            .collect();
        let empty = self.parsed.blocks.is_empty() && front.is_none();
        let watermark = self.parsed.meta("watermark").filter(|w| !w.is_empty()).map(|w| {
            div()
                .absolute()
                .top(px(120.0))
                .right(px(24.0))
                .text_size(px(12.0))
                .font_weight(FontWeight::BOLD)
                .text_color(col.muted.opacity(0.7))
                .child(w.to_string())
        });
        root.relative()
            .child(div().pt(px(32.0)).child(column(front.unwrap_or_else(|| div().into_any_element()), max_w)))
            .children(blocks)
            .when(empty, |d| {
                d.child(column(div().text_color(col.muted).child("Nothing to preview yet — start typing.").into_any_element(), max_w))
            })
            .child(div().h(px(240.0)))
            .children(watermark)
    }
}

/// The page column: centred, at most MdCraft's 980 px wide (or the pane's width).
fn column(el: AnyElement, max_w: f32) -> AnyElement {
    div().w_full().flex().justify_center().child(div().w_full().max_w(px(max_w)).px(px(40.0)).child(el)).into_any_element()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn slugs_like_github() {
        assert_eq!(slug("Hello, World!"), "hello-world");
        assert_eq!(slug("Step 2 — done"), "step-2--done");
    }

    #[test]
    fn diagrams_are_known() {
        assert!(is_diagram("mermaid"));
        assert!(!is_diagram("rust"));
    }
}
