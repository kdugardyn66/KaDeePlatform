//! The toolbar on the left of the editor and its panels: Find and replace, Comments,
//! Navigation (headings and slides) and Help & Feedback.

use gpui::{AnyElement, ClickEvent, Context, FontWeight, IntoElement, Window, div, prelude::*, px};
use kd_formats::md;
use kd_ui::{Palette, button, mix, primary_button, tooltip};

use super::{MarkdownView, icon_colors, md_icon};

/// The panel open beside the editor.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Side {
    Find,
    Comments,
    Nav,
    Help,
}

/// The width of the toolbar and of a panel.
pub const BAR_W: f32 = 44.0;
pub const PANEL_W: f32 = 290.0;

impl MarkdownView {
    /// Open a panel (or close it when it is open and `toggle`).
    pub(super) fn show_side(&mut self, side: Side, toggle: bool, window: &mut Window, cx: &mut Context<Self>) {
        if toggle && self.side == Some(side) {
            return self.close_side(window, cx);
        }
        if self.side == Some(Side::Find) && side != Side::Find {
            self.clear_find_marks(cx);
        }
        self.side = Some(side);
        match side {
            Side::Find => self.open_find(false, window, cx),
            Side::Nav => self.nav_filter.read(cx).focus(window),
            _ => {}
        }
        cx.notify();
    }

    pub(super) fn close_side(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if self.side == Some(Side::Find) {
            return self.close_find(window, cx);
        }
        self.side = None;
        self.adding_comment = false;
        self.focus_editor(window, cx);
        cx.notify();
    }

    /// The width the toolbar and the panel take.
    pub(super) fn side_width(&self) -> f32 {
        if self.focus_mode { 0.0 } else { BAR_W + if self.side.is_some() { PANEL_W } else { 0.0 } }
    }

    /// The width the toolbar and panel on the right take.
    pub(super) fn right_width(&self) -> f32 {
        if self.focus_mode { 0.0 } else { BAR_W + if self.right.is_some() { 300.0 } else { 0.0 } }
    }

    /// The toolbar: one button per panel.
    pub(super) fn side_bar(&mut self, pal: &Palette, cx: &mut Context<Self>) -> AnyElement {
        let (ic, ac) = icon_colors(pal);
        let comments = md::comments::comments(self.editor.read(cx).text()).len();
        let item = |id: &'static str, icon: &'static str, tip: &'static str, side: Side, badge: usize, cx: &mut Context<Self>| {
            let on = self.side == Some(side);
            div()
                .id(id)
                .relative()
                .size(px(34.0))
                .flex()
                .items_center()
                .justify_center()
                .rounded(px(7.0))
                .cursor_pointer()
                .when(on, |d| d.bg(pal.accent_soft))
                .when(!on, |d| d.hover(|d| d.bg(pal.surface_hover)))
                .child(md_icon(icon, 20.0, if on { pal.accent.into() } else { ic }, ac))
                .when(badge > 0, |d| {
                    d.child(
                        div()
                            .absolute()
                            .top(px(1.0))
                            .right(px(1.0))
                            .min_w(px(14.0))
                            .h(px(14.0))
                            .px(px(3.0))
                            .rounded_full()
                            .bg(pal.accent)
                            .text_color(pal.on_accent)
                            .text_size(px(9.0))
                            .flex()
                            .items_center()
                            .justify_center()
                            .child(badge.to_string()),
                    )
                })
                .tooltip(tooltip(tip))
                .on_click(cx.listener(move |this, _: &ClickEvent, window, cx| this.show_side(side, true, window, cx)))
        };
        div()
            .w(px(BAR_W))
            .flex_none()
            .h_full()
            .flex()
            .flex_col()
            .items_center()
            .gap(px(6.0))
            .pt(px(8.0))
            .bg(mix(pal.canvas, pal.surface, 0.5))
            .border_r_1()
            .border_color(pal.border)
            .child(item("side-find", "find", "Find & Replace (Ctrl+F)", Side::Find, 0, cx))
            .child(item("side-comments", "comment", "Comments", Side::Comments, comments, cx))
            .child(item("side-nav", "navigation", "Navigation (Ctrl+\\)", Side::Nav, 0, cx))
            .child(item("side-help", "megaphone", "Help & Feedback", Side::Help, 0, cx))
            .into_any_element()
    }

    /// The open panel.
    pub(super) fn side_panel(&mut self, pal: &Palette, cx: &mut Context<Self>) -> Option<AnyElement> {
        if self.focus_mode {
            return None;
        }
        Some(match self.side? {
            Side::Nav => self.nav_pane(pal, cx).into_any_element(),
            Side::Find => self.find_panel(pal, cx),
            Side::Comments => self.comments_panel(pal, cx),
            Side::Help => self.help_panel(pal, cx),
        })
    }

    /// A panel's frame: its title, buttons on the right and a close button.
    fn panel_frame(
        &mut self,
        title: &'static str,
        extra: Vec<AnyElement>,
        body: AnyElement,
        pal: &Palette,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        let (ic, ac) = icon_colors(pal);
        div()
            .w(px(PANEL_W))
            .flex_none()
            .h_full()
            .flex()
            .flex_col()
            .bg(mix(pal.canvas, pal.surface, 0.5))
            .border_r_1()
            .border_color(pal.border)
            .child(
                div()
                    .flex()
                    .items_center()
                    .gap(px(4.0))
                    .pl(px(14.0))
                    .pr(px(8.0))
                    .h(px(46.0))
                    .flex_none()
                    .border_b_1()
                    .border_color(pal.border)
                    .child(div().flex_1().text_size(px(15.0)).font_weight(FontWeight::SEMIBOLD).child(title))
                    .children(extra)
                    .child(
                        div()
                            .id("side-close")
                            .size(px(28.0))
                            .flex()
                            .items_center()
                            .justify_center()
                            .rounded(px(5.0))
                            .cursor_pointer()
                            .hover(|d| d.bg(pal.surface_hover))
                            .child(md_icon("x", 18.0, ic, ac))
                            .tooltip(tooltip("Close"))
                            .on_click(cx.listener(|this, _: &ClickEvent, window, cx| this.close_side(window, cx))),
                    ),
            )
            .child(div().id("side-body").flex_1().min_h_0().overflow_y_scroll().child(body))
            .into_any_element()
    }

    fn field(&self, input: gpui::Entity<kd_ui::TextInput>, pal: &Palette) -> gpui::Div {
        div()
            .w_full()
            .h(px(32.0))
            .px(px(9.0))
            .flex()
            .items_center()
            .rounded(px(5.0))
            .border_1()
            .border_color(pal.border_strong)
            .bg(pal.surface)
            .text_size(px(13.5))
            .child(input)
    }

    // ---- Find and replace --------------------------------------------------------------------------

    fn find_panel(&mut self, pal: &Palette, cx: &mut Context<Self>) -> AnyElement {
        let (ic, ac) = icon_colors(pal);
        let (current, total, case, word) = match &self.find {
            Some(f) => (f.current, f.matches.len(), f.opts.match_case, f.opts.whole_word),
            None => (None, 0, false, false),
        };
        let empty = self.find_input.read(cx).text().is_empty();
        let status = match (current, total) {
            (_, 0) if empty => "Type to search".to_string(),
            (_, 0) => "No search results".to_string(),
            (Some(i), n) => format!("{} of {n}", i + 1),
            (None, n) => format!("{n} results"),
        };
        let arrow = |id: &'static str, icon: &'static str, tip: &'static str, cx: &mut Context<Self>| {
            div()
                .id(id)
                .size(px(28.0))
                .flex()
                .items_center()
                .justify_center()
                .rounded(px(5.0))
                .when(total > 0, |d| d.cursor_pointer().hover(|d| d.bg(pal.surface_hover)))
                .when(total == 0, |d| d.opacity(0.4))
                .child(md_icon(icon, 18.0, ic, ac))
                .tooltip(tooltip(tip))
                .on_click(cx.listener(move |this, _: &ClickEvent, _, cx| this.find_step(id == "find-prev", cx)))
        };
        let check = |id: &'static str, label: &'static str, on: bool, cx: &mut Context<Self>| {
            div()
                .id(id)
                .flex()
                .items_center()
                .gap(px(8.0))
                .cursor_pointer()
                .text_size(px(13.5))
                .child(
                    div()
                        .size(px(16.0))
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
                    if let Some(f) = this.find.as_mut() {
                        if id == "find-case" {
                            f.opts.match_case = !f.opts.match_case;
                        } else {
                            f.opts.whole_word = !f.opts.whole_word;
                        }
                    }
                    this.refresh_matches(cx);
                    cx.notify();
                }))
        };
        let can_replace = total > 0;
        let body = div()
            .p(px(14.0))
            .flex()
            .flex_col()
            .gap(px(10.0))
            .child(self.field(self.find_input.clone(), pal))
            .child(self.field(self.replace_input.clone(), pal))
            .child(
                div()
                    .flex()
                    .items_center()
                    .child(div().flex_1().text_size(px(13.0)).text_color(pal.text_muted).child(status))
                    .child(arrow("find-prev", "chevron-up", "Previous (Shift+Enter)", cx))
                    .child(arrow("find-next", "chevron-down", "Next (Enter)", cx)),
            )
            .child(
                div()
                    .flex()
                    .gap(px(6.0))
                    .child(
                        button("find-do-replace", "Replace", pal)
                            .when(!can_replace, |d| d.opacity(0.45))
                            .on_click(cx.listener(|this, _: &ClickEvent, window, cx| this.replace_one(window, cx))),
                    )
                    .child(
                        button("find-do-all", "Replace All", pal)
                            .when(!can_replace, |d| d.opacity(0.45))
                            .on_click(cx.listener(|this, _: &ClickEvent, _, cx| this.replace_everything(cx))),
                    ),
            )
            .child(div().mt(px(4.0)).flex().flex_col().gap(px(10.0)).child(check("find-case", "Case sensitive", case, cx)).child(check(
                "find-word",
                "Whole words only",
                word,
                cx,
            )));
        self.panel_frame("Find and replace", Vec::new(), body.into_any_element(), pal, cx)
    }

    // ---- Comments ----------------------------------------------------------------------------------

    /// Review › New comment: the panel with a box to write it in.
    pub(super) fn start_comment(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        self.side = Some(Side::Comments);
        self.adding_comment = true;
        self.comment_input.update(cx, |i, cx| i.clear(cx));
        self.comment_input.read(cx).focus(window);
        cx.notify();
    }

    /// Put the comment after the selection (or at the cursor).
    pub(super) fn add_comment(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let text = self.comment_input.read(cx).text().trim().to_string();
        if text.is_empty() {
            return;
        }
        let date = chrono::Local::now().format("%Y-%m-%d %H:%M").to_string();
        let marker = md::comments::marker(&self.author, &date, &text);
        self.editor.update(cx, |e, cx| {
            let at = e.selection().end();
            let edit = md::Edit { changes: vec![kd_doc::Change::new(at, at, marker.clone())], selection: kd_doc::Selection::cursor(at) };
            e.apply_edit(edit, cx);
        });
        self.adding_comment = false;
        self.comment_input.update(cx, |i, cx| i.clear(cx));
        self.focus_editor(window, cx);
        cx.notify();
    }

    fn remove_comments(&mut self, which: Option<usize>, cx: &mut Context<Self>) {
        let text = self.editor.read(cx).text().to_string();
        let all = md::comments::comments(&text);
        let gone: Vec<_> = match which {
            Some(i) => all.into_iter().nth(i).into_iter().collect(),
            None => all,
        };
        if gone.is_empty() {
            return;
        }
        let n = gone.len();
        self.editor.update(cx, |e, cx| {
            let at = e.selection().head;
            let changes = gone.iter().rev().map(|c| kd_doc::Change::new(c.range.start, c.range.end, String::new())).collect();
            let removed_before: usize = gone.iter().filter(|c| c.range.end <= at).map(|c| c.range.len()).sum();
            e.apply_edit(md::Edit { changes, selection: kd_doc::Selection::cursor(at.saturating_sub(removed_before)) }, cx);
        });
        if which.is_none() {
            self.toast(format!("Deleted {n} comment{}", if n == 1 { "" } else { "s" }), cx);
        }
        cx.notify();
    }

    fn comments_panel(&mut self, pal: &Palette, cx: &mut Context<Self>) -> AnyElement {
        let (ic, ac) = icon_colors(pal);
        let text = self.editor.read(cx).text().to_string();
        let list = md::comments::comments(&text);
        let head_btn = |id: &'static str, icon: &'static str, tip: &'static str, cx: &mut Context<Self>| {
            div()
                .id(id)
                .size(px(28.0))
                .flex()
                .items_center()
                .justify_center()
                .rounded(px(5.0))
                .cursor_pointer()
                .hover(|d| d.bg(pal.surface_hover))
                .child(md_icon(icon, 18.0, ic, ac))
                .tooltip(tooltip(tip))
                .on_click(cx.listener(move |this, _: &ClickEvent, window, cx| {
                    if id == "comments-add" {
                        this.start_comment(window, cx);
                    } else {
                        this.remove_comments(None, cx);
                    }
                }))
                .into_any_element()
        };
        let mut extra = vec![head_btn("comments-add", "comment-add", "New comment on the selection", cx)];
        if !list.is_empty() {
            extra.push(head_btn("comments-clear", "trash", "Delete all comments", cx));
        }
        let composer = self.adding_comment.then(|| {
            let quote = {
                let ed = self.editor.read(cx);
                let s = ed.selected_text().trim().to_string();
                (!s.is_empty()).then(|| kd_ui::truncate(&s.replace('\n', " "), 80))
            };
            div()
                .p(px(10.0))
                .rounded(px(8.0))
                .border_1()
                .border_color(pal.accent)
                .bg(pal.surface)
                .flex()
                .flex_col()
                .gap(px(8.0))
                .child(div().text_size(px(12.0)).text_color(pal.text_muted).child(if self.author.is_empty() {
                    "Me".to_string()
                } else {
                    self.author.clone()
                }))
                .children(quote.map(|q| {
                    div().pl(px(8.0)).border_l_2().border_color(pal.border_strong).text_size(px(12.0)).text_color(pal.text_muted).child(q)
                }))
                .child(self.field(self.comment_input.clone(), pal))
                .child(
                    div()
                        .flex()
                        .justify_end()
                        .gap(px(6.0))
                        .child(button("comment-cancel", "Cancel", pal).on_click(cx.listener(|this, _: &ClickEvent, window, cx| {
                            this.adding_comment = false;
                            this.focus_editor(window, cx);
                            cx.notify();
                        })))
                        .child(
                            primary_button("comment-add", "Comment", pal)
                                .on_click(cx.listener(|this, _: &ClickEvent, window, cx| this.add_comment(window, cx))),
                        ),
                )
        });
        let cards: Vec<AnyElement> = list
            .iter()
            .enumerate()
            .map(|(i, c)| {
                let range = c.range.clone();
                let line = c.line;
                let initials: String =
                    c.author.split_whitespace().filter_map(|w| w.chars().next()).take(2).collect::<String>().to_uppercase();
                div()
                    .id(("comment", i))
                    .p(px(10.0))
                    .rounded(px(8.0))
                    .border_1()
                    .border_color(pal.border)
                    .bg(pal.surface)
                    .cursor_pointer()
                    .hover(|d| d.border_color(pal.accent))
                    .flex()
                    .flex_col()
                    .gap(px(6.0))
                    .child(
                        div()
                            .flex()
                            .items_center()
                            .gap(px(8.0))
                            .child(
                                div()
                                    .size(px(24.0))
                                    .flex_none()
                                    .rounded_full()
                                    .bg(pal.accent_soft)
                                    .text_color(pal.accent)
                                    .text_size(px(10.5))
                                    .font_weight(FontWeight::SEMIBOLD)
                                    .flex()
                                    .items_center()
                                    .justify_center()
                                    .child(initials),
                            )
                            .child(
                                div()
                                    .flex_1()
                                    .min_w_0()
                                    .child(div().text_size(px(12.5)).font_weight(FontWeight::SEMIBOLD).child(c.author.clone()))
                                    .child(div().text_size(px(11.0)).text_color(pal.text_muted).child(c.date.clone())),
                            )
                            .child(
                                div()
                                    .id(("comment-del", i))
                                    .size(px(24.0))
                                    .flex()
                                    .items_center()
                                    .justify_center()
                                    .rounded(px(4.0))
                                    .hover(|d| d.bg(pal.surface_hover))
                                    .child(md_icon("check", 15.0, ic, ac))
                                    .tooltip(tooltip("Resolve (delete the comment)"))
                                    .on_click(cx.listener(move |this, _: &ClickEvent, _, cx| {
                                        cx.stop_propagation();
                                        this.remove_comments(Some(i), cx);
                                    })),
                            ),
                    )
                    .when(!c.context.is_empty(), |d| {
                        d.child(
                            div()
                                .pl(px(8.0))
                                .border_l_2()
                                .border_color(pal.border_strong)
                                .text_size(px(12.0))
                                .text_color(pal.text_muted)
                                .child(c.context.clone()),
                        )
                    })
                    .child(div().text_size(px(13.0)).line_height(px(18.0)).child(c.text.clone()))
                    .on_click(cx.listener(move |this, _: &ClickEvent, window, cx| {
                        this.editor.update(cx, |ed, cx| {
                            ed.go_to_line(line, window, cx);
                            ed.set_selection(kd_doc::Selection::new(range.start, range.end), cx);
                        });
                        this.preview.read(cx).reveal_line(line, true);
                    }))
                    .into_any_element()
            })
            .collect();
        let empty = cards.is_empty() && !self.adding_comment;
        let body = div()
            .p(px(12.0))
            .flex()
            .flex_col()
            .gap(px(8.0))
            .children(composer)
            .when(empty, |d| {
                d.child(div().pt(px(4.0)).text_size(px(13.5)).text_color(pal.text_muted).child("There are no comments in the document."))
                    .child(div().text_size(px(12.0)).text_color(pal.text_faint).line_height(px(17.0)).child(
                        "Select text and click the new comment button to comment on it. Comments are kept in the file as hidden notes: other Markdown apps and every export leave them out.",
                    ))
            })
            .children(cards);
        self.panel_frame("Comments", extra, body.into_any_element(), pal, cx)
    }

    // ---- Help & Feedback ---------------------------------------------------------------------------

    fn help_panel(&mut self, pal: &Palette, cx: &mut Context<Self>) -> AnyElement {
        let (ic, ac) = icon_colors(pal);
        let row = |id: &'static str, icon: &'static str, title: &'static str, sub: &'static str, cx: &mut Context<Self>| {
            div()
                .id(id)
                .flex()
                .items_start()
                .gap(px(10.0))
                .p(px(10.0))
                .rounded(px(8.0))
                .cursor_pointer()
                .hover(|d| d.bg(pal.surface_hover))
                .child(div().pt(px(1.0)).child(md_icon(icon, 20.0, ic, ac)))
                .child(
                    div()
                        .flex_1()
                        .min_w_0()
                        .child(div().text_size(px(13.5)).font_weight(FontWeight::MEDIUM).child(title))
                        .child(div().text_size(px(12.0)).line_height(px(17.0)).text_color(pal.text_muted).child(sub)),
                )
                .on_click(cx.listener(move |this, _: &ClickEvent, window, cx| this.help_action(id, window, cx)))
        };
        let section = |t: &'static str| {
            div()
                .px(px(10.0))
                .pt(px(10.0))
                .pb(px(2.0))
                .text_size(px(11.5))
                .font_weight(FontWeight::SEMIBOLD)
                .text_color(pal.text_faint)
                .child(t.to_uppercase())
        };
        let body = div()
            .p(px(6.0))
            .flex()
            .flex_col()
            .child(section("Help"))
            .child(row(
                "help-guide",
                "help",
                "Markdown guide",
                "Headings, lists, tables, pictures, diagrams and slides, with examples.",
                cx,
            ))
            .child(row("help-keys", "keyboard", "Keyboard shortcuts", "Every shortcut of the editor.", cx))
            .child(row("help-commands", "search", "Search commands", "Find any command by name (Alt+Q).", cx))
            .child(section("Feedback"))
            .child(row("help-idea", "megaphone", "Suggest an idea", "Tell us what would make Kadee Markdown better.", cx))
            .child(row("help-bug", "info", "Report a problem", "Something does not work as it should.", cx))
            .child(div().px(px(10.0)).pt(px(14.0)).text_size(px(11.5)).text_color(pal.text_faint).line_height(px(16.0)).child(format!(
                "Kadee Markdown · KaDeePlatform {}. Feedback opens a new issue on GitHub with your version and system filled in.",
                env!("CARGO_PKG_VERSION")
            )));
        self.panel_frame("Help & Feedback", Vec::new(), body.into_any_element(), pal, cx)
    }

    fn help_action(&mut self, id: &str, window: &mut Window, cx: &mut Context<Self>) {
        match id {
            "help-guide" => self.run("help.markdown", None, window, cx),
            "help-keys" => self.run("help.shortcuts", None, window, cx),
            "help-commands" => self.run("help.search", None, window, cx),
            "help-idea" | "help-bug" => {
                let (title, label) = if id == "help-idea" { ("Idea: ", "enhancement") } else { ("Problem: ", "bug") };
                let body = format!(
                    "**What happened / what would help**\n\n\n\n---\nKadee Markdown · KaDeePlatform {} · {} {}",
                    env!("CARGO_PKG_VERSION"),
                    std::env::consts::OS,
                    std::env::consts::ARCH
                );
                let url = format!("{}/issues/new?labels={label}&title={}&body={}", kd_store::SOURCE_REPO, encode(title), encode(&body));
                cx.open_url(&url);
            }
            _ => {}
        }
    }
}

/// Percent-encoding for a URL's query.
fn encode(s: &str) -> String {
    let mut out = String::new();
    for b in s.bytes() {
        match b {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => out.push(b as char),
            _ => out.push_str(&format!("%{b:02X}")),
        }
    }
    out
}
