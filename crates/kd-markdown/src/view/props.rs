//! File › Properties: the document's properties in the front matter, as Pandoc names them —
//! identity (title, subtitle, author, date, abstract, keywords), fonts, page layout (paper,
//! orientation, margins, theme, table of contents, numbered sections) and text direction.

use gpui::{AnyElement, ClickEvent, Context, Entity, FontWeight, Hsla, IntoElement, Window, deferred, div, prelude::*, px};
use kd_formats::md::{self, library, page::PageSetup, props};
use kd_ui::{InputEvent, Palette, TextInput, primary_button, tooltip};

use super::{MarkdownView, icon_colors, md_icon};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PropsTab {
    Identity,
    Fonts,
    Layout,
    Language,
}

/// The text boxes: (front matter key, box). Margins use `margin-…` keys of their own.
pub struct PropsUi {
    pub tab: Option<PropsTab>,
    pub inputs: Vec<(&'static str, Entity<TextInput>)>,
}

const TEXT_KEYS: [(&str, &str); 10] = [
    ("title", "The document's title"),
    ("subtitle", "A secondary title"),
    ("author", "Names, separated by commas"),
    ("date", "2026-11-07"),
    ("abstract", "A short summary of the document"),
    ("keywords", "Search terms, separated by commas"),
    ("margin-top", "2.5 cm"),
    ("margin-bottom", "2.5 cm"),
    ("margin-left", "2.5 cm"),
    ("margin-right", "2.5 cm"),
];

impl PropsUi {
    pub fn new(window: &mut Window, cx: &mut Context<MarkdownView>) -> (PropsUi, Vec<gpui::Subscription>) {
        let mut inputs = Vec::new();
        let mut subs = Vec::new();
        for (key, hint) in TEXT_KEYS {
            let input = cx.new(|cx| {
                let i = TextInput::new(cx, hint);
                if key == "abstract" { i.multiline() } else { i }
            });
            subs.push(cx.subscribe_in(&input, window, move |this: &mut MarkdownView, _, e: &InputEvent, window, cx| match e {
                InputEvent::Submit => this.props_commit(Some(key), cx),
                InputEvent::Escape => this.close_props(window, cx),
                InputEvent::Changed => {}
            }));
            inputs.push((key, input));
        }
        (PropsUi { tab: None, inputs }, subs)
    }

    fn input(&self, key: &str) -> Entity<TextInput> {
        self.inputs.iter().find(|(k, _)| *k == key).map(|(_, i)| i.clone()).expect("a box for every key")
    }
}

fn cm(pt: f32) -> String {
    let v = (pt / 72.0 * 2.54 * 100.0).round() / 100.0;
    format!("{v} cm")
}

impl MarkdownView {
    fn front(&self, cx: &gpui::App) -> Vec<(String, String)> {
        md::edit::front_matter(self.editor.read(cx).text())
    }

    fn page_setup(&self, cx: &gpui::App) -> PageSetup {
        let front = self.front(cx);
        PageSetup::from_front(&|k| front.iter().find(|(key, _)| key == k).map(|(_, v)| v.clone()))
    }

    /// Open the properties (on a page: `layout` for Custom margins…).
    pub(super) fn open_props(&mut self, page: Option<&str>, window: &mut Window, cx: &mut Context<Self>) {
        let front = self.front(cx);
        let get = |k: &str| front.iter().find(|(key, _)| key == k).map(|(_, v)| v.clone()).unwrap_or_default();
        let m = self.page_setup(cx).margins;
        for (key, input) in self.props_ui.inputs.clone() {
            let v = match key {
                "author" | "keywords" => props::list(&get(key)).join(", "),
                "abstract" => get(key).trim_matches('"').replace("\\n", "\n"),
                "margin-top" => cm(m.top),
                "margin-bottom" => cm(m.bottom),
                "margin-left" => cm(m.left),
                "margin-right" => cm(m.right),
                k => get(k),
            };
            input.update(cx, |i, cx| i.set_text(v, cx));
        }
        self.props_ui.tab = Some(if page == Some("layout") { PropsTab::Layout } else { PropsTab::Identity });
        if page != Some("layout") {
            self.props_ui.input("title").read(cx).focus(window);
        }
        cx.notify();
    }

    pub(super) fn close_props(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        self.props_commit(None, cx);
        self.props_ui.tab = None;
        self.focus_editor(window, cx);
        cx.notify();
    }

    fn set_keys(&mut self, pairs: Vec<(&'static str, Option<String>)>, cx: &mut Context<Self>) {
        let front = self.front(cx);
        // only what changed, so undo stays meaningful
        let changed: Vec<(&str, Option<String>)> = pairs
            .into_iter()
            .filter(|(k, v)| {
                let cur = front.iter().find(|(key, _)| key == k).map(|(_, v)| v.clone());
                cur.as_deref().filter(|c| !c.is_empty()) != v.as_deref().filter(|c| !c.is_empty())
            })
            .collect();
        if changed.is_empty() {
            return;
        }
        let refs: Vec<(&str, Option<&str>)> = changed.iter().map(|(k, v)| (*k, v.as_deref())).collect();
        self.edit(cx, |t, s| Some(md::edit::set_front_matter(t, s, &refs)));
        cx.notify();
    }

    /// Write the text boxes (one, or all) to the front matter.
    fn props_commit(&mut self, only: Option<&str>, cx: &mut Context<Self>) {
        let mut pairs: Vec<(&'static str, Option<String>)> = Vec::new();
        let mut margins = false;
        for (key, input) in self.props_ui.inputs.clone() {
            if only.is_some_and(|o| o != key) {
                continue;
            }
            let v = input.read(cx).text().trim().to_string();
            match key {
                "author" | "keywords" => {
                    let items: Vec<String> = v.split(',').map(|s| s.trim().to_string()).filter(|s| !s.is_empty()).collect();
                    pairs.push((key, props::list_value(&items)));
                }
                "abstract" => pairs.push((key, Some(v.replace('\n', "\\n")).filter(|v| !v.is_empty()))),
                k if k.starts_with("margin-") => margins = true,
                k => pairs.push((k, Some(v).filter(|v| !v.is_empty()))),
            }
        }
        if margins {
            let mut page = self.page_setup(cx);
            let read =
                |k: &str, this: &Self, cx: &Context<Self>| md::page::points(&this.props_ui.input(k).read(cx).text().replace(' ', ""));
            if let Some(v) = read("margin-top", self, cx) {
                page.margins.top = v;
            }
            if let Some(v) = read("margin-bottom", self, cx) {
                page.margins.bottom = v;
            }
            if let Some(v) = read("margin-left", self, cx) {
                page.margins.left = v;
            }
            if let Some(v) = read("margin-right", self, cx) {
                page.margins.right = v;
            }
            let cur = self.page_setup(cx).margins;
            let same = |a: f32, b: f32| (a - b).abs() < 0.5;
            let m = page.margins;
            if !(same(m.top, cur.top) && same(m.bottom, cur.bottom) && same(m.left, cur.left) && same(m.right, cur.right)) {
                pairs.extend([("geometry", Some(page.to_geometry())), ("paper", None), ("orientation", None), ("margins", None)]);
            }
        }
        self.set_keys(pairs, cx);
    }

    fn set_page(&mut self, f: impl FnOnce(&mut PageSetup), cx: &mut Context<Self>) {
        self.props_commit(None, cx);
        let mut page = self.page_setup(cx);
        f(&mut page);
        self.set_keys(vec![("geometry", Some(page.to_geometry())), ("paper", None), ("orientation", None), ("margins", None)], cx);
        let m = page.margins;
        for (k, v) in [("margin-top", m.top), ("margin-bottom", m.bottom), ("margin-left", m.left), ("margin-right", m.right)] {
            self.props_ui.input(k).update(cx, |i, cx| i.set_text(cm(v), cx));
        }
    }

    fn pills(&self, id: &'static str, options: Vec<(String, String, bool)>, pal: &Palette, cx: &mut Context<Self>) -> AnyElement {
        div()
            .flex()
            .flex_wrap()
            .gap(px(6.0))
            .children(options.into_iter().enumerate().map(|(i, (value, label, on))| {
                div()
                    .id((id, i))
                    .px(px(10.0))
                    .h(px(28.0))
                    .flex()
                    .items_center()
                    .rounded(px(14.0))
                    .border_1()
                    .text_size(px(12.5))
                    .cursor_pointer()
                    .border_color(if on { pal.accent } else { pal.border_strong })
                    .when(on, |d| d.bg(pal.accent).text_color(pal.on_accent))
                    .when(!on, |d| d.bg(pal.surface).hover(|d| d.bg(pal.surface_hover)))
                    .child(label)
                    .on_click(cx.listener(move |this, _: &ClickEvent, _, cx| this.props_pick(id, &value, cx)))
            }))
            .into_any_element()
    }

    fn props_pick(&mut self, id: &str, value: &str, cx: &mut Context<Self>) {
        match id {
            "props-paper" => {
                let v = value.to_string();
                self.set_page(|p| p.paper = v, cx);
            }
            "props-orient" => {
                let land = value == "landscape";
                self.set_page(|p| p.landscape = land, cx);
            }
            "props-margins" => {
                let v: f32 = value.parse().unwrap_or(70.9);
                self.set_page(|p| p.margins = md::page::Margins::all(v), cx);
            }
            "props-theme" => {
                self.props_commit(None, cx);
                self.set_keys(vec![("theme", Some(value.to_string()).filter(|v| !v.is_empty()))], cx);
            }
            "props-depth" => {
                self.props_commit(None, cx);
                self.set_keys(vec![("toc-depth", Some(value.to_string()).filter(|v| v != "3"))], cx);
            }
            "props-dir" => {
                self.props_commit(None, cx);
                self.set_keys(vec![("dir", Some(value.to_string()).filter(|v| v == "rtl"))], cx);
            }
            "props-size" => {
                self.props_commit(None, cx);
                self.set_keys(vec![("fontsize", Some(value.to_string()).filter(|v| !v.is_empty()))], cx);
            }
            _ => {}
        }
    }

    fn props_check(
        &self,
        key: &'static str,
        label: &'static str,
        front: &[(String, String)],
        pal: &Palette,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        let on = front.iter().any(|(k, v)| (k == key || (key == "numbersections" && k == "numbered")) && v == "true");
        div()
            .id(key)
            .flex()
            .items_center()
            .gap(px(8.0))
            .cursor_pointer()
            .text_size(px(13.5))
            .child(
                div()
                    .size(px(16.0))
                    .flex_none()
                    .flex()
                    .items_center()
                    .justify_center()
                    .rounded(px(3.0))
                    .border_1()
                    .border_color(if on { pal.accent } else { pal.border_strong })
                    .when(on, |d| d.bg(pal.accent).child(md_icon("check", 13.0, pal.on_accent.into(), pal.on_accent.into())))
                    .when(!on, |d| d.bg(pal.surface)),
            )
            .child(label)
            .on_click(cx.listener(move |this, _: &ClickEvent, _, cx| {
                this.props_commit(None, cx);
                let mut pairs = vec![(key, if on { None } else { Some("true".to_string()) })];
                if key == "numbersections" {
                    pairs.push(("numbered", None));
                }
                this.set_keys(pairs, cx);
            }))
            .into_any_element()
    }

    fn props_field(&self, label: &'static str, key: &'static str, h: f32, pal: &Palette) -> AnyElement {
        div()
            .flex()
            .flex_col()
            .gap(px(4.0))
            .child(div().text_size(px(12.5)).text_color(pal.text_muted).child(label))
            .child(
                div()
                    .w_full()
                    .h(px(h))
                    .px(px(9.0))
                    .py(px(if h > 40.0 { 6.0 } else { 0.0 }))
                    .flex()
                    .when(h <= 40.0, |d| d.items_center())
                    .rounded(px(5.0))
                    .border_1()
                    .border_color(pal.border_strong)
                    .bg(pal.surface)
                    .text_size(px(13.5))
                    .child(self.props_ui.input(key)),
            )
            .into_any_element()
    }

    fn section_title(t: &'static str) -> AnyElement {
        div().mt(px(4.0)).text_size(px(13.5)).font_weight(FontWeight::SEMIBOLD).child(t).into_any_element()
    }

    /// The properties window.
    pub(super) fn props_dialog(&mut self, pal: &Palette, cx: &mut Context<Self>) -> Option<AnyElement> {
        let tab = self.props_ui.tab?;
        let front = self.front(cx);
        let get = |k: &str| front.iter().find(|(key, _)| key == k).map(|(_, v)| v.clone());
        let (ic, ac) = icon_colors(pal);
        let tabs = [
            (PropsTab::Identity, "Document"),
            (PropsTab::Fonts, "Fonts"),
            (PropsTab::Layout, "Page layout"),
            (PropsTab::Language, "Language"),
        ];
        let nav =
            div().w(px(170.0)).flex_none().py(px(10.0)).border_r_1().border_color(pal.border).children(tabs.iter().map(|(t, label)| {
                let t = *t;
                div()
                    .id(("props-tab", t as usize))
                    .px(px(16.0))
                    .py(px(10.0))
                    .text_size(px(14.0))
                    .font_weight(FontWeight::MEDIUM)
                    .cursor_pointer()
                    .when(t == tab, |d| d.bg(pal.surface_hover))
                    .hover(|d| d.bg(pal.surface_hover))
                    .child(*label)
                    .on_click(cx.listener(move |this, _: &ClickEvent, _, cx| {
                        this.props_commit(None, cx);
                        this.props_ui.tab = Some(t);
                        cx.notify();
                    }))
            }));
        let mut page = div().id("props-page").flex_1().min_w_0().overflow_y_scroll().p(px(20.0)).flex().flex_col().gap(px(12.0));
        match tab {
            PropsTab::Identity => {
                page = page
                    .child(self.props_field("Title", "title", 32.0, pal))
                    .child(self.props_field("Subtitle", "subtitle", 32.0, pal))
                    .child(
                        div().flex().gap(px(12.0)).child(div().flex_1().child(self.props_field("Author", "author", 32.0, pal))).child(
                            div()
                                .w(px(170.0))
                                .flex()
                                .items_end()
                                .gap(px(6.0))
                                .child(div().flex_1().child(self.props_field("Date", "date", 32.0, pal)))
                                .child(
                                    div()
                                        .id("props-today")
                                        .size(px(32.0))
                                        .flex()
                                        .items_center()
                                        .justify_center()
                                        .rounded(px(5.0))
                                        .border_1()
                                        .border_color(pal.border_strong)
                                        .cursor_pointer()
                                        .hover(|d| d.bg(pal.surface_hover))
                                        .child(md_icon("calendar", 16.0, ic, ac))
                                        .tooltip(tooltip("Today"))
                                        .on_click(cx.listener(|this, _: &ClickEvent, _, cx| {
                                            let today = chrono::Local::now().format("%Y-%m-%d").to_string();
                                            this.props_ui.input("date").update(cx, |i, cx| i.set_text(today, cx));
                                            this.props_commit(Some("date"), cx);
                                        })),
                                ),
                        ),
                    )
                    .child(self.props_field("Abstract", "abstract", 96.0, pal))
                    .child(self.props_field("Keywords", "keywords", 32.0, pal))
                    .child(
                        div()
                            .text_size(px(12.0))
                            .text_color(pal.text_faint)
                            .child("Press Enter to keep a value; the window keeps them all when it closes."),
                    );
            }
            PropsTab::Fonts => {
                let font = |label: &'static str,
                            cmd: &'static str,
                            key: &str,
                            fallback: &'static str,
                            this: &mut Self,
                            n: &mut usize,
                            cx: &mut Context<Self>| {
                    let value: &'static str = Box::leak(get(key).unwrap_or_else(|| fallback.to_string()).into_boxed_str());
                    div()
                        .flex()
                        .items_center()
                        .gap(px(12.0))
                        .child(div().w(px(150.0)).flex_none().text_size(px(13.0)).child(label))
                        .child(this.font_box(cmd, value, 260.0, n, pal, cx))
                };
                let mut n = 900;
                let sizes: Vec<(String, String, bool)> = ["", "10pt", "11pt", "12pt", "13pt", "14pt"]
                    .iter()
                    .map(|s| {
                        (
                            s.to_string(),
                            if s.is_empty() { "Theme".to_string() } else { s.to_string() },
                            get("fontsize").unwrap_or_default() == *s,
                        )
                    })
                    .collect();
                page = page
                    .child(Self::section_title("Fonts"))
                    .child(font("Body text", "design.mainfont", "mainfont", "The theme's font", self, &mut n, cx))
                    .child(font("Headings and tables", "design.sansfont", "sansfont", "The body font", self, &mut n, cx))
                    .child(font("Code", "design.monofont", "monofont", "The theme's code font", self, &mut n, cx))
                    .child(font("Formulas", "design.mathfont", "mathfont", "The body font", self, &mut n, cx))
                    .child(Self::section_title("Font size"))
                    .child(self.pills("props-size", sizes, pal, cx))
                    .child(
                        div()
                            .text_size(px(12.0))
                            .text_color(pal.text_faint)
                            .child("Kept as Pandoc's mainfont, sansfont, monofont, mathfont and fontsize."),
                    );
            }
            PropsTab::Layout => {
                let setup = self.page_setup(cx);
                let papers: Vec<(String, String, bool)> =
                    md::page::PAPERS.iter().map(|(id, label, ..)| (id.to_string(), label.to_string(), setup.paper == *id)).collect();
                let m = setup.margins;
                let even = m.top == m.bottom && m.bottom == m.left && m.left == m.right;
                let presets: Vec<(String, String, bool)> =
                    [(70.9f32, "Normal 2.5 cm"), (36.0, "Narrow 1.27 cm"), (56.7, "Moderate 2 cm"), (99.2, "Wide 3.5 cm")]
                        .iter()
                        .map(|(v, l)| (v.to_string(), l.to_string(), even && (m.top - v).abs() < 0.6))
                        .collect();
                let theme = get("theme").unwrap_or_default();
                let themes: Vec<(String, String, bool)> = std::iter::once((String::new(), "None".to_string(), theme.is_empty()))
                    .chain(library::THEMES.iter().filter(|t| !t.deck).map(|t| (t.id.to_string(), t.name.to_string(), theme == t.id)))
                    .collect();
                let depth = props::toc_depth(&front);
                let depths: Vec<(String, String, bool)> = (1..=4).map(|d| (d.to_string(), format!("Levels 1–{d}"), depth == d)).collect();
                page = page
                    .child(Self::section_title("Paper"))
                    .child(self.pills("props-paper", papers, pal, cx))
                    .child(Self::section_title("Orientation"))
                    .child(self.pills(
                        "props-orient",
                        vec![
                            ("portrait".into(), "Portrait".into(), !setup.landscape),
                            ("landscape".into(), "Landscape".into(), setup.landscape),
                        ],
                        pal,
                        cx,
                    ))
                    .child(Self::section_title("Margins"))
                    .child(self.pills("props-margins", presets, pal, cx))
                    .child(
                        div()
                            .flex()
                            .gap(px(10.0))
                            .child(div().flex_1().child(self.props_field("Top", "margin-top", 32.0, pal)))
                            .child(div().flex_1().child(self.props_field("Bottom", "margin-bottom", 32.0, pal)))
                            .child(div().flex_1().child(self.props_field("Left", "margin-left", 32.0, pal)))
                            .child(div().flex_1().child(self.props_field("Right", "margin-right", 32.0, pal))),
                    )
                    .child(Self::section_title("Theme"))
                    .child(self.pills("props-theme", themes, pal, cx))
                    .child(Self::section_title("Structure"))
                    .child(self.props_check("toc", "Table of contents at the top", &front, pal, cx))
                    .child(self.pills("props-depth", depths, pal, cx))
                    .child(self.props_check("numbersections", "Numbered headings (1, 1.1, 1.2 …)", &front, pal, cx))
                    .child(div().text_size(px(12.0)).text_color(pal.text_faint).child(format!("geometry: {}", setup.to_geometry())));
            }
            PropsTab::Language => {
                let rtl = props::rtl(&front);
                page = page
                    .child(Self::section_title("Text direction"))
                    .child(self.pills(
                        "props-dir",
                        vec![("ltr".into(), "Left-to-right".into(), !rtl), ("rtl".into(), "Right-to-left".into(), rtl)],
                        pal,
                        cx,
                    ))
                    .child(
                        div()
                            .text_size(px(12.0))
                            .text_color(pal.text_faint)
                            .child("The reading order of the whole document (dir: ltr or rtl)."),
                    );
            }
        }
        let card = div()
            .occlude()
            .w(px(760.0))
            .rounded(px(10.0))
            .bg(pal.surface)
            .border_1()
            .border_color(pal.border)
            .shadow_lg()
            .flex()
            .flex_col()
            .child(
                div()
                    .py(px(12.0))
                    .flex()
                    .justify_center()
                    .border_b_1()
                    .border_color(pal.border)
                    .text_size(px(16.0))
                    .font_weight(FontWeight::SEMIBOLD)
                    .child("Document properties"),
            )
            .child(div().h(px(500.0)).flex().child(nav).child(page))
            .child(
                div().flex().justify_center().py(px(14.0)).border_t_1().border_color(pal.border).child(
                    primary_button("props-close", "Close", pal)
                        .w(px(110.0))
                        .on_click(cx.listener(|this, _: &ClickEvent, window, cx| this.close_props(window, cx))),
                ),
            );
        Some(
            deferred(div().absolute().inset_0().flex().items_center().justify_center().bg(Hsla::from(pal.text).opacity(0.12)).child(card))
                .with_priority(2)
                .into_any_element(),
        )
    }
}
