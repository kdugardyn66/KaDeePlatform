//! Table settings and Picture settings: the panels of the toolbar on the right for the table
//! or the picture at the cursor, and their `tbl.…` and `pic.…` commands.

use std::cell::Cell;
use std::rc::Rc;

use gpui::{
    AnyElement, Bounds, ClickEvent, Context, Entity, FontWeight, MouseButton, MouseDownEvent, MouseMoveEvent, Pixels, Window, canvas, div,
    prelude::*, px, rgb,
};
use kd_formats::md::{
    self,
    look::{Borders, ImageRef, ImgStyle, TEMPLATES, TableBorder, TableLook, Wrap},
};
use kd_ui::{InputEvent, Palette, TextInput, tooltip};

use super::para::RightPanel;
use super::{MarkdownView, icon_colors, md_icon};
use crate::ribbon::{ColorTarget, Menu};

/// The number boxes of the two panels.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ObjField {
    TableWidth,
    RowHeight,
    BorderWidth,
    PicWidth,
    Angle,
    Opacity,
}

const FIELDS: [ObjField; 6] =
    [ObjField::TableWidth, ObjField::RowHeight, ObjField::BorderWidth, ObjField::PicWidth, ObjField::Angle, ObjField::Opacity];

pub struct ObjUi {
    inputs: Vec<(ObjField, Entity<TextInput>)>,
    /// What the boxes show (refreshed when the cursor moves to another table or picture).
    shown: Option<String>,
    /// The opacity slider: where it is, and the value while it is dragged.
    slider: Rc<Cell<Bounds<Pixels>>>,
    dragging: Option<f32>,
    /// Replace image: where the picture to replace is.
    pub replacing: Option<usize>,
}

impl ObjUi {
    pub fn new(window: &mut Window, cx: &mut Context<MarkdownView>) -> (ObjUi, Vec<gpui::Subscription>) {
        let mut inputs = Vec::new();
        let mut subs = Vec::new();
        for f in FIELDS {
            let input = cx.new(|cx| TextInput::new(cx, ""));
            subs.push(cx.subscribe_in(&input, window, move |this: &mut MarkdownView, _, e: &InputEvent, window, cx| match e {
                InputEvent::Submit => {
                    this.obj_commit(f, cx);
                    this.focus_editor(window, cx);
                }
                InputEvent::Escape => {
                    this.obj_ui.shown = None;
                    this.focus_editor(window, cx);
                    cx.notify();
                }
                InputEvent::Changed => {}
            }));
            inputs.push((f, input));
        }
        (ObjUi { inputs, shown: None, slider: Rc::default(), dragging: None, replacing: None }, subs)
    }

    fn input(&self, f: ObjField) -> Entity<TextInput> {
        self.inputs.iter().find(|(x, _)| *x == f).map(|(_, i)| i.clone()).expect("every field has a box")
    }
}

/// "5,2 cm" → 5.2.
fn number(s: &str) -> Option<f32> {
    let t: String = s.trim().chars().take_while(|c| c.is_ascii_digit() || *c == '.' || *c == ',' || *c == '-').collect();
    t.replace(',', ".").parse().ok()
}

fn fmt(v: f32) -> String {
    let r = (v * 100.0).round() / 100.0;
    if r.fract() == 0.0 { format!("{r:.0}") } else { format!("{r}") }
}

impl MarkdownView {
    /// The look of the table at the cursor (`None` outside a table).
    pub(super) fn table_here(&self, cx: &gpui::App) -> Option<TableLook> {
        let ed = self.editor.read(cx);
        md::look::table_look_at(ed.text(), ed.selection().head)
    }

    /// The picture at the cursor.
    pub(super) fn image_here(&self, cx: &gpui::App) -> Option<ImageRef> {
        let ed = self.editor.read(cx);
        md::look::image_at(ed.text(), ed.selection().head)
    }

    fn table_edit(&mut self, cx: &mut Context<Self>, f: impl FnOnce(&mut TableLook)) {
        self.edit(cx, |t, s| md::look::set_table_look(t, s, f));
        self.obj_ui.shown = None;
        cx.notify();
    }

    fn picture_edit(&mut self, cx: &mut Context<Self>, f: impl FnOnce(&mut ImgStyle)) {
        self.edit(cx, |t, s| md::look::set_image(t, s, |_, st| f(st)));
        self.obj_ui.shown = None;
        cx.notify();
    }

    /// The width of the text between the margins, in cm.
    fn text_width_cm(&self, cx: &gpui::App) -> f32 {
        let front = md::edit::front_matter(self.editor.read(cx).text());
        let page = md::page::PageSetup::from_front(&|k| front.iter().find(|(key, _)| key == k).map(|(_, v)| v.clone()));
        let (w, _) = page.size();
        ((w - page.margins.left - page.margins.right) / 72.0 * 2.54).max(1.0)
    }

    /// The picture's own size in pixels (local files).
    fn natural_size(&self, url: &str) -> Option<(f32, f32)> {
        let url = url.trim_start_matches('<').trim_end_matches('>');
        if url.contains("://") || url.starts_with("data:") {
            return None;
        }
        let url = url.split(['#', '?']).next().unwrap_or(url).replace("%20", " ");
        let p = std::path::Path::new(&url);
        let p = if p.is_absolute() { p.to_path_buf() } else { self.base_dir()?.join(p) };
        crate::media::picture_size(&p).map(|(w, h)| (w as f32, h as f32))
    }

    /// A new picture was chosen: put it in the text, or in place of the one being replaced.
    pub(super) fn place_picture(&mut self, alt: String, link: String, cx: &mut Context<Self>) {
        match self.obj_ui.replacing.take() {
            Some(at) => {
                self.editor.update(cx, |ed, cx| {
                    let link = link.trim_start_matches('<').trim_end_matches('>').to_string();
                    if let Some(e) = md::look::set_image(ed.text(), kd_doc::Selection::cursor(at), |url, _| *url = link) {
                        ed.apply_edit(e, cx);
                    }
                });
                self.obj_ui.shown = None;
            }
            None => self.block(format!("![\u{1}{alt}\u{1}]({link})"), cx),
        }
        cx.notify();
    }

    /// Run a `tbl.…` or `pic.…` command.
    pub(super) fn object_command(&mut self, id: &str, arg: Option<String>, window: &mut Window, cx: &mut Context<Self>) {
        let at = window.mouse_position();
        match id {
            "tbl.settings" | "pic.settings" => {
                let panel = if id == "tbl.settings" { RightPanel::Table } else { RightPanel::Picture };
                self.right = if self.right == Some(panel) && arg.is_none() { None } else { Some(panel) };
                self.obj_ui.shown = None;
            }
            "tbl.fill" => {
                let c = arg.as_deref().and_then(md::para::css_color);
                self.table_edit(cx, |l| l.fill = c);
            }
            "tbl.borderColor" => {
                let c = arg.as_deref().and_then(md::para::css_color);
                self.table_edit(cx, |l| {
                    let b = l.border();
                    l.border = Some(match c {
                        Some(c) => TableBorder { color: c, which: if b.which == Borders::None { Borders::All } else { b.which }, ..b },
                        None => TableBorder { color: l.tpl().border, ..b },
                    });
                });
            }
            "tbl.rowsCols" => self.open_menu_at(Menu::TableRowsCols, at, window, cx),
            "tbl.op" => {
                if let Some(op) = arg {
                    self.edit(cx, |t, s| md::look::table_op(t, s, &op));
                    self.obj_ui.shown = None;
                }
            }
            "pic.crop" => match arg.as_deref() {
                None => self.open_menu_at(Menu::Crop, at, window, cx),
                Some(r) => {
                    let ratio = r.split_once(':').and_then(|(a, b)| Some((a.trim().parse().ok()?, b.trim().parse().ok()?)));
                    self.picture_edit(cx, |s| s.crop = ratio);
                }
            },
            "pic.replace" => match arg.as_deref() {
                None => self.open_menu_at(Menu::ReplacePicture, at, window, cx),
                Some(how) => match self.image_here(cx) {
                    None => self.toast("Put the cursor on a picture first", cx),
                    Some(img) => {
                        self.obj_ui.replacing = Some(img.range.start);
                        match how {
                            "clipboard" => self.paste_picture(window, cx),
                            "url" => self.open_prompt(super::panels::PromptKind::PictureUrl, window, cx),
                            _ => self.pick_picture(window, cx),
                        }
                    }
                },
            },
            "pic.rotateLeft" => self.picture_edit(cx, |s| s.rotate = (s.rotate - 90.0).rem_euclid(360.0)),
            "pic.rotateRight" => self.picture_edit(cx, |s| s.rotate = (s.rotate + 90.0).rem_euclid(360.0)),
            "pic.flipH" => self.picture_edit(cx, |s| s.flip_h = !s.flip_h),
            "pic.flipV" => self.picture_edit(cx, |s| s.flip_v = !s.flip_v),
            "pic.actual" => self.picture_edit(cx, |s| s.width = None),
            "pic.fit" => {
                let w = self.text_width_cm(cx);
                self.picture_edit(cx, |s| s.width = Some((w * 100.0).round() / 100.0));
            }
            "pic.wrap" => {
                let wrap = match arg.as_deref() {
                    Some("left") => Wrap::Left,
                    Some("right") => Wrap::Right,
                    Some("block") => Wrap::Block,
                    _ => Wrap::Inline,
                };
                self.picture_edit(cx, |s| s.wrap = wrap);
            }
            _ => {}
        }
        cx.notify();
    }

    /// A number box was confirmed.
    fn obj_commit(&mut self, f: ObjField, cx: &mut Context<Self>) {
        let v = number(self.obj_ui.input(f).read(cx).text());
        self.obj_set(f, v, cx);
    }

    fn obj_set(&mut self, f: ObjField, v: Option<f32>, cx: &mut Context<Self>) {
        match f {
            ObjField::TableWidth => self.table_edit(cx, |l| {
                l.width = v.filter(|v| *v > 0.0);
                if l.width.is_some() {
                    l.fit = false;
                }
            }),
            ObjField::RowHeight => self.table_edit(cx, |l| l.row_height = v.filter(|v| *v > 0.0)),
            ObjField::BorderWidth => self.table_edit(cx, |l| {
                let b = l.border();
                l.border = Some(match v.filter(|v| *v > 0.0) {
                    Some(w) => TableBorder { width: w, which: if b.which == Borders::None { Borders::All } else { b.which }, ..b },
                    None => TableBorder { which: Borders::None, ..b },
                });
            }),
            ObjField::PicWidth => self.picture_edit(cx, |s| s.width = v.filter(|v| *v > 0.0)),
            ObjField::Angle => self.picture_edit(cx, |s| s.rotate = v.unwrap_or(0.0).rem_euclid(360.0)),
            ObjField::Opacity => self.picture_edit(cx, |s| s.opacity = v.map(|p| (p / 100.0).clamp(0.0, 1.0)).filter(|o| *o < 0.999)),
        }
    }

    fn obj_step(&mut self, f: ObjField, up: bool, cx: &mut Context<Self>) {
        let sign = if up { 1.0 } else { -1.0 };
        let next = |v: f32, step: f32, min: f32| Some((((v + sign * step) / step).round() * step).max(min));
        match f {
            ObjField::TableWidth => {
                let cur = self.table_here(cx).and_then(|l| l.width).unwrap_or_else(|| self.text_width_cm(cx));
                self.obj_set(f, next(cur, 0.5, 1.0), cx);
            }
            ObjField::RowHeight => {
                let cur = self.table_here(cx).and_then(|l| l.row_height).unwrap_or(0.6);
                self.obj_set(f, next(cur, 0.1, 0.0), cx);
            }
            ObjField::BorderWidth => {
                let cur =
                    self.table_here(cx).map(|l| if l.border().which == Borders::None { 0.0 } else { l.border().width }).unwrap_or(0.75);
                self.obj_set(f, next(cur, 0.25, 0.0), cx);
            }
            ObjField::PicWidth => {
                let img = self.image_here(cx);
                let cur = img
                    .as_ref()
                    .and_then(|i| i.style.width)
                    .unwrap_or_else(|| img.as_ref().and_then(|i| self.natural_size(&i.url)).map(|(w, _)| w / 96.0 * 2.54).unwrap_or(8.0));
                self.obj_set(f, next(cur, 0.5, 0.5), cx);
            }
            ObjField::Angle => {
                let cur = self.image_here(cx).map(|i| i.style.rotate).unwrap_or(0.0);
                self.obj_set(f, Some((cur + sign * 15.0).rem_euclid(360.0)), cx);
            }
            ObjField::Opacity => {
                let cur = self.image_here(cx).and_then(|i| i.style.opacity).unwrap_or(1.0) * 100.0;
                self.obj_set(f, next(cur, 5.0, 0.0).map(|v| v.min(100.0)), cx);
            }
        }
    }

    /// Show the table's or picture's values in the boxes.
    fn obj_sync(&mut self, window: &Window, cx: &mut Context<Self>) {
        let table = self.table_here(cx);
        let image = self.image_here(cx);
        let key = format!("{:?}|{:?}", table.as_ref().map(|t| t.to_attr()), image.as_ref().map(|i| (i.range.start, i.style.to_css())));
        if self.obj_ui.shown.as_deref() == Some(key.as_str()) {
            return;
        }
        let values = [
            (ObjField::TableWidth, table.as_ref().and_then(|t| t.width).map(|v| format!("{} cm", fmt(v))), "Page width"),
            (ObjField::RowHeight, table.as_ref().and_then(|t| t.row_height).map(|v| format!("{} cm", fmt(v))), "Auto"),
            (
                ObjField::BorderWidth,
                table.as_ref().map(|t| t.border()).filter(|b| b.which != Borders::None).map(|b| format!("{} pt", fmt(b.width))),
                "None",
            ),
            (ObjField::PicWidth, image.as_ref().and_then(|i| i.style.width).map(|v| format!("{} cm", fmt(v))), "Actual"),
            (ObjField::Angle, image.as_ref().map(|i| format!("{}°", fmt(i.style.rotate))), "0°"),
            (ObjField::Opacity, image.as_ref().map(|i| format!("{} %", fmt(i.style.opacity.unwrap_or(1.0) * 100.0))), "100 %"),
        ];
        for (f, v, placeholder) in values {
            let input = self.obj_ui.input(f);
            if !input.read(cx).is_focused(window) {
                input.update(cx, |i, cx| {
                    i.set_placeholder(placeholder, cx);
                    i.set_text(v.unwrap_or_default(), cx)
                });
            }
        }
        self.obj_ui.shown = Some(key);
    }

    // ---- pieces --------------------------------------------------------------------------------

    fn obj_spin(&self, f: ObjField, w: f32, pal: &Palette, cx: &mut Context<Self>) -> AnyElement {
        let (ic, ac) = icon_colors(pal);
        let arrow = |up: bool, cx: &mut Context<Self>| {
            div()
                .id((if up { "obj-up" } else { "obj-down" }, f as usize))
                .h(px(13.0))
                .w(px(16.0))
                .flex()
                .items_center()
                .justify_center()
                .cursor_pointer()
                .hover(|d| d.bg(pal.surface_hover))
                .child(md_icon(if up { "chevron-up" } else { "chevron-down" }, 11.0, ic, ac))
                .on_click(cx.listener(move |this, _: &ClickEvent, _, cx| this.obj_step(f, up, cx)))
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
            .child(div().flex_1().min_w_0().child(self.obj_ui.input(f)))
            .child(div().flex().flex_col().child(arrow(true, cx)).child(arrow(false, cx)))
            .into_any_element()
    }

    fn obj_check(
        &self,
        id: &'static str,
        label: &'static str,
        on: bool,
        pal: &Palette,
        cx: &mut Context<Self>,
        f: impl Fn(&mut TableLook) + 'static,
    ) -> AnyElement {
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
            .on_click(cx.listener(move |this, _: &ClickEvent, _, cx| this.table_edit(cx, &f)))
            .into_any_element()
    }

    /// A button with an icon (and a label), shown as on when `on`.
    #[allow(clippy::too_many_arguments)]
    fn obj_button(
        &self,
        id: &'static str,
        icon: &'static str,
        label: Option<&'static str>,
        tip: &'static str,
        on: bool,
        pal: &Palette,
        cx: &mut Context<Self>,
        cmd: &'static str,
        arg: Option<&'static str>,
    ) -> AnyElement {
        let (ic, ac) = icon_colors(pal);
        div()
            .id(id)
            .h(px(30.0))
            .px(px(if label.is_some() { 8.0 } else { 0.0 }))
            .when(label.is_none(), |d| d.w(px(34.0)).justify_center())
            .flex()
            .items_center()
            .gap(px(5.0))
            .rounded(px(5.0))
            .border_1()
            .border_color(if on { pal.accent } else { pal.border_strong })
            .when(on, |d| d.bg(pal.accent_soft))
            .when(!on, |d| d.bg(pal.surface).hover(|d| d.bg(pal.surface_hover)))
            .cursor_pointer()
            .text_size(px(12.5))
            .child(md_icon(icon, 17.0, if on { pal.accent.into() } else { ic }, ac))
            .children(label)
            .tooltip(tooltip(tip))
            .on_click(cx.listener(move |this, _: &ClickEvent, window, cx| this.run(cmd, arg.map(str::to_string), window, cx)))
            .into_any_element()
    }

    /// A button that opens a menu (label and ▾).
    fn obj_menu_button(
        &self,
        id: &'static str,
        icon: &'static str,
        label: &'static str,
        cmd: &'static str,
        pal: &Palette,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        let (ic, ac) = icon_colors(pal);
        div()
            .flex()
            .child(
                div()
                    .id(id)
                    .h(px(30.0))
                    .px(px(8.0))
                    .flex()
                    .items_center()
                    .gap(px(6.0))
                    .rounded(px(5.0))
                    .border_1()
                    .border_color(pal.border_strong)
                    .bg(pal.surface)
                    .hover(|d| d.bg(pal.surface_hover))
                    .cursor_pointer()
                    .text_size(px(12.5))
                    .child(md_icon(icon, 17.0, ic, ac))
                    .child(label)
                    .child(md_icon("chevron-down", 12.0, ic, ac))
                    .on_click(cx.listener(move |this, _: &ClickEvent, window, cx| this.run(cmd, None, window, cx))),
            )
            .into_any_element()
    }

    fn obj_color(&self, id: &'static str, color: Option<u32>, target: ColorTarget, pal: &Palette, cx: &mut Context<Self>) -> AnyElement {
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

    fn obj_pills(
        &self,
        id: &'static str,
        options: &[(&'static str, bool, &'static str)],
        pal: &Palette,
        cx: &mut Context<Self>,
        cmd: fn(&mut Self, &'static str, &mut Context<Self>),
    ) -> AnyElement {
        div()
            .flex()
            .child(div().flex().rounded(px(5.0)).border_1().border_color(pal.border_strong).overflow_hidden().children(
                options.iter().enumerate().map(|(i, (label, on, value))| {
                    let value = *value;
                    div()
                        .id((id, i))
                        .px(px(8.0))
                        .h(px(28.0))
                        .flex()
                        .items_center()
                        .text_size(px(12.5))
                        .cursor_pointer()
                        .when(i > 0, |d| d.border_l_1().border_color(pal.border_strong))
                        .when(*on, |d| d.bg(pal.accent).text_color(pal.on_accent))
                        .when(!*on, |d| d.bg(pal.surface).hover(|d| d.bg(pal.surface_hover)))
                        .child(*label)
                        .on_click(cx.listener(move |this, _: &ClickEvent, _, cx| cmd(this, value, cx)))
                }),
            ))
            .into_any_element()
    }

    fn obj_label(t: &'static str, pal: &Palette) -> gpui::Div {
        div().text_size(px(12.5)).text_color(pal.text_muted).child(t)
    }

    fn obj_section(t: &'static str) -> gpui::Div {
        div().text_size(px(13.5)).font_weight(FontWeight::SEMIBOLD).child(t)
    }

    fn rule(pal: &Palette) -> AnyElement {
        div().h(px(1.0)).bg(pal.border).my(px(4.0)).into_any_element()
    }

    fn not_here(text: &'static str, pal: &Palette) -> AnyElement {
        div().p(px(14.0)).text_size(px(13.0)).text_color(pal.text_muted).child(text).into_any_element()
    }

    // ---- Table settings ------------------------------------------------------------------------

    /// A template as a small table in its colours.
    fn template_tile(&self, i: usize, look: &TableLook, pal: &Palette, cx: &mut Context<Self>) -> AnyElement {
        let t = &TEMPLATES[i];
        let on = look.tpl().id == t.id && look.template.is_some();
        let mut preview = TableLook { template: Some(t.id.to_string()), border: None, fill: None, ..look.clone() };
        preview.banded_rows |= !look.banded_cols;
        let (rows, cols) = (4, 3);
        let border = preview.border();
        let grid = div().flex().flex_col().w(px(52.0)).border_1().border_color(rgb(border.color)).children((0..rows).map(|r| {
            div().flex().h(px(7.0)).children((0..cols).map(|c| {
                let cl = preview.cell(r, c, rows, cols);
                div().flex_1().h_full().bg(rgb(cl.bg.unwrap_or(0xFFFFFF))).when(c > 0, |d| d.border_l_1().border_color(rgb(border.color)))
            }))
        }));
        let id = t.id;
        div()
            .id(("tbl-tpl", i))
            .p(px(4.0))
            .rounded(px(5.0))
            .border_1()
            .border_color(if on { pal.accent } else { pal.border })
            .when(on, |d| d.bg(pal.accent_soft))
            .hover(|d| d.bg(pal.surface_hover))
            .cursor_pointer()
            .child(grid)
            .tooltip(tooltip(t.name))
            .on_click(cx.listener(move |this, _: &ClickEvent, _, cx| {
                this.table_edit(cx, |l| {
                    l.template = Some(id.to_string());
                    // the template's own border colour
                    if let Some(b) = l.border.as_mut() {
                        b.color = md::look::template(id).map(|t| t.border).unwrap_or(b.color);
                    }
                })
            }))
            .into_any_element()
    }

    fn table_panel(&mut self, pal: &Palette, cx: &mut Context<Self>) -> AnyElement {
        let Some(look) = self.table_here(cx) else {
            return Self::not_here("Put the cursor in a table to change its look.", pal);
        };
        let b = look.border();
        let which = if look.border.is_none() { Borders::All } else { b.which };
        let set_which = |this: &mut Self, v: &'static str, cx: &mut Context<Self>| {
            this.table_edit(cx, |l| {
                let b = l.border();
                let which = match v {
                    "outer" => Borders::Outer,
                    "inner" => Borders::Inner,
                    "horizontal" => Borders::Horizontal,
                    "none" => Borders::None,
                    _ => Borders::All,
                };
                l.border = Some(TableBorder { which, ..b });
            })
        };
        let mut tiles = div().flex().flex_wrap().gap(px(6.0));
        for i in 0..TEMPLATES.len() {
            tiles = tiles.child(self.template_tile(i, &look, pal, cx));
        }
        div()
            .p(px(14.0))
            .flex()
            .flex_col()
            .gap(px(10.0))
            .child(Self::obj_section("Rows"))
            .child(self.obj_check("tbl-header", "Header row", look.header, pal, cx, |l| l.header = !l.header))
            .child(self.obj_check("tbl-total", "Total row", look.total, pal, cx, |l| l.total = !l.total))
            .child(self.obj_check("tbl-brows", "Banded rows", look.banded_rows, pal, cx, |l| l.banded_rows = !l.banded_rows))
            .child(Self::obj_section("Columns").mt(px(4.0)))
            .child(self.obj_check("tbl-first", "First column", look.first_col, pal, cx, |l| l.first_col = !l.first_col))
            .child(self.obj_check("tbl-last", "Last column", look.last_col, pal, cx, |l| l.last_col = !l.last_col))
            .child(self.obj_check("tbl-bcols", "Banded columns", look.banded_cols, pal, cx, |l| l.banded_cols = !l.banded_cols))
            .child(Self::rule(pal))
            .child(Self::obj_section("Select from template"))
            .child(tiles)
            .child(Self::rule(pal))
            .child(Self::obj_section("Borders"))
            .child(self.obj_pills(
                "tbl-borders",
                &[
                    ("All", which == Borders::All, "all"),
                    ("Outer", which == Borders::Outer, "outer"),
                    ("Inner", which == Borders::Inner, "inner"),
                    ("Rows", which == Borders::Horizontal, "horizontal"),
                    ("None", which == Borders::None, "none"),
                ],
                pal,
                cx,
                set_which,
            ))
            .child(
                div()
                    .flex()
                    .items_center()
                    .gap(px(10.0))
                    .child(self.obj_spin(ObjField::BorderWidth, 96.0, pal, cx))
                    .child(self.obj_color("tbl-border-color", Some(b.color), ColorTarget::TableBorder, pal, cx))
                    .child(Self::obj_label("Line and color", pal)),
            )
            .child(
                div()
                    .flex()
                    .items_center()
                    .gap(px(10.0))
                    .child(self.obj_color("tbl-fill", look.fill, ColorTarget::TableFill, pal, cx))
                    .child(div().text_size(px(13.0)).child("Background color")),
            )
            .child(Self::rule(pal))
            .child(self.obj_menu_button("tbl-rows-cols", "table-rows", "Rows & columns", "tbl.rowsCols", pal, cx))
            .child(Self::obj_section("Rows & columns size").mt(px(4.0)))
            .child(
                div()
                    .flex()
                    .gap(px(12.0))
                    .child(div().flex().flex_col().gap(px(4.0)).child(Self::obj_label("Table width", pal)).child(self.obj_spin(
                        ObjField::TableWidth,
                        118.0,
                        pal,
                        cx,
                    )))
                    .child(div().flex().flex_col().gap(px(4.0)).child(Self::obj_label("Row height", pal)).child(self.obj_spin(
                        ObjField::RowHeight,
                        118.0,
                        pal,
                        cx,
                    ))),
            )
            .child(self.obj_check("tbl-fit", "Automatically resize to fit content", look.fit, pal, cx, |l| {
                l.fit = !l.fit;
                if l.fit {
                    l.width = None;
                }
            }))
            .child(self.obj_check("tbl-break", "Allow row to break across pages", !look.keep_rows, pal, cx, |l| l.keep_rows = !l.keep_rows))
            .into_any_element()
    }

    // ---- Picture settings ----------------------------------------------------------------------

    fn opacity_slider(&self, opacity: f32, pal: &Palette, cx: &mut Context<Self>) -> AnyElement {
        let value = self.obj_ui.dragging.unwrap_or(opacity);
        let bounds = self.obj_ui.slider.clone();
        let w = 150.0;
        let from = |this: &MarkdownView, x: Pixels| {
            let b = this.obj_ui.slider.get();
            ((f32::from(x - b.left())) / f32::from(b.size.width).max(1.0)).clamp(0.0, 1.0)
        };
        div()
            .id("pic-opacity")
            .relative()
            .w(px(w))
            .h(px(22.0))
            .flex_none()
            .cursor_pointer()
            .child(canvas(move |b, _, _| bounds.set(b), |_, _, _, _| {}).absolute().size_full())
            .child(div().absolute().left_0().right_0().top(px(9.0)).h(px(4.0)).rounded(px(2.0)).bg(pal.border_strong))
            .child(div().absolute().left_0().top(px(9.0)).w(px(w * value)).h(px(4.0)).rounded(px(2.0)).bg(pal.accent))
            .child(
                div()
                    .absolute()
                    .left(px(w * value - 8.0))
                    .top(px(3.0))
                    .size(px(16.0))
                    .rounded_full()
                    .bg(pal.surface)
                    .border_2()
                    .border_color(pal.accent),
            )
            .on_mouse_down(
                MouseButton::Left,
                cx.listener(move |this, e: &MouseDownEvent, _, cx| {
                    this.obj_ui.dragging = Some(from(this, e.position.x));
                    cx.notify();
                }),
            )
            .on_mouse_move(cx.listener(move |this, e: &MouseMoveEvent, _, cx| {
                if this.obj_ui.dragging.is_some() && e.pressed_button == Some(MouseButton::Left) {
                    this.obj_ui.dragging = Some(from(this, e.position.x));
                    cx.notify();
                }
            }))
            .on_mouse_up(
                MouseButton::Left,
                cx.listener(|this, _, _, cx| {
                    if let Some(v) = this.obj_ui.dragging.take() {
                        this.obj_set(ObjField::Opacity, Some((v * 100.0).round()), cx);
                    }
                }),
            )
            .on_mouse_up_out(
                MouseButton::Left,
                cx.listener(|this, _, _, cx| {
                    if let Some(v) = this.obj_ui.dragging.take() {
                        this.obj_set(ObjField::Opacity, Some((v * 100.0).round()), cx);
                    }
                }),
            )
            .into_any_element()
    }

    #[allow(clippy::too_many_arguments)]
    fn wrap_tile(
        &self,
        i: usize,
        icon: &'static str,
        label: &'static str,
        value: &'static str,
        on: bool,
        pal: &Palette,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        let (ic, ac) = icon_colors(pal);
        div()
            .id(("pic-wrap", i))
            .w(px(62.0))
            .py(px(6.0))
            .flex()
            .flex_col()
            .items_center()
            .gap(px(3.0))
            .rounded(px(5.0))
            .border_1()
            .border_color(if on { pal.accent } else { pal.border })
            .when(on, |d| d.bg(pal.accent_soft))
            .hover(|d| d.bg(pal.surface_hover))
            .cursor_pointer()
            .child(md_icon(icon, 22.0, if on { pal.accent.into() } else { ic }, ac))
            .child(div().text_size(px(11.0)).text_center().child(label))
            .on_click(cx.listener(move |this, _: &ClickEvent, window, cx| this.run("pic.wrap", Some(value.to_string()), window, cx)))
            .into_any_element()
    }

    fn picture_panel(&mut self, pal: &Palette, cx: &mut Context<Self>) -> AnyElement {
        let Some(img) = self.image_here(cx) else {
            return Self::not_here("Put the cursor on a picture to change it.", pal);
        };
        let st = img.style.clone();
        // the height that follows from the width
        let natural = self.natural_size(&img.url);
        let height = natural.map(|(w, h)| {
            let width = st.width.unwrap_or(w / 96.0 * 2.54);
            let (_, hh) = md::look::styled_size(w, h, &ImgStyle { rotate: 0.0, ..st.clone() }, Some(width));
            hh
        });
        let size_note = match (height, natural) {
            (Some(h), Some((w, hpx))) => format!("Height {} cm · picture {} × {} px", fmt(h), w as u32, hpx as u32),
            _ => "The height follows the width".to_string(),
        };
        div()
            .p(px(14.0))
            .flex()
            .flex_col()
            .gap(px(10.0))
            .child(Self::obj_section("Size"))
            .child(div().flex().items_center().gap(px(10.0)).child(
                div().flex().flex_col().gap(px(4.0)).child(Self::obj_label("Width", pal)).child(self.obj_spin(
                    ObjField::PicWidth,
                    118.0,
                    pal,
                    cx,
                )),
            ))
            .child(Self::obj_label("", pal).child(size_note))
            .child(
                div()
                    .flex()
                    .flex_wrap()
                    .gap(px(6.0))
                    .child(self.obj_button(
                        "pic-actual",
                        "zoom-100",
                        Some("Actual size"),
                        "The picture's own size",
                        st.width.is_none(),
                        pal,
                        cx,
                        "pic.actual",
                        None,
                    ))
                    .child(self.obj_button(
                        "pic-fit",
                        "wrap-block",
                        Some("Fit to margin"),
                        "As wide as the text",
                        false,
                        pal,
                        cx,
                        "pic.fit",
                        None,
                    ))
                    .child(self.obj_menu_button("pic-crop", "crop", "Crop", "pic.crop", pal, cx)),
            )
            .child(Self::rule(pal))
            .child(Self::obj_section("Opacity"))
            .child(
                div()
                    .flex()
                    .items_center()
                    .gap(px(10.0))
                    .child(self.opacity_slider(st.opacity.unwrap_or(1.0), pal, cx))
                    .child(self.obj_spin(ObjField::Opacity, 86.0, pal, cx)),
            )
            .child(Self::rule(pal))
            .child(Self::obj_section("Rotation"))
            .child(
                div()
                    .flex()
                    .items_center()
                    .gap(px(6.0))
                    .child(self.obj_button("pic-rot-l", "rotate-left", None, "Rotate left 90°", false, pal, cx, "pic.rotateLeft", None))
                    .child(self.obj_button("pic-rot-r", "rotate-right", None, "Rotate right 90°", false, pal, cx, "pic.rotateRight", None))
                    .child(self.obj_spin(ObjField::Angle, 86.0, pal, cx)),
            )
            .child(
                div()
                    .flex()
                    .gap(px(6.0))
                    .child(self.obj_button(
                        "pic-flip-h",
                        "flip-h",
                        Some("Flip horizontal"),
                        "Mirror left to right",
                        st.flip_h,
                        pal,
                        cx,
                        "pic.flipH",
                        None,
                    ))
                    .child(self.obj_button(
                        "pic-flip-v",
                        "flip-v",
                        Some("Flip vertical"),
                        "Mirror top to bottom",
                        st.flip_v,
                        pal,
                        cx,
                        "pic.flipV",
                        None,
                    )),
            )
            .child(Self::rule(pal))
            .child(Self::obj_section("Wrapping style"))
            .child(
                div()
                    .flex()
                    .gap(px(6.0))
                    .child(self.wrap_tile(0, "wrap-inline", "In line", "inline", st.wrap == Wrap::Inline, pal, cx))
                    .child(self.wrap_tile(1, "wrap-left", "Left", "left", st.wrap == Wrap::Left, pal, cx))
                    .child(self.wrap_tile(2, "wrap-right", "Right", "right", st.wrap == Wrap::Right, pal, cx))
                    .child(self.wrap_tile(3, "wrap-block", "Top and bottom", "block", st.wrap == Wrap::Block, pal, cx)),
            )
            .child(Self::rule(pal))
            .child(self.obj_menu_button("pic-replace", "replace", "Replace image", "pic.replace", pal, cx))
            .child(
                div()
                    .id("pic-reset")
                    .flex()
                    .justify_center()
                    .text_size(px(12.0))
                    .text_color(pal.text_muted)
                    .cursor_pointer()
                    .hover(|d| d.underline())
                    .child("Reset picture")
                    .on_click(cx.listener(|this, _: &ClickEvent, _, cx| this.picture_edit(cx, |s| *s = ImgStyle::default()))),
            )
            .into_any_element()
    }

    /// The body of the Table or Picture panel.
    pub(super) fn object_panel(&mut self, panel: RightPanel, pal: &Palette, window: &Window, cx: &mut Context<Self>) -> AnyElement {
        self.obj_sync(window, cx);
        match panel {
            RightPanel::Table => self.table_panel(pal, cx),
            _ => self.picture_panel(pal, cx),
        }
    }
}
