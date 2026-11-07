//! Paragraph formatting in the editor: the Home ribbon's alignment, spacing, direction and
//! shading, the toolbar on the right with Paragraph settings, and Paragraph – advanced settings.

use gpui::{AnyElement, ClickEvent, Context, Entity, FontWeight, Hsla, IntoElement, Window, deferred, div, prelude::*, px, rgb};
use kd_formats::md::{
    self,
    para::{Border, BorderStyle, LineSpacing, ParaAlign, ParaStyle, Special},
};
use kd_ui::{InputEvent, Palette, TextInput, mix, primary_button, tooltip};

use super::{MarkdownView, icon_colors, md_icon};
use crate::ribbon::{ColorTarget, Menu};

/// The number boxes of the paragraph settings.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Field {
    Line,
    Before,
    After,
    Left,
    Right,
    Special,
    BorderWidth,
    Padding,
}

pub const FIELDS: [Field; 8] =
    [Field::Line, Field::Before, Field::After, Field::Left, Field::Right, Field::Special, Field::BorderWidth, Field::Padding];

/// The panels of the toolbar on the right.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RightPanel {
    Paragraph,
    Table,
    Picture,
}

/// Paragraph – advanced settings: its pages.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AdvTab {
    Indents,
    Breaks,
    Borders,
    Padding,
}

pub struct ParaUi {
    pub inputs: Vec<(Field, Entity<TextInput>)>,
    /// The style the boxes show (to refresh them when the cursor moves to another paragraph).
    pub shown: Option<ParaStyle>,
    pub advanced: Option<AdvTab>,
}

impl ParaUi {
    pub fn new(window: &mut Window, cx: &mut Context<MarkdownView>) -> (ParaUi, Vec<gpui::Subscription>) {
        let mut inputs = Vec::new();
        let mut subs = Vec::new();
        for f in FIELDS {
            let input = cx.new(|cx| TextInput::new(cx, ""));
            subs.push(cx.subscribe_in(&input, window, move |this: &mut MarkdownView, _, e: &InputEvent, window, cx| match e {
                InputEvent::Submit => {
                    this.para_field_commit(f, cx);
                    this.focus_editor(window, cx);
                }
                InputEvent::Escape => {
                    this.para_ui.shown = None;
                    this.focus_editor(window, cx);
                    cx.notify();
                }
                InputEvent::Changed => {}
            }));
            inputs.push((f, input));
        }
        (ParaUi { inputs, shown: None, advanced: None }, subs)
    }

    fn input(&self, f: Field) -> Entity<TextInput> {
        self.inputs.iter().find(|(x, _)| *x == f).map(|(_, i)| i.clone()).expect("every field has a box")
    }
}

/// "0.35 cm" → 0.35.
fn number(s: &str) -> Option<f32> {
    let t: String = s.trim().chars().take_while(|c| c.is_ascii_digit() || *c == '.' || *c == ',' || *c == '-').collect();
    t.replace(',', ".").parse().ok()
}

fn fmt(v: f32) -> String {
    let r = (v * 100.0).round() / 100.0;
    if r.fract() == 0.0 { format!("{r:.0}") } else { format!("{r}") }
}

impl MarkdownView {
    fn para_edit(&mut self, cx: &mut Context<Self>, f: impl FnOnce(&mut ParaStyle)) {
        self.edit(cx, |t, s| md::para::apply(t, s, f));
        self.para_ui.shown = None;
        cx.notify();
    }

    /// The formatting of the paragraph at the cursor.
    pub(super) fn para_here(&self, cx: &gpui::App) -> ParaStyle {
        let ed = self.editor.read(cx);
        md::para::style_at(ed.text(), ed.selection().head)
    }

    /// Run a `para.…` command.
    pub(super) fn para_command(&mut self, id: &str, arg: Option<String>, window: &mut Window, cx: &mut Context<Self>) {
        let align = |a: ParaAlign| move |s: &mut ParaStyle| s.align = if s.align == Some(a) { None } else { Some(a) };
        match id {
            "para.alignLeft" => self.para_edit(cx, align(ParaAlign::Left)),
            "para.alignCenter" => self.para_edit(cx, align(ParaAlign::Center)),
            "para.alignRight" => self.para_edit(cx, align(ParaAlign::Right)),
            "para.justify" => self.para_edit(cx, align(ParaAlign::Justify)),
            "para.lineSpacing" => match arg.as_deref().and_then(number) {
                Some(m) => self.para_edit(cx, |s| s.line = Some(LineSpacing::Multiple(m))),
                None => self.open_menu_at(Menu::LineSpacing, window.mouse_position(), window, cx),
            },
            "para.direction" => self.open_menu_at(Menu::Direction, window.mouse_position(), window, cx),
            "para.ltr" => self.para_edit(cx, |s| s.rtl = false),
            "para.rtl" => self.para_edit(cx, |s| s.rtl = true),
            "para.spaceBefore" => self.para_edit(cx, |s| s.before = if s.before.is_some_and(|b| b > 0.0) { None } else { Some(0.42) }),
            "para.spaceAfter" => self.para_edit(cx, |s| s.after = if s.after.is_some_and(|a| a <= 0.0) { None } else { Some(0.0) }),
            "para.shading" => match arg {
                Some(c) => {
                    let c = md::para::css_color(&c);
                    self.para_edit(cx, |s| s.background = c);
                }
                None => self.open_menu_at(Menu::Color(ColorTarget::Shading), window.mouse_position(), window, cx),
            },
            "para.borderColor" => {
                let c = arg.as_deref().and_then(md::para::css_color);
                self.para_edit(cx, |s| match (s.border.as_mut(), c) {
                    (Some(b), Some(c)) => b.color = c,
                    (None, Some(c)) => s.border = Some(Border { width: 1.0, style: BorderStyle::Solid, color: c }),
                    (_, None) => s.border = None,
                });
            }
            "para.settings" => {
                self.right = if self.right == Some(RightPanel::Paragraph) && arg.is_none() { None } else { Some(RightPanel::Paragraph) };
                self.para_ui.shown = None;
            }
            "para.advanced" => {
                self.para_ui.advanced = Some(AdvTab::Indents);
                self.para_ui.shown = None;
            }
            _ => {}
        }
        cx.notify();
    }

    pub(super) fn para_checked(&self, id: &str, cx: &gpui::App) -> Option<bool> {
        let s = || self.para_here(cx);
        Some(match id {
            "para.alignLeft" => s().align == Some(ParaAlign::Left),
            "para.alignCenter" => s().align == Some(ParaAlign::Center),
            "para.alignRight" => s().align == Some(ParaAlign::Right),
            "para.justify" => s().align == Some(ParaAlign::Justify),
            "para.settings" => self.right == Some(RightPanel::Paragraph),
            _ => return None,
        })
    }

    /// A number box was confirmed: put its value in the paragraph.
    fn para_field_commit(&mut self, f: Field, cx: &mut Context<Self>) {
        let v = number(self.para_ui.input(f).read(cx).text());
        self.para_set(f, v, cx);
    }

    fn para_set(&mut self, f: Field, v: Option<f32>, cx: &mut Context<Self>) {
        self.para_edit(cx, move |s| match f {
            Field::Line => {
                s.line = v.filter(|v| *v > 0.0).map(|v| match s.line {
                    Some(LineSpacing::Exactly(_)) => LineSpacing::Exactly(v),
                    _ => LineSpacing::Multiple(v),
                })
            }
            Field::Before => s.before = v.map(|v| v.max(0.0)),
            Field::After => s.after = v.map(|v| v.max(0.0)),
            Field::Left => s.left = v.filter(|v| *v != 0.0),
            Field::Right => s.right = v.filter(|v| *v != 0.0),
            Field::Special => {
                s.special = match (s.special, v.filter(|v| *v > 0.0)) {
                    (Some(Special::Hanging(_)), Some(v)) => Some(Special::Hanging(v)),
                    (_, Some(v)) => Some(Special::FirstLine(v)),
                    (_, None) => None,
                }
            }
            Field::BorderWidth => {
                if let Some(w) = v.filter(|v| *v > 0.0) {
                    match s.border.as_mut() {
                        Some(b) => b.width = w,
                        None => s.border = Some(Border { width: w, style: BorderStyle::Solid, color: 0x000000 }),
                    }
                } else {
                    s.border = None;
                }
            }
            Field::Padding => s.padding = v.filter(|v| *v > 0.0),
        });
    }

    /// The up and down arrows of a number box.
    fn para_step(&mut self, f: Field, up: bool, cx: &mut Context<Self>) {
        let cur = self.para_here(cx);
        let (value, step) = match f {
            Field::Line => match cur.line {
                Some(LineSpacing::Multiple(m)) => (m, 0.05),
                Some(LineSpacing::Exactly(p)) => (p, 1.0),
                None => (1.15, 0.05),
            },
            Field::Before => (cur.before.unwrap_or(0.0), 0.1),
            Field::After => (cur.after.unwrap_or(0.0), 0.1),
            Field::Left => (cur.left.unwrap_or(0.0), 0.25),
            Field::Right => (cur.right.unwrap_or(0.0), 0.25),
            Field::Special => (
                match cur.special {
                    Some(Special::FirstLine(v)) | Some(Special::Hanging(v)) => v,
                    None => 0.0,
                },
                0.25,
            ),
            Field::BorderWidth => (cur.border.map(|b| b.width).unwrap_or(0.0), 0.5),
            Field::Padding => (cur.padding.unwrap_or(0.0), 0.05),
        };
        let next = ((value + if up { step } else { -step }) * 100.0).round() / 100.0;
        self.para_set(f, Some(next.max(if f == Field::Left || f == Field::Right { -5.0 } else { 0.0 })), cx);
    }

    /// Show the paragraph's values in the boxes (when the cursor moved to another paragraph).
    fn para_sync(&mut self, window: &Window, cx: &mut Context<Self>) {
        let cur = self.para_here(cx);
        if self.para_ui.shown.as_ref() == Some(&cur) {
            return;
        }
        let special = match cur.special {
            Some(Special::FirstLine(v)) | Some(Special::Hanging(v)) => Some(v),
            None => None,
        };
        let values = [
            (
                Field::Line,
                match cur.line {
                    Some(LineSpacing::Multiple(m)) => Some(fmt(m)),
                    Some(LineSpacing::Exactly(p)) => Some(format!("{} pt", fmt(p))),
                    None => None,
                },
            ),
            (Field::Before, cur.before.map(|v| format!("{} cm", fmt(v)))),
            (Field::After, cur.after.map(|v| format!("{} cm", fmt(v)))),
            (Field::Left, cur.left.map(|v| format!("{} cm", fmt(v)))),
            (Field::Right, cur.right.map(|v| format!("{} cm", fmt(v)))),
            (Field::Special, special.map(|v| format!("{} cm", fmt(v)))),
            (Field::BorderWidth, cur.border.map(|b| format!("{} pt", fmt(b.width)))),
            (Field::Padding, cur.padding.map(|v| format!("{} cm", fmt(v)))),
        ];
        for (f, v) in values {
            let input = self.para_ui.input(f);
            if !input.read(cx).is_focused(window) {
                input.update(cx, |i, cx| {
                    i.set_placeholder(
                        match f {
                            Field::Line => "Single",
                            Field::BorderWidth => "None",
                            _ => "0 cm",
                        },
                        cx,
                    );
                    i.set_text(v.unwrap_or_default(), cx)
                });
            }
        }
        self.para_ui.shown = Some(cur);
    }

    // ---- the toolbar on the right --------------------------------------------------------------

    pub(super) fn right_bar(&mut self, pal: &Palette, cx: &mut Context<Self>) -> AnyElement {
        let (ic, ac) = icon_colors(pal);
        let in_table = self.table_here(cx).is_some();
        let on_image = self.image_here(cx).is_some();
        let button = |id: &'static str,
                      icon: &'static str,
                      tip: &'static str,
                      panel: RightPanel,
                      here: bool,
                      cmd: &'static str,
                      cx: &mut Context<Self>| {
            let on = self.right == Some(panel);
            // dimmed when the cursor is not in such a thing
            let color: Hsla = if on {
                pal.accent.into()
            } else if here {
                ic
            } else {
                ic.opacity(0.45)
            };
            div()
                .id(id)
                .size(px(34.0))
                .flex()
                .items_center()
                .justify_center()
                .rounded(px(7.0))
                .cursor_pointer()
                .when(on, |d| d.bg(pal.accent_soft))
                .when(!on, |d| d.hover(|d| d.bg(pal.surface_hover)))
                .child(md_icon(icon, 20.0, color, ac))
                .tooltip(tooltip(tip))
                .on_click(cx.listener(move |this, _: &ClickEvent, window, cx| this.run(cmd, None, window, cx)))
        };
        div()
            .w(px(super::side::BAR_W))
            .flex_none()
            .h_full()
            .flex()
            .flex_col()
            .items_center()
            .gap(px(6.0))
            .pt(px(8.0))
            .bg(mix(pal.canvas, pal.surface, 0.5))
            .border_l_1()
            .border_color(pal.border)
            .child(button("right-para", "pilcrow", "Paragraph settings", RightPanel::Paragraph, true, "para.settings", cx))
            .child(button("right-table", "table", "Table settings", RightPanel::Table, in_table, "tbl.settings", cx))
            .child(button("right-picture", "picture", "Picture settings", RightPanel::Picture, on_image, "pic.settings", cx))
            .into_any_element()
    }

    /// A number box with up and down arrows.
    fn spin(&self, f: Field, w: f32, pal: &Palette, cx: &mut Context<Self>) -> AnyElement {
        let (ic, ac) = icon_colors(pal);
        let arrow = |up: bool, cx: &mut Context<Self>| {
            div()
                .id((if up { "para-up" } else { "para-down" }, f as usize))
                .h(px(13.0))
                .w(px(16.0))
                .flex()
                .items_center()
                .justify_center()
                .cursor_pointer()
                .hover(|d| d.bg(pal.surface_hover))
                .child(md_icon(if up { "chevron-up" } else { "chevron-down" }, 11.0, ic, ac))
                .on_click(cx.listener(move |this, _: &ClickEvent, _, cx| this.para_step(f, up, cx)))
        };
        div()
            .w(px(w))
            .h(px(30.0))
            .pl(px(8.0))
            .pr(px(2.0))
            .flex()
            .items_center()
            .rounded(px(5.0))
            .border_1()
            .border_color(pal.border_strong)
            .bg(pal.surface)
            .text_size(px(13.0))
            .child(div().flex_1().min_w_0().child(self.para_ui.input(f)))
            .child(div().flex().flex_col().child(arrow(true, cx)).child(arrow(false, cx)))
            .into_any_element()
    }

    /// A choice of a few options (Multiple / Exactly …).
    fn choice(&self, id: &'static str, options: &[(&'static str, bool)], pal: &Palette, cx: &mut Context<Self>) -> AnyElement {
        // in a row of its own, so a column does not stretch it
        let row = div().flex();
        row.child(div().flex().rounded(px(5.0)).border_1().border_color(pal.border_strong).overflow_hidden().children(
            options.iter().enumerate().map(|(i, (label, on))| {
                let label = *label;
                div()
                    .id((id, i))
                    .px(px(9.0))
                    .h(px(28.0))
                    .flex()
                    .items_center()
                    .text_size(px(12.5))
                    .cursor_pointer()
                    .when(i > 0, |d| d.border_l_1().border_color(pal.border_strong))
                    .when(*on, |d| d.bg(pal.accent).text_color(pal.on_accent))
                    .when(!*on, |d| d.bg(pal.surface).hover(|d| d.bg(pal.surface_hover)))
                    .child(label)
                    .on_click(cx.listener(move |this, _: &ClickEvent, _, cx| this.para_choice(id, label, cx)))
            }),
        ))
        .into_any_element()
    }

    fn para_choice(&mut self, id: &str, label: &str, cx: &mut Context<Self>) {
        match (id, label) {
            ("line-rule", "Multiple") => self.para_edit(cx, |s| {
                s.line = Some(LineSpacing::Multiple(match s.line {
                    Some(LineSpacing::Multiple(m)) => m,
                    _ => 1.15,
                }))
            }),
            ("line-rule", "Exactly") => self.para_edit(cx, |s| {
                s.line = Some(LineSpacing::Exactly(match s.line {
                    Some(LineSpacing::Exactly(p)) => p,
                    _ => 14.0,
                }))
            }),
            ("special", "(none)") => self.para_edit(cx, |s| s.special = None),
            ("special", "First line") => self.para_edit(cx, |s| {
                s.special = Some(Special::FirstLine(match s.special {
                    Some(Special::Hanging(v)) | Some(Special::FirstLine(v)) => v,
                    None => 1.25,
                }))
            }),
            ("special", "Hanging") => self.para_edit(cx, |s| {
                s.special = Some(Special::Hanging(match s.special {
                    Some(Special::Hanging(v)) | Some(Special::FirstLine(v)) => v,
                    None => 1.25,
                }))
            }),
            ("border", style) => self.para_edit(cx, |s| {
                let st = match style {
                    "Solid" => Some(BorderStyle::Solid),
                    "Dashed" => Some(BorderStyle::Dashed),
                    "Dotted" => Some(BorderStyle::Dotted),
                    "Double" => Some(BorderStyle::Double),
                    _ => None,
                };
                s.border = st.map(|st| Border { style: st, ..s.border.unwrap_or(Border { width: 1.0, style: st, color: 0 }) });
            }),
            _ => {}
        }
    }

    fn check(&self, id: &'static str, label: &'static str, on: bool, pal: &Palette, cx: &mut Context<Self>) -> AnyElement {
        div()
            .id(id)
            .flex()
            .items_center()
            .gap(px(8.0))
            .cursor_pointer()
            .text_size(px(13.0))
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
                this.para_edit(cx, |s| match id {
                    "brk-before" => s.page_break_before = !s.page_break_before,
                    "brk-next" => s.keep_with_next = !s.keep_with_next,
                    _ => s.keep_together = !s.keep_together,
                })
            }))
            .into_any_element()
    }

    /// A colour button that opens a colour picker.
    fn color_button(&self, id: &'static str, color: Option<u32>, target: ColorTarget, pal: &Palette, cx: &mut Context<Self>) -> AnyElement {
        let (ic, ac) = icon_colors(pal);
        div()
            .id(id)
            .flex()
            .items_center()
            .gap(px(4.0))
            .h(px(30.0))
            .px(px(5.0))
            .rounded(px(5.0))
            .border_1()
            .border_color(pal.border_strong)
            .bg(pal.surface)
            .cursor_pointer()
            .hover(|d| d.bg(pal.surface_hover))
            .child(
                div()
                    .w(px(38.0))
                    .h(px(20.0))
                    .relative()
                    .overflow_hidden()
                    .border_1()
                    .border_color(pal.border_strong)
                    .when_some(color, |d, c| d.bg(rgb(c)))
                    .when(color.is_none(), |d| {
                        d.bg(gpui::white()).child(div().absolute().left(px(-4.0)).top(px(8.0)).w(px(48.0)).h(px(1.5)).bg(rgb(0xE5484D)))
                    }),
            )
            .child(md_icon("chevron-down", 12.0, ic, ac))
            .on_click(cx.listener(move |this, _: &ClickEvent, window, cx| {
                this.open_menu_at(Menu::Color(target), window.mouse_position(), window, cx)
            }))
            .into_any_element()
    }

    fn label(t: &'static str, pal: &Palette) -> gpui::Div {
        div().text_size(px(12.5)).text_color(pal.text_muted).child(t)
    }

    fn section(t: &'static str) -> gpui::Div {
        div().text_size(px(13.5)).font_weight(FontWeight::SEMIBOLD).child(t)
    }

    /// Two boxes side by side with their labels.
    fn pair(&self, a: (&'static str, Field), b: (&'static str, Field), pal: &Palette, cx: &mut Context<Self>) -> AnyElement {
        div()
            .flex()
            .gap(px(12.0))
            .child(div().flex().flex_col().gap(px(4.0)).child(Self::label(a.0, pal)).child(self.spin(a.1, 118.0, pal, cx)))
            .child(div().flex().flex_col().gap(px(4.0)).child(Self::label(b.0, pal)).child(self.spin(b.1, 118.0, pal, cx)))
            .into_any_element()
    }

    /// Line spacing, paragraph spacing, indents: the panel and the dialog's first page.
    fn indents_and_spacing(&mut self, cur: &ParaStyle, pal: &Palette, cx: &mut Context<Self>) -> Vec<AnyElement> {
        let exactly = matches!(cur.line, Some(LineSpacing::Exactly(_)));
        let special = match cur.special {
            None => "(none)",
            Some(Special::FirstLine(_)) => "First line",
            Some(Special::Hanging(_)) => "Hanging",
        };
        vec![
            Self::section("Line spacing").into_any_element(),
            div()
                .flex()
                .items_center()
                .gap(px(12.0))
                .child(self.choice("line-rule", &[("Multiple", !exactly), ("Exactly", exactly)], pal, cx))
                .child(self.spin(Field::Line, 96.0, pal, cx))
                .into_any_element(),
            Self::section("Paragraph spacing").mt(px(6.0)).into_any_element(),
            self.pair(("Before", Field::Before), ("After", Field::After), pal, cx),
            Self::section("Indents").mt(px(6.0)).into_any_element(),
            self.pair(("Left", Field::Left), ("Right", Field::Right), pal, cx),
            div()
                .flex()
                .flex_col()
                .gap(px(4.0))
                .child(Self::label("Special", pal))
                .child(div().flex().items_center().gap(px(10.0)).child(self.choice(
                    "special",
                    &[("(none)", special == "(none)"), ("First line", special == "First line"), ("Hanging", special == "Hanging")],
                    pal,
                    cx,
                )))
                .into_any_element(),
            div().w(px(118.0)).child(self.spin(Field::Special, 118.0, pal, cx)).into_any_element(),
        ]
    }

    /// The Paragraph settings panel.
    pub(super) fn right_panel(&mut self, pal: &Palette, window: &Window, cx: &mut Context<Self>) -> Option<AnyElement> {
        let panel = self.right?;
        if self.focus_mode {
            return None;
        }
        if panel != RightPanel::Paragraph {
            let body = self.object_panel(panel, pal, window, cx);
            let title = if panel == RightPanel::Table { "Table settings" } else { "Picture settings" };
            return Some(self.right_frame(title, body, pal, cx));
        }
        self.para_sync(window, cx);
        let cur = self.para_here(cx);

        let mut body = div().p(px(14.0)).flex().flex_col().gap(px(10.0));
        for el in self.indents_and_spacing(&cur, pal, cx) {
            body = body.child(el);
        }
        body = body
            .child(div().h(px(1.0)).bg(pal.border).my(px(4.0)))
            .child(
                div()
                    .flex()
                    .items_center()
                    .gap(px(10.0))
                    .child(self.color_button("para-bg", cur.background, ColorTarget::Shading, pal, cx))
                    .child(div().text_size(px(13.0)).child("Background color")),
            )
            .child(div().h(px(1.0)).bg(pal.border).my(px(4.0)))
            .child(
                div()
                    .id("para-adv")
                    .flex()
                    .justify_center()
                    .text_size(px(13.0))
                    .text_color(pal.accent)
                    .cursor_pointer()
                    .hover(|d| d.underline())
                    .child("Show advanced settings")
                    .on_click(cx.listener(|this, _: &ClickEvent, window, cx| this.para_command("para.advanced", None, window, cx))),
            )
            .child(
                div()
                    .id("para-reset")
                    .flex()
                    .justify_center()
                    .text_size(px(12.0))
                    .text_color(pal.text_muted)
                    .cursor_pointer()
                    .hover(|d| d.underline())
                    .child("Clear paragraph formatting")
                    .on_click(cx.listener(|this, _: &ClickEvent, _, cx| this.para_edit(cx, |s| *s = ParaStyle::default()))),
            );
        Some(self.right_frame("Paragraph", body.into_any_element(), pal, cx))
    }

    /// A panel of the toolbar on the right: its title, a close button and the body.
    pub(super) fn right_frame(&self, title: &'static str, body: AnyElement, pal: &Palette, cx: &mut Context<Self>) -> AnyElement {
        let (ic, ac) = icon_colors(pal);
        div()
            .w(px(300.0))
            .flex_none()
            .h_full()
            .flex()
            .flex_col()
            .bg(mix(pal.canvas, pal.surface, 0.5))
            .border_l_1()
            .border_color(pal.border)
            .child(
                div()
                    .flex()
                    .items_center()
                    .h(px(46.0))
                    .flex_none()
                    .pl(px(14.0))
                    .pr(px(8.0))
                    .border_b_1()
                    .border_color(pal.border)
                    .child(div().flex_1().text_size(px(15.0)).font_weight(FontWeight::SEMIBOLD).child(title))
                    .child(
                        div()
                            .id("right-close")
                            .size(px(28.0))
                            .flex()
                            .items_center()
                            .justify_center()
                            .rounded(px(5.0))
                            .cursor_pointer()
                            .hover(|d| d.bg(pal.surface_hover))
                            .child(md_icon("x", 18.0, ic, ac))
                            .on_click(cx.listener(|this, _: &ClickEvent, _, cx| {
                                this.right = None;
                                cx.notify();
                            })),
                    ),
            )
            .child(div().id("right-body").flex_1().min_h_0().overflow_y_scroll().child(body))
            .into_any_element()
    }

    /// Paragraph – advanced settings.
    pub(super) fn para_dialog(&mut self, pal: &Palette, window: &Window, cx: &mut Context<Self>) -> Option<AnyElement> {
        let tab = self.para_ui.advanced?;
        self.para_sync(window, cx);
        let cur = self.para_here(cx);
        let tabs = [
            (AdvTab::Indents, "Indents & spacing"),
            (AdvTab::Breaks, "Line & page breaks"),
            (AdvTab::Borders, "Borders & fill"),
            (AdvTab::Padding, "Paddings"),
        ];
        let nav =
            div().w(px(190.0)).flex_none().py(px(10.0)).border_r_1().border_color(pal.border).children(tabs.iter().map(|(t, label)| {
                let t = *t;
                div()
                    .id(("para-tab", t as usize))
                    .px(px(16.0))
                    .py(px(10.0))
                    .text_size(px(14.0))
                    .font_weight(FontWeight::MEDIUM)
                    .cursor_pointer()
                    .when(t == tab, |d| d.bg(pal.surface_hover))
                    .hover(|d| d.bg(pal.surface_hover))
                    .child(*label)
                    .on_click(cx.listener(move |this, _: &ClickEvent, _, cx| {
                        this.para_ui.advanced = Some(t);
                        cx.notify();
                    }))
            }));
        let mut page = div().flex_1().p(px(20.0)).flex().flex_col().gap(px(10.0));
        match tab {
            AdvTab::Indents => {
                for el in self.indents_and_spacing(&cur, pal, cx) {
                    page = page.child(el);
                }
            }
            AdvTab::Breaks => {
                page = page
                    .child(Self::section("Pagination"))
                    .child(self.check("brk-before", "Page break before", cur.page_break_before, pal, cx))
                    .child(self.check("brk-next", "Keep with next", cur.keep_with_next, pal, cx))
                    .child(self.check("brk-together", "Keep lines together", cur.keep_together, pal, cx))
                    .child(Self::label("Used when printing and in the Word, OpenDocument and PDF exports.", pal).mt(px(6.0)));
            }
            AdvTab::Borders => {
                let st = cur.border.map(|b| b.style);
                page = page
                    .child(Self::section("Border"))
                    .child(self.choice(
                        "border",
                        &[
                            ("None", st.is_none()),
                            ("Solid", st == Some(BorderStyle::Solid)),
                            ("Dashed", st == Some(BorderStyle::Dashed)),
                            ("Dotted", st == Some(BorderStyle::Dotted)),
                            ("Double", st == Some(BorderStyle::Double)),
                        ],
                        pal,
                        cx,
                    ))
                    .child(
                        div()
                            .flex()
                            .gap(px(16.0))
                            .child(div().flex().flex_col().gap(px(4.0)).child(Self::label("Width", pal)).child(self.spin(
                                Field::BorderWidth,
                                110.0,
                                pal,
                                cx,
                            )))
                            .child(div().flex().flex_col().gap(px(4.0)).child(Self::label("Color", pal)).child(self.color_button(
                                "para-bd",
                                cur.border.map(|b| b.color),
                                ColorTarget::Border,
                                pal,
                                cx,
                            ))),
                    )
                    .child(Self::section("Fill").mt(px(8.0)))
                    .child(
                        div()
                            .flex()
                            .items_center()
                            .gap(px(10.0))
                            .child(self.color_button("para-fill", cur.background, ColorTarget::Shading, pal, cx))
                            .child(div().text_size(px(13.0)).child("Background color")),
                    );
            }
            AdvTab::Padding => {
                page = page
                    .child(Self::section("Padding"))
                    .child(Self::label("Space between the text and its border or background.", pal))
                    .child(div().w(px(118.0)).child(self.spin(Field::Padding, 118.0, pal, cx)));
            }
        }
        let card = div()
            .occlude()
            .w(px(720.0))
            .rounded(px(10.0))
            .bg(pal.surface)
            .border_1()
            .border_color(pal.border)
            .shadow_lg()
            .flex()
            .flex_col()
            .child(
                div()
                    .relative()
                    .py(px(12.0))
                    .flex()
                    .justify_center()
                    .border_b_1()
                    .border_color(pal.border)
                    .text_size(px(16.0))
                    .font_weight(FontWeight::SEMIBOLD)
                    .child("Paragraph – advanced settings"),
            )
            .child(div().h(px(400.0)).flex().child(nav).child(page))
            .child(div().flex().justify_center().gap(px(10.0)).py(px(14.0)).border_t_1().border_color(pal.border).child(
                primary_button("para-ok", "OK", pal).w(px(110.0)).on_click(cx.listener(|this, _: &ClickEvent, window, cx| {
                    // a value typed but not confirmed with Enter counts too
                    for f in FIELDS {
                        if this.para_ui.input(f).read(cx).is_focused(window) {
                            this.para_field_commit(f, cx);
                        }
                    }
                    this.para_ui.advanced = None;
                    this.focus_editor(window, cx);
                    cx.notify();
                })),
            ));
        Some(
            deferred(div().absolute().inset_0().flex().items_center().justify_center().bg(Hsla::from(pal.text).opacity(0.12)).child(
                card.on_mouse_down_out(cx.listener(|this, _, _, cx| {
                    if this.more_colors.is_none() && this.menu.is_none() {
                        this.para_ui.advanced = None;
                        cx.notify();
                    }
                })),
            ))
            // under the menus (colour pickers) it opens
            .with_priority(2)
            .into_any_element(),
        )
    }
}
