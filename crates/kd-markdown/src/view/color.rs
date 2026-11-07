//! The colour picker of the Font color, Highlight color and Shading buttons: Automatic, the
//! theme's colours with lighter and darker shades, the standard colours, the recent ones and
//! More colors… (a square of saturation and brightness, a hue strip, R G B and #hex).

use std::cell::Cell;
use std::rc::Rc;

use gpui::{
    AnyElement, Bounds, ClickEvent, Context, FontWeight, Hsla, IntoElement, MouseButton, MouseDownEvent, MouseMoveEvent, Pixels, Window,
    canvas, deferred, div, linear_color_stop, linear_gradient, prelude::*, px, rgb,
};
use kd_ui::{Palette, TextInput, button, primary_button, tooltip};

use super::{MarkdownView, icon_colors, md_icon};
use crate::ribbon::ColorTarget;

/// The theme colours (Office's), as Word shows them.
pub const THEME: [u32; 10] = [0xFFFFFF, 0x000000, 0xE7E6E6, 0x44546A, 0x4472C4, 0xED7D31, 0xA5A5A5, 0xFFC000, 0x5B9BD5, 0x70AD47];
pub const STANDARD: [u32; 10] = [0xC00000, 0xFF0000, 0xFFC000, 0xFFFF00, 0x92D050, 0x00B050, 0x00B0F0, 0x0070C0, 0x002060, 0x7030A0];

fn channels(c: u32) -> (f32, f32, f32) {
    (((c >> 16) & 255) as f32, ((c >> 8) & 255) as f32, (c & 255) as f32)
}

fn pack(r: f32, g: f32, b: f32) -> u32 {
    let k = |v: f32| v.round().clamp(0.0, 255.0) as u32;
    (k(r) << 16) | (k(g) << 8) | k(b)
}

/// A shade of a theme colour: `t` > 0 lighter (towards white), < 0 darker.
pub fn shade(c: u32, t: f32) -> u32 {
    let (r, g, b) = channels(c);
    if t >= 0.0 {
        pack(r + (255.0 - r) * t, g + (255.0 - g) * t, b + (255.0 - b) * t)
    } else {
        pack(r * (1.0 + t), g * (1.0 + t), b * (1.0 + t))
    }
}

/// The five shades under each theme colour (as Word: lighter 80/60/40 %, darker 25/50 %; for
/// white and black darker or lighter only).
fn shades(c: u32) -> [u32; 5] {
    match c {
        0xFFFFFF => [shade(c, -0.05), shade(c, -0.15), shade(c, -0.25), shade(c, -0.35), shade(c, -0.5)],
        0x000000 => [shade(c, 0.5), shade(c, 0.35), shade(c, 0.25), shade(c, 0.15), shade(c, 0.05)],
        _ => [shade(c, 0.8), shade(c, 0.6), shade(c, 0.4), shade(c, -0.25), shade(c, -0.5)],
    }
}

/// RGB → (hue 0–360, saturation 0–1, value 0–1).
pub fn to_hsv(c: u32) -> (f32, f32, f32) {
    let (r, g, b) = channels(c);
    let (r, g, b) = (r / 255.0, g / 255.0, b / 255.0);
    let max = r.max(g).max(b);
    let min = r.min(g).min(b);
    let d = max - min;
    let h = if d == 0.0 {
        0.0
    } else if max == r {
        60.0 * (((g - b) / d).rem_euclid(6.0))
    } else if max == g {
        60.0 * ((b - r) / d + 2.0)
    } else {
        60.0 * ((r - g) / d + 4.0)
    };
    (h, if max == 0.0 { 0.0 } else { d / max }, max)
}

pub fn from_hsv(h: f32, s: f32, v: f32) -> u32 {
    let c = v * s;
    let x = c * (1.0 - ((h / 60.0).rem_euclid(2.0) - 1.0).abs());
    let m = v - c;
    let (r, g, b) = match (h.rem_euclid(360.0) / 60.0) as u32 {
        0 => (c, x, 0.0),
        1 => (x, c, 0.0),
        2 => (0.0, c, x),
        3 => (0.0, x, c),
        4 => (x, 0.0, c),
        _ => (c, 0.0, x),
    };
    pack((r + m) * 255.0, (g + m) * 255.0, (b + m) * 255.0)
}

/// More colors…: the colour being made.
pub struct ColorDialog {
    pub target: ColorTarget,
    pub h: f32,
    pub s: f32,
    pub v: f32,
    /// The colour before (shown as Current).
    pub current: Option<u32>,
    /// What is being dragged: the square or the hue strip.
    drag: Option<bool>,
    square: Rc<Cell<Bounds<Pixels>>>,
    strip: Rc<Cell<Bounds<Pixels>>>,
    pub inputs: [gpui::Entity<TextInput>; 4],
}

impl ColorDialog {
    fn color(&self) -> u32 {
        from_hsv(self.h, self.s, self.v)
    }
}

fn swatch(c: u32) -> Hsla {
    rgb(c).into()
}

impl MarkdownView {
    /// Apply a colour (`None`: Automatic / no colour) and remember it.
    pub(super) fn pick_color(&mut self, target: ColorTarget, c: Option<u32>, window: &mut Window, cx: &mut Context<Self>) {
        self.menu = None;
        if let Some(c) = c {
            self.recent_colors.retain(|x| *x != c);
            self.recent_colors.insert(0, c);
            self.recent_colors.truncate(10);
        }
        let arg = c.map(|c| format!("#{c:06X}")).unwrap_or_default();
        self.run(target.command(), Some(arg), window, cx);
        cx.notify();
    }

    /// The picker under a colour button.
    pub(super) fn color_menu(&mut self, target: ColorTarget, pal: &Palette, cx: &mut Context<Self>) -> AnyElement {
        let (ic, ac) = icon_colors(pal);
        let cell = |id: (&'static str, usize), c: u32, size: f32, cx: &mut Context<Self>| {
            div()
                .id(id)
                .size(px(size))
                .border_1()
                .border_color(Hsla::from(pal.border_strong).opacity(0.6))
                .bg(swatch(c))
                .cursor_pointer()
                .hover(|d| d.border_color(pal.text).border_2())
                .tooltip(tooltip(format!("#{c:06X}")))
                .on_click(cx.listener(move |this, _: &ClickEvent, window, cx| this.pick_color(target, Some(c), window, cx)))
        };
        let header =
            |t: &'static str| div().px(px(12.0)).pt(px(10.0)).pb(px(6.0)).text_size(px(13.0)).font_weight(FontWeight::MEDIUM).child(t);
        let row = |children: Vec<AnyElement>| div().px(px(12.0)).flex().gap(px(4.0)).children(children);
        let automatic = div()
            .id("color-auto")
            .flex()
            .items_center()
            .gap(px(10.0))
            .px(px(12.0))
            .py(px(8.0))
            .cursor_pointer()
            .hover(|d| d.bg(pal.surface_hover))
            .child(div().size(px(18.0)).border_1().border_color(pal.border_strong).bg(if target == ColorTarget::Text {
                swatch(0)
            } else {
                gpui::transparent_black()
            }))
            .child(if target == ColorTarget::Text { "Automatic" } else { "No color" })
            .on_click(cx.listener(move |this, _: &ClickEvent, window, cx| this.pick_color(target, None, window, cx)));
        let theme_row: Vec<AnyElement> =
            THEME.iter().enumerate().map(|(i, c)| cell(("color-theme", i), *c, 20.0, cx).into_any_element()).collect();
        // the shades: one column per theme colour
        let shade_cols: Vec<AnyElement> = THEME
            .iter()
            .enumerate()
            .map(|(i, c)| {
                div()
                    .flex()
                    .flex_col()
                    .children(
                        shades(*c).iter().enumerate().map(|(k, s)| cell(("color-shade", i * 10 + k), *s, 20.0, cx).into_any_element()),
                    )
                    .into_any_element()
            })
            .collect();
        let standard: Vec<AnyElement> =
            STANDARD.iter().enumerate().map(|(i, c)| cell(("color-std", i), *c, 20.0, cx).into_any_element()).collect();
        let recent: Vec<AnyElement> = self
            .recent_colors
            .clone()
            .iter()
            .enumerate()
            .map(|(i, c)| cell(("color-recent", i), *c, 20.0, cx).into_any_element())
            .collect();
        let has_recent = !recent.is_empty();
        div()
            .w(px(264.0))
            .pb(px(6.0))
            .flex()
            .flex_col()
            .child(automatic)
            .child(div().h(px(1.0)).bg(pal.border))
            .child(header("Theme colors"))
            .child(row(theme_row))
            .child(div().h(px(8.0)))
            .child(row(shade_cols))
            .child(header("Standard colors"))
            .child(row(standard))
            .when(has_recent, |d| d.child(header("Recent colors")).child(row(recent)))
            .child(div().mt(px(10.0)).h(px(1.0)).bg(pal.border))
            .child(
                div()
                    .id("color-more")
                    .flex()
                    .items_center()
                    .gap(px(10.0))
                    .px(px(12.0))
                    .py(px(8.0))
                    .cursor_pointer()
                    .hover(|d| d.bg(pal.surface_hover))
                    .child(md_icon("palette", 18.0, ic, ac))
                    .child("More colors…")
                    .on_click(cx.listener(move |this, _: &ClickEvent, window, cx| this.open_more_colors(target, window, cx))),
            )
            .into_any_element()
    }

    pub(super) fn open_more_colors(&mut self, target: ColorTarget, window: &mut Window, cx: &mut Context<Self>) {
        self.menu = None;
        let current = self.current_color(target, cx);
        let start = current.or(self.recent_colors.first().copied()).unwrap_or(0xFF0000);
        let (h, s, v) = to_hsv(start);
        let inputs = [
            cx.new(|cx| TextInput::new(cx, "R")),
            cx.new(|cx| TextInput::new(cx, "G")),
            cx.new(|cx| TextInput::new(cx, "B")),
            cx.new(|cx| TextInput::new(cx, "RRGGBB")),
        ];
        for (k, input) in inputs.iter().enumerate() {
            let sub = cx.subscribe_in(input, window, move |this, _, e: &kd_ui::InputEvent, window, cx| match e {
                kd_ui::InputEvent::Submit => {
                    this.color_from_inputs(k, cx);
                    this.add_more_color(window, cx);
                }
                kd_ui::InputEvent::Changed => this.color_from_inputs(k, cx),
                kd_ui::InputEvent::Escape => {
                    this.more_colors = None;
                    cx.notify();
                }
            });
            self._subs.push(sub);
        }
        self.more_colors = Some(ColorDialog { target, h, s, v, current, drag: None, square: Rc::default(), strip: Rc::default(), inputs });
        self.sync_color_inputs(None, cx);
        cx.notify();
    }

    /// The colour the selection or paragraph has now.
    fn current_color(&self, target: ColorTarget, cx: &Context<Self>) -> Option<u32> {
        let ed = self.editor.read(cx);
        let (t, head) = (ed.text(), ed.selection().head);
        match target {
            ColorTarget::Text => kd_formats::md::style::style_value(t, head, "color").and_then(|c| kd_formats::md::para::css_color(&c)),
            ColorTarget::Highlight => {
                kd_formats::md::style::style_value(t, head, "background-color").and_then(|c| kd_formats::md::para::css_color(&c))
            }
            ColorTarget::Shading => kd_formats::md::para::style_at(t, head).background,
            ColorTarget::Border => kd_formats::md::para::style_at(t, head).border.map(|b| b.color),
            ColorTarget::TableFill => kd_formats::md::look::table_look_at(t, head).and_then(|l| l.fill),
            ColorTarget::TableBorder => kd_formats::md::look::table_look_at(t, head).map(|l| l.border().color),
        }
    }

    /// Show the colour in the R, G, B and hex boxes (except the one being typed in).
    fn sync_color_inputs(&mut self, skip: Option<usize>, cx: &mut Context<Self>) {
        let Some(d) = &self.more_colors else { return };
        let c = d.color();
        let (r, g, b) = channels(c);
        let values = [format!("{r}"), format!("{g}"), format!("{b}"), format!("{c:06X}")];
        let inputs = d.inputs.clone();
        for (k, (input, v)) in inputs.iter().zip(values).enumerate() {
            if Some(k) != skip {
                input.update(cx, |i, cx| i.set_text(v, cx));
            }
        }
    }

    fn color_from_inputs(&mut self, changed: usize, cx: &mut Context<Self>) {
        let Some(d) = &self.more_colors else { return };
        let text: Vec<String> = d.inputs.iter().map(|i| i.read(cx).text().trim().to_string()).collect();
        let c = if changed == 3 {
            match u32::from_str_radix(text[3].trim_start_matches('#'), 16) {
                Ok(c) if text[3].trim_start_matches('#').len() == 6 => c,
                _ => return,
            }
        } else {
            let n = |s: &str| s.parse::<f32>().ok().map(|v| v.clamp(0.0, 255.0));
            match (n(&text[0]), n(&text[1]), n(&text[2])) {
                (Some(r), Some(g), Some(b)) => pack(r, g, b),
                _ => return,
            }
        };
        // the boxes were filled from the colour: nothing changed (and no endless round of updates)
        if self.more_colors.as_ref().is_some_and(|d| d.color() == c) {
            return;
        }
        let (h, s, v) = to_hsv(c);
        if let Some(d) = self.more_colors.as_mut() {
            // grey has no hue: keep the strip where it is
            if s > 0.0 {
                d.h = h;
            }
            d.s = s;
            d.v = v;
        }
        self.sync_color_inputs(Some(changed), cx);
        cx.notify();
    }

    fn add_more_color(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let Some(d) = self.more_colors.take() else { return };
        self.pick_color(d.target, Some(d.color()), window, cx);
    }

    /// A drag in the square (saturation × brightness) or the hue strip.
    fn color_drag(&mut self, at: gpui::Point<Pixels>, cx: &mut Context<Self>) {
        let Some(d) = self.more_colors.as_mut() else { return };
        let Some(square) = d.drag else { return };
        let b = if square { d.square.get() } else { d.strip.get() };
        let fx = ((f32::from(at.x - b.left())) / f32::from(b.size.width).max(1.0)).clamp(0.0, 1.0);
        let fy = ((f32::from(at.y - b.top())) / f32::from(b.size.height).max(1.0)).clamp(0.0, 1.0);
        if square {
            d.s = fx;
            d.v = 1.0 - fy;
        } else {
            d.h = fy * 360.0;
        }
        self.sync_color_inputs(None, cx);
        cx.notify();
    }

    /// More colors… as a window over the editor.
    pub(super) fn more_colors_card(&mut self, pal: &Palette, cx: &mut Context<Self>) -> Option<AnyElement> {
        let d = self.more_colors.as_ref()?;
        let (h, s, v) = (d.h, d.s, d.v);
        let new = d.color();
        let current = d.current;
        let hue = from_hsv(h, 1.0, 1.0);
        let (sq, st) = (d.square.clone(), d.strip.clone());
        let size = 260.0;
        let knob = |x: f32, y: f32, c: u32| {
            div()
                .absolute()
                .left(px(x - 11.0))
                .top(px(y - 11.0))
                .size(px(22.0))
                .rounded_full()
                .border(px(3.0))
                .border_color(gpui::white())
                .shadow_md()
                .bg(swatch(c))
        };
        let square = div()
            .id("color-square")
            .relative()
            .size(px(size))
            .flex_none()
            .rounded(px(4.0))
            .bg(linear_gradient(90.0, linear_color_stop(gpui::white(), 0.0), linear_color_stop(swatch(hue), 1.0)))
            .child(div().absolute().size_full().rounded(px(4.0)).bg(linear_gradient(
                180.0,
                linear_color_stop(gpui::transparent_black(), 0.0),
                linear_color_stop(gpui::black(), 1.0),
            )))
            .child(canvas(move |b, _, _| sq.set(b), |_, _, _, _| {}).absolute().size_full())
            .child(knob(s * size, (1.0 - v) * size, new))
            .on_mouse_down(
                MouseButton::Left,
                cx.listener(|this, e: &MouseDownEvent, _, cx| {
                    if let Some(d) = this.more_colors.as_mut() {
                        d.drag = Some(true);
                    }
                    this.color_drag(e.position, cx);
                }),
            );
        // the hue strip: six gradients from red round to red
        let stops: [u32; 7] = [0xFF0000, 0xFFFF00, 0x00FF00, 0x00FFFF, 0x0000FF, 0xFF00FF, 0xFF0000];
        let strip = div()
            .id("color-strip")
            .relative()
            .w(px(22.0))
            .h(px(size))
            .flex_none()
            .flex()
            .flex_col()
            .children((0..6).map(|i| {
                div().w_full().flex_1().bg(linear_gradient(
                    180.0,
                    linear_color_stop(swatch(stops[i]), 0.0),
                    linear_color_stop(swatch(stops[i + 1]), 1.0),
                ))
            }))
            .child(canvas(move |b, _, _| st.set(b), |_, _, _, _| {}).absolute().size_full())
            .child(knob(11.0, h / 360.0 * size, hue))
            .on_mouse_down(
                MouseButton::Left,
                cx.listener(|this, e: &MouseDownEvent, _, cx| {
                    if let Some(d) = this.more_colors.as_mut() {
                        d.drag = Some(false);
                    }
                    this.color_drag(e.position, cx);
                }),
            );
        let field = |label: &'static str, input: gpui::Entity<TextInput>| {
            div().flex().items_center().gap(px(8.0)).child(div().w(px(14.0)).text_size(px(14.0)).child(label)).child(
                div()
                    .w(px(84.0))
                    .h(px(30.0))
                    .px(px(8.0))
                    .flex()
                    .items_center()
                    .rounded(px(5.0))
                    .border_1()
                    .border_color(pal.border_strong)
                    .bg(pal.surface)
                    .child(input),
            )
        };
        let inputs = d.inputs.clone();
        let side = div()
            .w(px(120.0))
            .flex()
            .flex_col()
            .items_center()
            .gap(px(8.0))
            .child(div().text_size(px(13.0)).child("New"))
            .child(
                div()
                    .w(px(96.0))
                    .h(px(64.0))
                    .flex()
                    .flex_col()
                    .border_1()
                    .border_color(pal.border_strong)
                    .child(div().flex_1().bg(swatch(new)))
                    .child(div().flex_1().bg(current.map(swatch).unwrap_or(gpui::transparent_black()))),
            )
            .child(div().text_size(px(13.0)).child("Current"))
            .child(div().h(px(4.0)))
            .child(field("R", inputs[0].clone()))
            .child(field("G", inputs[1].clone()))
            .child(field("B", inputs[2].clone()))
            .child(field("#", inputs[3].clone()));
        let card = div()
            .occlude()
            .p(px(20.0))
            .rounded(px(10.0))
            .bg(pal.surface)
            .border_1()
            .border_color(pal.border)
            .shadow_lg()
            .flex()
            .flex_col()
            .gap(px(16.0))
            .child(div().flex().gap(px(16.0)).child(square).child(strip).child(side))
            .child(
                div()
                    .flex()
                    .justify_center()
                    .gap(px(10.0))
                    .child(
                        primary_button("color-add", "Add", pal)
                            .w(px(110.0))
                            .on_click(cx.listener(|this, _: &ClickEvent, window, cx| this.add_more_color(window, cx))),
                    )
                    .child(button("color-cancel", "Cancel", pal).w(px(110.0)).on_click(cx.listener(|this, _: &ClickEvent, _, cx| {
                        this.more_colors = None;
                        cx.notify();
                    }))),
            );
        Some(
            deferred(
                div()
                    .absolute()
                    .inset_0()
                    .flex()
                    .items_center()
                    .justify_center()
                    .bg(Hsla::from(pal.text).opacity(0.12))
                    .on_mouse_move(cx.listener(|this, e: &MouseMoveEvent, _, cx| {
                        if e.pressed_button == Some(MouseButton::Left) {
                            this.color_drag(e.position, cx);
                        }
                    }))
                    .on_mouse_up(
                        MouseButton::Left,
                        cx.listener(|this, _, _, cx| {
                            if let Some(d) = this.more_colors.as_mut() {
                                d.drag = None;
                            }
                            cx.notify();
                        }),
                    )
                    .child(card),
            )
            .with_priority(5)
            .into_any_element(),
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn hsv_round_trip_and_shades() {
        for c in [0xFF0000, 0x4472C4, 0x70AD47, 0x123456, 0xFFFFFF, 0x000000] {
            let (h, s, v) = to_hsv(c);
            assert_eq!(from_hsv(h, s, v), c, "{c:06X}");
        }
        assert_eq!(shade(0x000000, 0.5), 0x808080);
        assert_eq!(shade(0xFFFFFF, -0.5), 0x808080);
        assert_eq!(shades(0x4472C4)[0], shade(0x4472C4, 0.8));
    }
}
