//! The Markdown view's panels and helpers: the Find bar, the small questions (Go to line, Link,
//! Watermark …), information cards (accessibility, shortcuts), the font menus, file pickers,
//! export and printing, and the slide show.

use std::ops::Range;
use std::path::{Path, PathBuf};

use gpui::{
    AnyElement, ClickEvent, Context, FontWeight, IntoElement, KeyBinding, MouseButton, PathPromptOptions, Pixels, Point, SharedString,
    Window, actions, anchored, deferred, div, point, prelude::*, px,
};
use kd_formats::md::{self, find::FindOptions};
use kd_ui::{Palette, button, primary_button, tooltip};

use super::{MarkdownView, OpenMenu, icon_colors, md_icon};
use crate::commands;
use crate::ribbon::{self, FontTarget, Menu};

actions!(kd_present, [NextSlide, PrevSlide, FirstSlide, LastSlide, EndShow]);

const PRESENT: &str = "KdPresent";

/// The slide show's keys.
pub fn bind_present_keys(cx: &mut gpui::App) {
    let c = Some(PRESENT);
    cx.bind_keys([
        KeyBinding::new("right", NextSlide, c),
        KeyBinding::new("down", NextSlide, c),
        KeyBinding::new("space", NextSlide, c),
        KeyBinding::new("enter", NextSlide, c),
        KeyBinding::new("pagedown", NextSlide, c),
        KeyBinding::new("n", NextSlide, c),
        KeyBinding::new("left", PrevSlide, c),
        KeyBinding::new("up", PrevSlide, c),
        KeyBinding::new("backspace", PrevSlide, c),
        KeyBinding::new("pageup", PrevSlide, c),
        KeyBinding::new("p", PrevSlide, c),
        KeyBinding::new("home", FirstSlide, c),
        KeyBinding::new("end", LastSlide, c),
        KeyBinding::new("escape", EndShow, c),
    ]);
}

pub struct FindState {
    pub opts: FindOptions,
    pub matches: Vec<Range<usize>>,
    pub current: Option<usize>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PromptKind {
    GoToLine,
    Link,
    Watermark,
    OnlineVideo,
    TableSize,
    PictureUrl,
}

impl PromptKind {
    fn title(self) -> &'static str {
        match self {
            PromptKind::GoToLine => "Go to line",
            PromptKind::Link => "Link",
            PromptKind::Watermark => "Custom watermark",
            PromptKind::OnlineVideo => "Online video",
            PromptKind::PictureUrl => "Picture from URL",
            PromptKind::TableSize => "Insert table",
        }
    }

    fn label(self) -> &'static str {
        match self {
            PromptKind::GoToLine => "Line number",
            PromptKind::Link => "Address: https://…, another-file.md or #heading",
            PromptKind::Watermark => "Text shown across every page",
            PromptKind::OnlineVideo => "YouTube or Vimeo link",
            PromptKind::PictureUrl => "The picture's address (https://…)",
            PromptKind::TableSize => "Rows × columns, for example 4 x 3",
        }
    }

    fn placeholder(self) -> &'static str {
        match self {
            PromptKind::GoToLine => "1",
            PromptKind::Link => "https://",
            PromptKind::Watermark => "DRAFT",
            PromptKind::OnlineVideo => "https://youtu.be/…",
            PromptKind::PictureUrl => "https://",
            PromptKind::TableSize => "4 x 3",
        }
    }
}

pub enum Info {
    Accessibility(Vec<md::review::Issue>),
    Shortcuts,
    /// File › Document info: label and value rows.
    Document(Vec<(&'static str, String)>),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Picked {
    Csv,
    Video,
    Include,
    IncludePreview,
}

/// `path` as the document links it: relative to its folder when inside it, `<…>` with spaces.
pub fn link_to(base: Option<&Path>, path: &Path) -> String {
    let shown = base.and_then(|b| path.strip_prefix(b).ok()).unwrap_or(path).to_string_lossy().replace('\\', "/");
    if shown.contains([' ', '(', ')']) { format!("<{shown}>") } else { shown }
}

/// The installed font families, sorted (once).
fn font_names(cx: &gpui::App) -> &'static [String] {
    static NAMES: std::sync::OnceLock<Vec<String>> = std::sync::OnceLock::new();
    NAMES.get_or_init(|| {
        let mut v: Vec<String> =
            cx.text_system().all_font_names().into_iter().filter(|n| !n.starts_with('.') && !n.trim().is_empty()).collect();
        v.sort_by_key(|n| n.to_lowercase());
        v.dedup();
        v
    })
}

impl MarkdownView {
    pub(super) fn open_menu_at(&mut self, menu: Menu, at: Point<Pixels>, window: &mut Window, cx: &mut Context<Self>) {
        self.menu = Some(OpenMenu { menu, at });
        if matches!(menu, Menu::Fonts(_)) {
            self.font_filter.update(cx, |i, cx| i.clear(cx));
            self.font_filter.read(cx).focus(window);
        }
        cx.notify();
    }

    pub(super) fn base_dir(&self) -> Option<PathBuf> {
        self.path.as_ref().and_then(|p| p.parent().map(Path::to_path_buf))
    }

    // ---- Find and Replace ----------------------------------------------------------------------

    pub(super) fn open_find(&mut self, replace: bool, window: &mut Window, cx: &mut Context<Self>) {
        self.side = Some(super::side::Side::Find);
        let sel = self.editor.read(cx).selected_text().to_string();
        let opts = self.find.as_ref().map(|f| f.opts).unwrap_or_default();
        self.find = Some(FindState { opts, matches: Vec::new(), current: None });
        if !sel.is_empty() && !sel.contains('\n') {
            self.find_input.update(cx, |i, cx| i.set_text(sel, cx));
        }
        self.refresh_matches(cx);
        if replace && !self.find_input.read(cx).text().is_empty() {
            self.replace_input.read(cx).focus(window);
        } else {
            self.find_input.read(cx).focus(window);
        }
        cx.notify();
    }

    pub(super) fn close_find(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        self.clear_find_marks(cx);
        if self.side == Some(super::side::Side::Find) {
            self.side = None;
        }
        self.focus_editor(window, cx);
        cx.notify();
    }

    /// Stop showing the matches in the editor.
    pub(super) fn clear_find_marks(&mut self, cx: &mut Context<Self>) {
        self.find = None;
        self.editor.update(cx, |e, cx| {
            e.marks.clear();
            e.mark_current = None;
            cx.notify();
        });
    }

    pub(super) fn refresh_matches(&mut self, cx: &mut Context<Self>) {
        let q = self.find_input.read(cx).text().to_string();
        let Some(f) = self.find.as_mut() else { return };
        let (matches, head) = {
            let ed = self.editor.read(cx);
            (md::find::find_all(ed.text(), &q, f.opts), ed.selection().start())
        };
        f.current = md::find::next_match(&matches, head, false);
        f.matches = matches.clone();
        let current = f.current;
        self.editor.update(cx, |e, cx| {
            e.marks = matches;
            e.mark_current = current;
            cx.notify();
        });
    }

    pub(super) fn on_find_input(&mut self, e: &kd_ui::InputEvent, window: &mut Window, cx: &mut Context<Self>) {
        match e {
            kd_ui::InputEvent::Changed => {
                self.refresh_matches(cx);
                self.show_current(false, cx);
            }
            kd_ui::InputEvent::Submit => self.find_step(false, cx),
            kd_ui::InputEvent::Escape => self.close_find(window, cx),
        }
        cx.notify();
    }

    /// Select the current match (and scroll to it), keeping the focus in the Find bar.
    fn show_current(&mut self, select: bool, cx: &mut Context<Self>) {
        let Some(f) = &self.find else { return };
        let Some(r) = f.current.and_then(|i| f.matches.get(i)).cloned() else { return };
        let current = f.current;
        self.editor.update(cx, |e, cx| {
            if select {
                e.set_selection(kd_doc::Selection::new(r.start, r.end), cx);
            } else {
                e.set_selection(kd_doc::Selection::cursor(r.start), cx);
            }
            e.mark_current = current;
        });
    }

    pub(super) fn find_step(&mut self, back: bool, cx: &mut Context<Self>) {
        let head = self.editor.read(cx).selection();
        let Some(f) = self.find.as_mut() else { return };
        let from = if back { head.start() } else { head.end().max(head.start() + 1) };
        f.current = md::find::next_match(&f.matches, from, back);
        self.show_current(true, cx);
        cx.notify();
    }

    pub(super) fn replace_one(&mut self, _window: &mut Window, cx: &mut Context<Self>) {
        let with = self.replace_input.read(cx).text().to_string();
        let Some(r) = self.find.as_ref().and_then(|f| f.current.and_then(|i| f.matches.get(i))).cloned() else { return };
        self.editor.update(cx, |e, cx| {
            let at = r.start + with.len();
            let edit =
                md::Edit { changes: vec![kd_doc::Change::new(r.start, r.end, with.clone())], selection: kd_doc::Selection::cursor(at) };
            e.apply_edit(edit, cx);
        });
        self.refresh_matches(cx);
        self.show_current(true, cx);
        cx.notify();
    }

    pub(super) fn replace_everything(&mut self, cx: &mut Context<Self>) {
        let q = self.find_input.read(cx).text().to_string();
        let with = self.replace_input.read(cx).text().to_string();
        let opts = self.find.as_ref().map(|f| f.opts).unwrap_or_default();
        let mut n = 0;
        self.editor.update(cx, |e, cx| {
            if let Some((edit, count)) = md::find::replace_all(e.text(), &q, &with, opts, e.selection()) {
                n = count;
                e.apply_edit(edit, cx);
            }
        });
        self.refresh_matches(cx);
        self.toast(if n == 0 { "Nothing to replace".to_string() } else { format!("Replaced {n} × “{q}”") }, cx);
        cx.notify();
    }

    // ---- questions -------------------------------------------------------------------------------

    pub(super) fn open_prompt(&mut self, kind: PromptKind, window: &mut Window, cx: &mut Context<Self>) {
        let initial = match kind {
            PromptKind::GoToLine => {
                let ed = self.editor.read(cx);
                (ed.doc.line_of(ed.selection().head) + 1).to_string()
            }
            PromptKind::Link => {
                let s = self.editor.read(cx).selected_text().trim().to_string();
                if s.starts_with("http") { s } else { String::new() }
            }
            PromptKind::Watermark => self.fm("watermark", cx).unwrap_or_default(),
            PromptKind::TableSize => "3 x 3".into(),
            PromptKind::OnlineVideo | PromptKind::PictureUrl => String::new(),
        };
        self.prompt_input.update(cx, |i, cx| {
            i.set_placeholder(kind.placeholder(), cx);
            i.set_text(initial, cx);
        });
        self.prompt = Some(kind);
        self.prompt_input.read(cx).focus(window);
        cx.notify();
    }

    pub(super) fn close_prompt(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        self.prompt = None;
        self.focus_editor(window, cx);
        cx.notify();
    }

    pub(super) fn submit_prompt(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let Some(kind) = self.prompt.take() else { return };
        let v = self.prompt_input.read(cx).text().trim().to_string();
        self.focus_editor(window, cx);
        match kind {
            PromptKind::GoToLine => match v.parse::<usize>() {
                Ok(n) if n > 0 => self.editor.update(cx, |e, cx| e.go_to_line(n, window, cx)),
                _ => self.toast("Type a line number", cx),
            },
            PromptKind::Link if !v.is_empty() => {
                let sel = self.editor.read(cx).selected_text().trim().to_string();
                let text = if sel.is_empty() || sel == v { format!("\u{1}{}\u{1}", v.trim_start_matches('#')) } else { sel };
                let url = if v.contains(' ') { format!("<{v}>") } else { v };
                self.editor.update(cx, |e, cx| e.insert(&format!("[{text}]({url})"), cx));
            }
            PromptKind::Watermark => self.set_fm("watermark", (!v.is_empty()).then_some(v.as_str()), cx),
            PromptKind::OnlineVideo if !v.is_empty() => self.block(format!("![Video]({v})"), cx),
            PromptKind::PictureUrl if !v.is_empty() => {
                let name = md::media::file_name(&v);
                let alt = name.rsplit_once('.').map(|(n, _)| n.to_string()).unwrap_or(name);
                self.place_picture(alt, v, cx)
            }
            PromptKind::TableSize => {
                let nums: Vec<usize> = v.split(|c: char| !c.is_ascii_digit()).filter_map(|p| p.parse().ok()).collect();
                match nums.as_slice() {
                    [r, c, ..] if *r > 0 && *c > 0 && *r <= 200 && *c <= 30 => self.block(md::edit::table_markdown(*r, *c), cx),
                    _ => self.toast("Type the rows and columns, for example 4 x 3", cx),
                }
            }
            _ => {}
        }
        cx.notify();
    }

    pub(super) fn prompt_card(&mut self, pal: &Palette, cx: &mut Context<Self>) -> Option<AnyElement> {
        let kind = self.prompt?;
        let card = div()
            .w(px(420.0))
            .p(px(18.0))
            .flex()
            .flex_col()
            .gap(px(10.0))
            .rounded(px(12.0))
            .bg(pal.surface)
            .border_1()
            .border_color(pal.border)
            .shadow_lg()
            .occlude()
            .child(div().text_size(px(16.0)).font_weight(FontWeight::SEMIBOLD).child(kind.title()))
            .child(div().text_size(px(12.5)).text_color(pal.text_muted).child(kind.label()))
            .child(
                div()
                    .h(px(34.0))
                    .px(px(10.0))
                    .flex()
                    .items_center()
                    .rounded(px(6.0))
                    .bg(pal.canvas)
                    .border_1()
                    .border_color(pal.accent)
                    .child(self.prompt_input.clone()),
            )
            .child(
                div()
                    .flex()
                    .justify_end()
                    .gap(px(8.0))
                    .child(
                        button("prompt-cancel", "Cancel", pal)
                            .on_click(cx.listener(|this, _: &ClickEvent, window, cx| this.close_prompt(window, cx))),
                    )
                    .child(
                        primary_button("prompt-ok", "OK", pal)
                            .on_click(cx.listener(|this, _: &ClickEvent, window, cx| this.submit_prompt(window, cx))),
                    ),
            );
        Some(overlay(card, pal, cx.listener(|this, _, window, cx| this.close_prompt(window, cx))))
    }

    pub(super) fn info_card(&mut self, pal: &Palette, cx: &mut Context<Self>) -> Option<AnyElement> {
        let info = self.info.as_ref()?;
        let (title, rows): (&str, Vec<AnyElement>) = match info {
            Info::Accessibility(issues) if issues.is_empty() => (
                "Check accessibility",
                vec![
                    div()
                        .text_color(pal.success)
                        .child("No problems found: headings, pictures, links and tables read well.")
                        .into_any_element(),
                ],
            ),
            Info::Accessibility(issues) => (
                "Check accessibility",
                issues
                    .iter()
                    .enumerate()
                    .map(|(i, is)| {
                        let line = is.line;
                        div()
                            .id(("a11y", i))
                            .flex()
                            .gap(px(10.0))
                            .px(px(8.0))
                            .py(px(5.0))
                            .rounded(px(5.0))
                            .cursor_pointer()
                            .hover(|d| d.bg(pal.surface_hover))
                            .child(div().w(px(56.0)).flex_none().text_color(pal.text_muted).child(format!("Line {line}")))
                            .child(div().flex_1().child(is.message.clone()))
                            .on_click(cx.listener(move |this, _: &ClickEvent, window, cx| {
                                this.info = None;
                                this.editor.update(cx, |e, cx| e.go_to_line(line, window, cx));
                                cx.notify();
                            }))
                            .into_any_element()
                    })
                    .collect(),
            ),
            Info::Document(rows) => (
                "Document info",
                rows.iter()
                    .map(|(label, value)| {
                        // the label sits in the left padding, so long values wrap beside it
                        div()
                            .relative()
                            .w_full()
                            .pl(px(110.0))
                            .py(px(4.0))
                            .line_height(px(19.0))
                            .border_b_1()
                            .border_color(pal.border)
                            .child(div().absolute().left_0().top(px(4.0)).text_color(pal.text_muted).child(*label))
                            .child(div().w_full().child(value.clone()))
                            .into_any_element()
                    })
                    .collect(),
            ),
            Info::Shortcuts => (
                "Keyboard shortcuts",
                commands::COMMANDS
                    .iter()
                    .filter_map(|c| c.key.map(|k| (c, k)))
                    .map(|(c, k)| {
                        div()
                            .flex()
                            .justify_between()
                            .px(px(8.0))
                            .py(px(3.0))
                            .child(c.label.trim_end_matches('…').to_string())
                            .child(div().text_color(pal.text_muted).child(commands::key_label(k)))
                            .into_any_element()
                    })
                    .collect(),
            ),
        };
        let card = div()
            .w(px(520.0))
            .max_h(px(560.0))
            .p(px(18.0))
            .flex()
            .flex_col()
            .gap(px(10.0))
            .rounded(px(12.0))
            .bg(pal.surface)
            .border_1()
            .border_color(pal.border)
            .shadow_lg()
            .occlude()
            .text_size(px(13.0))
            .child(div().text_size(px(16.0)).font_weight(FontWeight::SEMIBOLD).child(title))
            .child(div().id("info-rows").flex().flex_col().max_h(px(440.0)).overflow_y_scroll().children(rows))
            .child(div().flex().justify_end().child(primary_button("info-ok", "Close", pal).on_click(cx.listener(
                |this, _: &ClickEvent, window, cx| {
                    this.info = None;
                    this.focus_editor(window, cx);
                    cx.notify();
                },
            ))));
        Some(overlay(
            card,
            pal,
            cx.listener(|this, _, _, cx| {
                this.info = None;
                cx.notify();
            }),
        ))
    }

    // ---- fonts, emoji, colours --------------------------------------------------------------------

    pub(super) fn pick_first_font(&mut self, filter: String, window: &mut Window, cx: &mut Context<Self>) {
        let Some(OpenMenu { menu: Menu::Fonts(target), .. }) = self.menu else { return };
        let q = filter.trim().to_lowercase();
        let pick = font_names(cx).iter().find(|n| n.to_lowercase().contains(&q)).cloned().unwrap_or(filter.trim().to_string());
        self.run(target.command(false), Some(pick), window, cx);
    }

    /// The panels of the menus that are not lists: fonts, emoji, slide backgrounds.
    pub(super) fn special_menu(&mut self, m: Menu, pal: &Palette, cx: &mut Context<Self>) -> Option<AnyElement> {
        let header = |t: &str| {
            div()
                .px(px(10.0))
                .pt(px(6.0))
                .pb(px(4.0))
                .text_size(px(11.5))
                .font_weight(FontWeight::SEMIBOLD)
                .text_color(pal.text_muted)
                .child(t.to_string())
        };
        if let Menu::Color(target) = m {
            return Some(self.color_menu(target, pal, cx));
        }
        match m {
            Menu::Fonts(target) => {
                let q = self.font_filter.read(cx).text().trim().to_lowercase();
                let cmd = target.command(false);
                let theme_label = match target {
                    FontTarget::DocMono => "The theme's code font",
                    FontTarget::DocSans | FontTarget::DocMath => "The body font",
                    _ => "The theme's font",
                };
                let mut rows: Vec<AnyElement> = Vec::new();
                if q.is_empty() {
                    rows.push(
                        div()
                            .id("font-theme")
                            .px(px(10.0))
                            .py(px(4.0))
                            .rounded(px(5.0))
                            .cursor_pointer()
                            .hover(|d| d.bg(pal.surface_hover))
                            .child(theme_label)
                            .on_click(cx.listener(move |this, _: &ClickEvent, window, cx| this.run(cmd, Some(String::new()), window, cx)))
                            .into_any_element(),
                    );
                }
                for (i, name) in font_names(cx).iter().filter(|n| q.is_empty() || n.to_lowercase().contains(&q)).take(200).enumerate() {
                    let n = name.clone();
                    rows.push(
                        div()
                            .id(("font", i))
                            .px(px(10.0))
                            .py(px(3.0))
                            .rounded(px(5.0))
                            .cursor_pointer()
                            .hover(|d| d.bg(pal.surface_hover))
                            .font_family(SharedString::from(name.clone()))
                            .text_size(px(14.0))
                            .child(name.clone())
                            .on_click(cx.listener(move |this, _: &ClickEvent, window, cx| this.run(cmd, Some(n.clone()), window, cx)))
                            .into_any_element(),
                    );
                }
                let filter = div()
                    .mx(px(8.0))
                    .my(px(6.0))
                    .h(px(28.0))
                    .px(px(8.0))
                    .flex()
                    .items_center()
                    .rounded(px(5.0))
                    .border_1()
                    .border_color(pal.accent)
                    .text_size(px(13.0))
                    .child(self.font_filter.clone());
                Some(
                    div()
                        .w(px(280.0))
                        .flex()
                        .flex_col()
                        .child(header("Fonts"))
                        .child(filter)
                        .child(div().id("font-list").max_h(px(360.0)).overflow_y_scroll().px(px(4.0)).flex().flex_col().children(rows))
                        .into_any_element(),
                )
            }
            Menu::Recent => {
                let (ic, ac) = icon_colors(pal);
                let mut rows: Vec<AnyElement> = Vec::new();
                for (i, p) in self.recent.iter().take(10).enumerate() {
                    let path = p.clone();
                    let name = p.file_name().map(|n| n.to_string_lossy().to_string()).unwrap_or_default();
                    let dir = p.parent().map(|d| d.to_string_lossy().to_string()).unwrap_or_default();
                    rows.push(
                        div()
                            .id(("recent", i))
                            .flex()
                            .items_center()
                            .gap(px(8.0))
                            .mx(px(4.0))
                            .px(px(6.0))
                            .py(px(4.0))
                            .rounded(px(5.0))
                            .cursor_pointer()
                            .hover(|d| d.bg(pal.surface_hover))
                            .child(md_icon("file-md", 18.0, ic, ac))
                            .child(
                                div()
                                    .flex_1()
                                    .min_w_0()
                                    .flex()
                                    .flex_col()
                                    .child(div().text_size(px(13.0)).text_color(pal.text).child(name))
                                    .child(div().text_size(px(11.0)).text_color(pal.text_muted).child(kd_ui::truncate(&dir, 60))),
                            )
                            .on_click(cx.listener(move |this, _: &ClickEvent, _, cx| {
                                this.menu = None;
                                cx.emit(super::MarkdownEvent::OpenPath(path.clone()));
                                cx.notify();
                            }))
                            .into_any_element(),
                    );
                }
                if rows.is_empty() {
                    rows.push(
                        div()
                            .px(px(10.0))
                            .py(px(6.0))
                            .text_size(px(12.5))
                            .text_color(pal.text_muted)
                            .child("No Markdown files opened yet.")
                            .into_any_element(),
                    );
                }
                let empty = self.recent.is_empty();
                rows.push(div().my(px(4.0)).h(px(1.0)).bg(pal.border).into_any_element());
                rows.push(
                    div()
                        .id("recent-clear")
                        .flex()
                        .items_center()
                        .gap(px(8.0))
                        .mx(px(4.0))
                        .px(px(6.0))
                        .py(px(4.0))
                        .rounded(px(5.0))
                        .text_size(px(13.0))
                        .when(empty, |d| d.opacity(0.45))
                        .when(!empty, |d| d.cursor_pointer().hover(|d| d.bg(pal.surface_hover)))
                        .child(md_icon("x", 18.0, ic, ac))
                        .child("Clear Recent List")
                        .on_click(cx.listener(move |this, _: &ClickEvent, window, cx| {
                            if !empty {
                                this.run("file.clearRecent", None, window, cx);
                            }
                        }))
                        .into_any_element(),
                );
                Some(div().w(px(360.0)).flex().flex_col().child(header("Open Recent")).children(rows).into_any_element())
            }
            Menu::Emoji => {
                let cells: Vec<AnyElement> = ribbon::EMOJI
                    .split_whitespace()
                    .enumerate()
                    .map(|(i, e)| {
                        let e = e.to_string();
                        div()
                            .id(("emoji", i))
                            .size(px(32.0))
                            .flex()
                            .items_center()
                            .justify_center()
                            .rounded(px(5.0))
                            .text_size(px(19.0))
                            .cursor_pointer()
                            .hover(|d| d.bg(pal.surface_hover))
                            .child(e.clone())
                            .on_click(
                                cx.listener(move |this, _: &ClickEvent, window, cx| this.run("insert.emoji", Some(e.clone()), window, cx)),
                            )
                            .into_any_element()
                    })
                    .collect();
                Some(
                    div()
                        .w(px(340.0))
                        .child(header("Emoji"))
                        .child(div().px(px(6.0)).pb(px(6.0)).grid().grid_cols(10).children(cells))
                        .into_any_element(),
                )
            }
            Menu::SlideBackground => {
                let cells: Vec<AnyElement> = ribbon::SLIDE_BACKGROUNDS
                    .iter()
                    .enumerate()
                    .map(|(i, c)| {
                        let c = c.to_string();
                        let color = crate::preview::parse_color(&c);
                        div()
                            .id(("slide-bg", i))
                            .size(px(26.0))
                            .rounded(px(4.0))
                            .border_1()
                            .border_color(pal.border_strong)
                            .cursor_pointer()
                            .when_some(color, |d, col| d.bg(col))
                            .when(color.is_none(), |d| d.flex().items_center().justify_center().text_size(px(10.0)).child("—"))
                            .tooltip(tooltip(if c.is_empty() { "The theme's background".to_string() } else { c.clone() }))
                            .on_click(cx.listener(move |this, _: &ClickEvent, window, cx| {
                                this.run("slides.background", Some(c.clone()), window, cx)
                            }))
                            .into_any_element()
                    })
                    .collect();
                Some(
                    div()
                        .w(px(230.0))
                        .child(header("Slide background"))
                        .child(div().px(px(10.0)).pb(px(10.0)).grid().grid_cols(7).gap(px(5.0)).children(cells))
                        .into_any_element(),
                )
            }
            _ => None,
        }
    }

    // ---- files ---------------------------------------------------------------------------------------

    /// Insert › Picture from clipboard: the picture is saved in an `images` folder next to the
    /// document and linked.
    pub(super) fn paste_picture(&mut self, _window: &mut Window, cx: &mut Context<Self>) {
        let image = cx.read_from_clipboard().and_then(|item| {
            item.entries().iter().find_map(|e| match e {
                gpui::ClipboardEntry::Image(i) => Some(i.clone()),
                _ => None,
            })
        });
        let Some(image) = image else {
            self.toast("The clipboard holds no picture: copy one first", cx);
            return;
        };
        let ext = match image.format {
            gpui::ImageFormat::Png => "png",
            gpui::ImageFormat::Jpeg => "jpg",
            gpui::ImageFormat::Webp => "webp",
            gpui::ImageFormat::Gif => "gif",
            gpui::ImageFormat::Svg => "svg",
            gpui::ImageFormat::Bmp => "bmp",
            gpui::ImageFormat::Tiff => "tiff",
        };
        let base = self.base_dir();
        let dir = base.clone().or_else(|| self.documents_dir.clone()).unwrap_or_else(std::env::temp_dir).join("images");
        let path = dir.join(format!("pasted-{}.{ext}", chrono::Local::now().format("%Y%m%d-%H%M%S")));
        if let Err(e) = std::fs::create_dir_all(&dir).and_then(|_| std::fs::write(&path, &image.bytes)) {
            self.toast(format!("Could not save the picture: {e}"), cx);
            return;
        }
        let link = link_to(base.as_deref(), &path);
        self.place_picture("Picture".into(), link, cx);
        if base.is_none() {
            self.toast(format!("The picture is in {} — save the document to keep them together", dir.display()), cx);
        }
    }

    pub(super) fn pick_file(&mut self, title: &str, what: Picked, window: &mut Window, cx: &mut Context<Self>) {
        let rx = cx.prompt_for_paths(PathPromptOptions {
            files: true,
            directories: false,
            multiple: false,
            prompt: Some(title.to_string().into()),
        });
        let base = self.base_dir();
        cx.spawn_in(window, async move |this, cx| {
            let Ok(Ok(Some(paths))) = rx.await else { return };
            let Some(p) = paths.into_iter().next() else { return };
            let _ = this.update(cx, |this, cx| {
                let link = link_to(base.as_deref(), &p);
                let name = p.file_stem().map(|s| s.to_string_lossy().to_string()).unwrap_or_default();
                match what {
                    Picked::Csv => match std::fs::read_to_string(&p) {
                        Ok(t) => this.block(md::review::csv_to_table(&t), cx),
                        Err(e) => this.toast(format!("Could not read the file: {e}"), cx),
                    },
                    Picked::Video => this.block(format!("![\u{1}{name}\u{1}]({link}){{width=640}}"), cx),
                    Picked::Include => this.block(format!("@import \"{}\"", link.trim_start_matches('<').trim_end_matches('>')), cx),
                    Picked::IncludePreview => {
                        this.block(format!("@import \"{}\" {{preview}}", link.trim_start_matches('<').trim_end_matches('>')), cx)
                    }
                }
            });
        })
        .detach();
    }

    /// The document as a web page.
    pub(super) fn html(&self, print: bool, cx: &gpui::App) -> String {
        md::html::to_html(self.editor.read(cx).text(), &md::html::HtmlOptions { dark: false, print })
    }

    pub(super) fn export_html(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let dir = self.base_dir().unwrap_or_else(std::env::temp_dir);
        let name = self
            .path
            .as_ref()
            .and_then(|p| p.file_stem())
            .map(|s| format!("{}.html", s.to_string_lossy()))
            .unwrap_or_else(|| "Document.html".into());
        let html = self.html(false, cx);
        let rx = cx.prompt_for_new_path(&dir, Some(&name));
        cx.spawn_in(window, async move |this, cx| {
            let Ok(Ok(Some(path))) = rx.await else { return };
            let path = if path.extension().is_none() { path.with_extension("html") } else { path };
            let msg = match std::fs::write(&path, html) {
                Ok(()) => format!("Exported “{}”", path.file_name().map(|n| n.to_string_lossy().to_string()).unwrap_or_default()),
                Err(e) => format!("Could not export: {e}"),
            };
            let _ = this.update(cx, |this, cx| this.toast(msg, cx));
        })
        .detach();
    }

    /// Print (and Save as PDF) through the browser: the page opens with the print window.
    pub(super) fn print(&mut self, _window: &mut Window, cx: &mut Context<Self>) {
        let html = self.html(true, cx);
        let stem =
            self.path.as_ref().and_then(|p| p.file_stem()).map(|s| s.to_string_lossy().to_string()).unwrap_or_else(|| "Document".into());
        let file = std::env::temp_dir().join(format!("{stem} (print).html"));
        match std::fs::write(&file, html) {
            Ok(()) => {
                cx.open_url(&format!("file://{}", file.to_string_lossy().replace(' ', "%20")));
                self.toast("Opened in your browser to print — choose “Save as PDF” there for a PDF", cx);
            }
            Err(e) => self.toast(format!("Could not print: {e}"), cx),
        }
    }

    // ---- the slide show ------------------------------------------------------------------------------

    pub(super) fn start_present(&mut self, slide: usize, window: &mut Window, cx: &mut Context<Self>) {
        self.present = Some(slide);
        self.menu = None;
        if !window.is_fullscreen() {
            window.toggle_fullscreen();
            self.present_fullscreen = true;
        }
        window.focus(&self.present_focus);
        cx.notify();
    }

    fn end_present(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if let Some(i) = self.present.take() {
            let line = self.preview.read(cx).deck.get(i).map(|s| s.line);
            if let Some(line) = line {
                self.editor.update(cx, |e, cx| e.go_to_line(line, window, cx));
            }
        }
        if self.present_fullscreen && window.is_fullscreen() {
            window.toggle_fullscreen();
        }
        self.present_fullscreen = false;
        self.focus_editor(window, cx);
        cx.notify();
    }

    fn step_slide(&mut self, to: i64, relative: bool, window: &mut Window, cx: &mut Context<Self>) {
        let n = self.preview.read(cx).deck.len() as i64;
        let Some(cur) = self.present else { return };
        let next = if relative {
            cur as i64 + to
        } else if to < 0 {
            n - 1
        } else {
            to
        };
        if next >= n {
            return self.end_present(window, cx);
        }
        self.present = Some(next.clamp(0, (n - 1).max(0)) as usize);
        cx.notify();
    }

    pub(super) fn present_overlay(&mut self, window: &mut Window, cx: &mut Context<Self>) -> Option<AnyElement> {
        let i = self.present?;
        let size = window.viewport_size();
        let (w, h) = (f32::from(size.width), f32::from(size.height));
        let width = (w - 40.0).min((h - 40.0) * 16.0 / 9.0).max(200.0);
        let n = self.preview.read(cx).deck.len();
        let frame = self.preview.update(cx, |p, cx| p.slide_frame(i, width, cx));
        Some(
            deferred(
                anchored().position(point(px(0.0), px(0.0))).child(
                    div()
                        .id("kd-present")
                        .key_context(PRESENT)
                        .track_focus(&self.present_focus)
                        .w(size.width)
                        .h(size.height)
                        .bg(gpui::black())
                        .flex()
                        .items_center()
                        .justify_center()
                        .occlude()
                        .on_action(cx.listener(|this, _: &NextSlide, window, cx| this.step_slide(1, true, window, cx)))
                        .on_action(cx.listener(|this, _: &PrevSlide, window, cx| this.step_slide(-1, true, window, cx)))
                        .on_action(cx.listener(|this, _: &FirstSlide, window, cx| this.step_slide(0, false, window, cx)))
                        .on_action(cx.listener(|this, _: &LastSlide, window, cx| this.step_slide(-1, false, window, cx)))
                        .on_action(cx.listener(|this, _: &EndShow, window, cx| this.end_present(window, cx)))
                        .on_mouse_down(MouseButton::Left, cx.listener(|this, _, window, cx| this.step_slide(1, true, window, cx)))
                        .on_mouse_down(MouseButton::Right, cx.listener(|this, _, window, cx| this.step_slide(-1, true, window, cx)))
                        .children(frame)
                        .child(
                            div()
                                .absolute()
                                .bottom(px(10.0))
                                .right(px(16.0))
                                .text_size(px(12.0))
                                .text_color(gpui::white().opacity(0.45))
                                .child(format!("{} / {n} · Esc ends the slide show", i + 1)),
                        ),
                ),
            )
            .with_priority(10)
            .into_any_element(),
        )
    }
}

impl MarkdownView {
    /// A bar over the editor when the file is a template (`.mdd` or `.mds`).
    pub(super) fn template_bar(&mut self, pal: &Palette, cx: &mut Context<Self>) -> Option<AnyElement> {
        let kind = md::templates::kind_of(self.path.as_ref()?)?;
        Some(
            div()
                .flex()
                .items_center()
                .gap(px(12.0))
                .px(px(14.0))
                .py(px(6.0))
                .bg(pal.accent_soft)
                .border_b_1()
                .border_color(pal.border)
                .text_size(px(13.0))
                .child(div().flex_1().child(format!(
                    "This is a {} (.{}): saving changes the template.",
                    kind.label().to_lowercase(),
                    kind.extension()
                )))
                .child(
                    primary_button("template-use", "New document from this template", pal)
                        .h(px(28.0))
                        .on_click(cx.listener(|this, _: &ClickEvent, window, cx| this.run("template.use", None, window, cx))),
                )
                .into_any_element(),
        )
    }
}

/// A card in the middle of the view over a dimmed background.
fn overlay(card: gpui::Div, pal: &Palette, close: impl Fn(&gpui::MouseDownEvent, &mut Window, &mut gpui::App) + 'static) -> AnyElement {
    deferred(
        div()
            .absolute()
            .inset_0()
            .flex()
            .items_center()
            .justify_center()
            .bg(gpui::Hsla::from(pal.text).opacity(0.18))
            .child(card.on_mouse_down_out(close)),
    )
    .with_priority(4)
    .into_any_element()
}
