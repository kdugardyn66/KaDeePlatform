//! The template chooser (File › New… in Kadee Markdown, and New Markdown on the home page) and
//! small previews of templates: a miniature of the first page of a document, or of the first
//! slide of a deck, in the template's theme.

use std::cell::RefCell;
use std::collections::HashMap;
use std::path::PathBuf;
use std::rc::Rc;

use gpui::{AnyElement, App, FontWeight, Hsla, IntoElement, Window, deferred, div, prelude::*, px, rgb};
use kd_formats::md::{self, Block, Inline, library, templates};
use kd_ui::{Palette, button};

/// One line of a miniature.
#[derive(Clone, Debug)]
enum Mini {
    Title(String),
    Heading(String),
    Sub(String),
    Text(String),
    Bullets(usize),
    Table(usize, usize),
    Figure,
    Rule,
}

/// What a template's miniature shows.
#[derive(Clone, Debug)]
struct Thumb {
    deck: bool,
    /// Background, title and text colours.
    bg: u32,
    title: u32,
    text: u32,
    accent: u32,
    lines: Vec<Mini>,
}

fn luminance(c: u32) -> f32 {
    let (r, g, b) = ((c >> 16) & 255, (c >> 8) & 255, c & 255);
    (0.299 * r as f32 + 0.587 * g as f32 + 0.114 * b as f32) / 255.0
}

fn text_of(inl: &[Inline]) -> String {
    md::inline_text(inl).split_whitespace().collect::<Vec<_>>().join(" ")
}

/// The blocks of `text` as miniature lines (at most `max`).
fn minis(blocks: &[Block], max: usize, out: &mut Vec<Mini>) {
    for b in blocks {
        if out.len() >= max {
            return;
        }
        match b {
            Block::Heading { level: 1, inlines, .. } => out.push(Mini::Title(text_of(inlines))),
            Block::Heading { inlines, .. } => out.push(Mini::Heading(text_of(inlines))),
            Block::Paragraph { inlines, .. } => {
                let t = text_of(inlines);
                if t.is_empty() {
                    if matches!(inlines.as_slice(), [Inline::Image { .. }]) {
                        out.push(Mini::Figure);
                    }
                } else if out.last().is_some_and(|m| matches!(m, Mini::Title(_))) && t.len() < 80 {
                    out.push(Mini::Sub(t));
                } else {
                    out.push(Mini::Text(t));
                }
            }
            Block::List { items, .. } => out.push(Mini::Bullets(items.len().min(4))),
            Block::Table { header, rows, .. } => out.push(Mini::Table(header.len().clamp(1, 5), rows.len().clamp(1, 4))),
            Block::Code { lang, .. } if md::diagram::supported(lang, "") || matches!(lang.as_str(), "mermaid" | "chart" | "csv") => {
                out.push(Mini::Figure)
            }
            Block::Code { .. } | Block::Math { .. } => out.push(Mini::Text(String::new())),
            Block::Quote { blocks, .. } | Block::Callout { blocks, .. } => minis(blocks, max, out),
            Block::Rule { .. } => out.push(Mini::Rule),
            Block::PageBreak { .. } | Block::SlideBreak { .. } => return,
            _ => {}
        }
    }
}

/// A template's miniature, cached by its text.
fn thumb(source: &str, theme_hint: Option<&str>) -> Thumb {
    thread_local! {
        static CACHE: RefCell<HashMap<u64, Thumb>> = RefCell::new(HashMap::new());
    }
    use std::hash::{Hash, Hasher};
    let mut h = std::collections::hash_map::DefaultHasher::new();
    (source, theme_hint).hash(&mut h);
    let key = h.finish();
    if let Some(t) = CACHE.with(|c| c.borrow().get(&key).cloned()) {
        return t;
    }
    let vars = templates::Vars::today("");
    let text = templates::fill(source, &vars);
    let parsed = md::parse(&text);
    let theme_name = parsed.front_matter.iter().find(|(k, _)| k == "theme").map(|(_, v)| v.clone()).or(theme_hint.map(str::to_string));
    let theme = theme_name.as_deref().and_then(library::theme);
    let slides = md::slides::slides(&text);
    let deck = !slides.is_empty();
    let mut lines = Vec::new();
    let t = if deck {
        let (content, _) = slides[0].content_and_notes(&text);
        minis(&md::parse(content).blocks, 4, &mut lines);
        let (bg, a, b) =
            theme.filter(|t| t.deck).map(|t| (t.colors[0], t.colors[1], t.colors[2])).unwrap_or((0xffffff, 0x2f5dd6, 0x1f2328));
        let text_c = if luminance(bg) < 0.5 { 0xf3f4f6 } else { 0x1f2328 };
        let title = if (luminance(a) - luminance(bg)).abs() >= (luminance(b) - luminance(bg)).abs() { a } else { b };
        let title = if (luminance(title) - luminance(bg)).abs() < 0.3 { text_c } else { title };
        Thumb { deck, bg, title, text: text_c, accent: a, lines }
    } else {
        minis(&parsed.blocks, 14, &mut lines);
        let heading = theme.map(|t| t.colors[0]).unwrap_or(0x1f2328);
        let accent = theme.map(|t| t.colors[1]).unwrap_or(0x2f5dd6);
        Thumb { deck, bg: 0xffffff, title: heading, text: 0x57606a, accent, lines }
    };
    CACHE.with(|c| c.borrow_mut().insert(key, t.clone()));
    t
}

/// A miniature of a template `w` pixels wide: an A4 page (documents) or a 16:9 slide (decks).
pub fn template_preview(source: &str, theme: Option<&str>, w: f32, pal: &Palette) -> AnyElement {
    let t = thumb(source, theme);
    let k = w / 120.0; // sizes below are for a 120 px wide miniature
    let text = |s: &str, size: f32, color: u32, bold: bool, lines: usize| {
        div()
            .w_full()
            .flex_none()
            .overflow_hidden()
            .max_h(px(size * k * 1.3 * lines as f32))
            .text_size(px(size * k))
            .line_height(px(size * k * 1.3))
            .text_color(rgb(color))
            .when(bold, |d| d.font_weight(FontWeight::BOLD))
            .child(s.to_string())
    };
    let bar = |frac: f32, color: Hsla| div().w(gpui::relative(frac)).h(px(2.0 * k)).rounded(px(1.0)).bg(color);
    let soft: Hsla = Hsla::from(rgb(t.text)).opacity(0.35);
    let mut body = div().size_full().flex().flex_col().overflow_hidden();
    if t.deck {
        body = body.justify_center().gap(px(3.0 * k)).px(px(10.0 * k));
        for m in &t.lines {
            body = match m {
                Mini::Title(s) => body.child(text(s, 11.0, t.title, true, 2)),
                Mini::Heading(s) => body.child(text(s, 9.0, t.title, true, 2)),
                Mini::Sub(s) | Mini::Text(s) if !s.is_empty() => body.child(text(s, 5.5, t.text, false, 2)),
                Mini::Bullets(n) => {
                    body.child(div().flex().flex_col().gap(px(2.0 * k)).children((0..*n).map(|i| bar(0.75 - i as f32 * 0.1, soft))))
                }
                Mini::Figure | Mini::Table(..) => {
                    body.child(div().w(gpui::relative(0.7)).h(px(18.0 * k)).rounded(px(2.0)).bg(Hsla::from(rgb(t.accent)).opacity(0.35)))
                }
                _ => body,
            };
        }
        // a band in the accent colour along the bottom, as on the slides
        let frame = div()
            .w(px(w))
            .h(px(w * 9.0 / 16.0))
            .relative()
            .rounded(px(4.0))
            .overflow_hidden()
            .border_1()
            .border_color(pal.border)
            .bg(rgb(t.bg))
            .child(body)
            .child(div().absolute().left_0().right_0().bottom_0().h(px(2.5 * k)).bg(rgb(t.accent)));
        return frame.into_any_element();
    }
    // block layout: lines keep their height and the page clips what does not fit
    let mut page = div().size_full().overflow_hidden().px(px(9.0 * k)).py(px(10.0 * k));
    for m in &t.lines {
        let line: gpui::Div = match m {
            Mini::Title(s) => div()
                .child(text(s, 8.0, t.title, true, 2))
                .child(div().w_full().mt(px(1.0 * k)).h(px(0.6 * k)).bg(Hsla::from(rgb(t.title)).opacity(0.25))),
            Mini::Heading(s) => div().mt(px(1.5 * k)).child(text(s, 5.5, t.title, true, 1)),
            Mini::Sub(s) => text(s, 4.2, t.text, false, 2),
            Mini::Text(s) if s.is_empty() => div().w_full().h(px(9.0 * k)).rounded(px(1.0)).bg(rgb(0xf3f4f6)),
            Mini::Text(s) => text(s, 3.6, t.text, false, 3),
            Mini::Bullets(n) => div().flex().flex_col().gap(px(1.6 * k)).children((0..*n).map(|i| {
                div()
                    .flex()
                    .items_center()
                    .gap(px(2.0 * k))
                    .child(div().size(px(1.6 * k)).rounded_full().bg(rgb(t.accent)))
                    .child(bar(0.7 - i as f32 * 0.12, soft))
            })),
            Mini::Table(c, r) => div().w_full().flex().flex_col().border_1().border_color(soft).children((0..=*r).map(|ri| {
                div()
                    .flex()
                    .h(px(4.0 * k))
                    .when(ri == 0, |d| d.bg(Hsla::from(rgb(t.accent)).opacity(0.25)))
                    .children((0..*c).map(|_| div().flex_1().border_r_1().border_color(Hsla::from(rgb(t.text)).opacity(0.15))))
            })),
            Mini::Figure => div()
                .w_full()
                .h(px(16.0 * k))
                .rounded(px(1.5))
                .bg(Hsla::from(rgb(t.accent)).opacity(0.12))
                .flex()
                .items_end()
                .justify_center()
                .gap(px(2.0 * k))
                .pb(px(2.0 * k))
                .children(
                    [0.5f32, 0.8, 0.35, 0.65].map(|f| div().w(px(5.0 * k)).h(px(11.0 * k * f)).bg(Hsla::from(rgb(t.accent)).opacity(0.7))),
                ),
            Mini::Rule => div().w_full().h(px(0.6 * k)).bg(soft),
        };
        page = page.child(line.mb(px(2.5 * k)));
    }
    div()
        .w(px(w))
        .h(px(w * 1.32))
        .rounded(px(3.0))
        .overflow_hidden()
        .border_1()
        .border_color(pal.border)
        .bg(rgb(t.bg))
        .shadow_sm()
        .child(page)
        .into_any_element()
}

/// What the chooser does when a template is picked: (template name, Markdown source).
pub type OnPick = Rc<dyn Fn(String, String, &mut Window, &mut App)>;
/// What the chooser does on Cancel, Escape or a click outside.
pub type OnClose = Rc<dyn Fn(&mut Window, &mut App)>;

/// The template chooser as a window over the screen: built-in documents and slide decks and
/// the templates in `template_dirs`, each with a preview.
pub fn template_chooser(template_dirs: &[PathBuf], pal: &Palette, on_pick: OnPick, on_close: OnClose) -> AnyElement {
    let tile = |id: (&'static str, usize), name: String, sub: String, source: &str, theme: Option<&str>, deck: bool, pick: OnClose| {
        let (hover_bg, hover_border) = (pal.surface_hover, pal.border_strong);
        let preview = template_preview(source, theme, if deck { 136.0 } else { 104.0 }, pal);
        div()
            .id(id)
            .p(px(10.0))
            .flex()
            .flex_col()
            .gap(px(6.0))
            .rounded(px(8.0))
            .border_1()
            .border_color(pal.border)
            .cursor_pointer()
            .hover(move |s| s.bg(hover_bg).border_color(hover_border))
            .child(div().h(px(140.0)).flex().items_center().justify_center().rounded(px(6.0)).bg(pal.canvas).child(preview))
            .child(div().text_size(px(12.5)).font_weight(FontWeight::MEDIUM).child(name))
            .child(div().text_size(px(11.0)).text_color(pal.text_muted).line_height(px(14.0)).child(sub))
            .on_click(move |_, window, cx| pick(window, cx))
            .into_any_element()
    };
    let mut docs: Vec<AnyElement> = Vec::new();
    let mut decks: Vec<AnyElement> = Vec::new();
    for (i, t) in templates::BUILT_IN.iter().enumerate() {
        let deck = t.kind == templates::Kind::Presentation;
        let on_pick = on_pick.clone();
        let (name, source) = (t.name.to_string(), t.source.to_string());
        let el = tile(
            ("tpl-new", i),
            t.name.to_string(),
            t.description.to_string(),
            t.source,
            Some(t.theme),
            deck,
            Rc::new(move |w, cx| on_pick(name.clone(), source.clone(), w, cx)),
        );
        if deck { decks.push(el) } else { docs.push(el) }
    }
    let mine: Vec<AnyElement> = templates::user_templates(template_dirs)
        .into_iter()
        .enumerate()
        .map(|(i, t)| {
            let deck = t.kind == templates::Kind::Presentation;
            let source = std::fs::read_to_string(&t.path).unwrap_or_default();
            let on_pick = on_pick.clone();
            let (name, src) = (t.name.clone(), source.clone());
            tile(
                ("tpl-mine", i),
                t.name.clone(),
                format!("{} (.{})", t.kind.label(), t.kind.extension()),
                &source,
                None,
                deck,
                Rc::new(move |w, cx| on_pick(name.clone(), src.clone(), w, cx)),
            )
        })
        .collect();
    let section = |title: &str| {
        div()
            .mt(px(14.0))
            .mb(px(8.0))
            .text_size(px(13.0))
            .font_weight(FontWeight::SEMIBOLD)
            .text_color(pal.text_muted)
            .child(title.to_string())
    };
    let cancel = on_close.clone();
    let card = div()
        .w(px(900.0))
        .max_h(px(680.0))
        .flex()
        .flex_col()
        .rounded(px(12.0))
        .bg(pal.surface)
        .border_1()
        .border_color(pal.border)
        .shadow_lg()
        .occlude()
        .child(
            div()
                .flex()
                .items_center()
                .px(px(18.0))
                .py(px(12.0))
                .border_b_1()
                .border_color(pal.border)
                .child(div().text_size(px(16.0)).font_weight(FontWeight::SEMIBOLD).child("New"))
                .child(div().ml(px(10.0)).text_size(px(12.0)).text_color(pal.text_muted).child("Start from a Markdown template"))
                .child(div().flex_1())
                .child(button("chooser-cancel", "Cancel", pal).on_click(move |_, window, cx| cancel(window, cx))),
        )
        .child(
            div()
                .id("chooser-body")
                .flex_1()
                .min_h_0()
                .overflow_y_scroll()
                .px(px(18.0))
                .pb(px(18.0))
                .child(section("Documents"))
                .child(div().grid().grid_cols(5).gap(px(10.0)).children(docs))
                .child(section("Slide decks"))
                .child(div().grid().grid_cols(5).gap(px(10.0)).children(decks))
                .when(!mine.is_empty(), |d| d.child(section("My templates")).child(div().grid().grid_cols(5).gap(px(10.0)).children(mine))),
        );
    let outside = on_close;
    deferred(
        div()
            .absolute()
            .inset_0()
            .flex()
            .items_center()
            .justify_center()
            .bg(gpui::Hsla::from(pal.text).opacity(0.18))
            .child(card.on_mouse_down_out(move |_, window, cx| outside(window, cx))),
    )
    .with_priority(4)
    .into_any_element()
}
