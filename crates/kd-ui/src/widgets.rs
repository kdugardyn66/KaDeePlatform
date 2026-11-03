//! Small shared widgets, drawn from the [`Palette`]: icons, badges, pills, buttons, the logo and
//! tooltips. Each returns a GPUI element the caller finishes (adds `on_click`, sizes …).

use gpui::{
    AnyView, App, Div, ElementId, FontWeight, Hsla, IntoElement, ParentElement, Render, SharedString, Stateful, Styled, Svg, Window, div,
    prelude::*, px, svg,
};
use kd_store::{DocKind, ModuleKind};

use crate::icons::Icon;
use crate::theme::{Palette, Theme, mix};

/// An icon at `size` pixels in `color`.
pub fn icon(icon: Icon, size: f32, color: impl Into<Hsla>) -> Svg {
    svg().path(icon.path()).size(px(size)).flex_none().text_color(color.into())
}

/// A rounded square with short text: the badge of a module or document type.
pub fn badge(text: impl Into<SharedString>, size: f32, color: gpui::Rgba, pal: &Palette) -> Div {
    let text: SharedString = text.into();
    let scale = if text.len() > 2 { 0.30 } else { 0.34 };
    div()
        .size(px(size))
        .flex_none()
        .flex()
        .items_center()
        .justify_center()
        .rounded(px(size * 0.24))
        .bg(pal.soft(color))
        .text_color(color)
        .text_size(px((size * scale).max(8.5)))
        .font_weight(FontWeight::MEDIUM)
        .child(text)
}

pub fn module_badge(kind: ModuleKind, size: f32, pal: &Palette) -> Div {
    badge(kind.badge(), size, pal.module(kind), pal)
}

pub fn doc_badge(kind: &DocKind, size: f32, pal: &Palette) -> Div {
    badge(kind.badge(), size, pal.group(kind.group), pal)
}

/// A rounded filter "pill".
pub fn pill(id: impl Into<ElementId>, label: impl Into<SharedString>, selected: bool, pal: &Palette) -> Stateful<Div> {
    let base = div()
        .id(id)
        .h(px(30.0))
        .px(px(14.0))
        .flex()
        .items_center()
        .rounded(px(15.0))
        .border_1()
        .text_size(px(13.5))
        .cursor_pointer()
        .child(label.into());
    if selected {
        base.bg(pal.text).border_color(pal.text).text_color(pal.surface)
    } else {
        let (hover_bg, hover_border, hover_text) = (pal.surface_hover, pal.border_strong, pal.text);
        base.bg(pal.surface)
            .border_color(pal.border)
            .text_color(pal.text_muted)
            .hover(move |s| s.bg(hover_bg).border_color(hover_border).text_color(hover_text))
    }
}

/// A square icon button with a hover background.
pub fn icon_button(id: impl Into<ElementId>, which: Icon, size: f32, pal: &Palette) -> Stateful<Div> {
    let hover = pal.surface_hover;
    let (muted, text) = (pal.text_muted, pal.text);
    div()
        .id(id)
        .size(px(size))
        .flex_none()
        .flex()
        .items_center()
        .justify_center()
        .rounded(px(6.0))
        .cursor_pointer()
        .text_color(muted)
        .hover(move |s| s.bg(hover).text_color(text))
        .child(svg().path(which.path()).size(px((size * 0.54).round())).text_color(muted))
}

/// A plain (secondary) button.
pub fn button(id: impl Into<ElementId>, label: impl Into<SharedString>, pal: &Palette) -> Stateful<Div> {
    let (hover, strong) = (pal.surface_hover, pal.border_strong);
    div()
        .id(id)
        .h(px(32.0))
        .px(px(12.0))
        .flex()
        .flex_none()
        .items_center()
        .justify_center()
        .gap(px(6.0))
        .rounded(px(7.0))
        .border_1()
        .border_color(pal.border)
        .bg(pal.surface)
        .text_color(pal.text)
        .text_size(px(13.5))
        .cursor_pointer()
        .hover(move |s| s.bg(hover).border_color(strong))
        .child(label.into())
}

/// The accent button.
pub fn primary_button(id: impl Into<ElementId>, label: impl Into<SharedString>, pal: &Palette) -> Stateful<Div> {
    let hover = mix(pal.accent, pal.canvas, 0.12);
    div()
        .id(id)
        .h(px(32.0))
        .px(px(14.0))
        .flex()
        .flex_none()
        .items_center()
        .justify_center()
        .rounded(px(7.0))
        .bg(pal.accent)
        .text_color(pal.on_accent)
        .text_size(px(13.5))
        .font_weight(FontWeight::MEDIUM)
        .cursor_pointer()
        .hover(move |s| s.bg(hover))
        .child(label.into())
}

/// A red button for destructive actions.
pub fn danger_button(id: impl Into<ElementId>, label: impl Into<SharedString>, pal: &Palette) -> Stateful<Div> {
    let hover = mix(pal.danger, pal.canvas, 0.12);
    primary_button(id, label, pal).bg(pal.danger).text_color(gpui::white()).hover(move |s| s.bg(hover))
}

/// The KaDee logo: a K drawn in a 3×3 grid of module tiles, with an AI spark on the amber
/// centre tile (the geometry of `assets/app-icon/kadeestudio.svg`, a 256-unit grid).
pub fn logo(size: f32, pal: &Palette) -> Div {
    let u = size / 256.0;
    let (tile, edge, on, off) = if pal.dark {
        (gpui::rgb(0x232838), gpui::rgb(0x343B52), gpui::rgb(0x6E93F2), gpui::rgb(0x343B52))
    } else {
        (gpui::rgb(0xFFFFFF), gpui::rgb(0xE3E0D9), gpui::rgb(0x2F5DD6), gpui::rgb(0xDCE4F8))
    };
    let amber = gpui::rgb(0xFFB020);
    let k = [(0, 0), (0, 1), (0, 2), (2, 0), (2, 2)];
    let mut root = div().size(px(size)).flex_none().relative().rounded(px(58.0 * u)).bg(tile).border_1().border_color(edge);
    for i in 0..3 {
        for j in 0..3 {
            let color = if (i, j) == (1, 1) {
                amber
            } else if k.contains(&(i, j)) {
                on
            } else {
                off
            };
            let mut cell = div()
                .absolute()
                .left(px((41.0 + 66.0 * i as f32) * u - 1.0))
                .top(px((41.0 + 66.0 * j as f32) * u - 1.0))
                .size(px(54.0 * u))
                .rounded(px((15.0 * u).max(1.0)))
                .bg(color);
            if (i, j) == (1, 1) {
                cell = cell
                    .flex()
                    .items_center()
                    .justify_center()
                    .child(svg().path(Icon::Spark.path()).size(px(40.0 * u)).text_color(gpui::white()));
            }
            root = root.child(cell);
        }
    }
    root
}

/// A tooltip with one line of text; use with `.tooltip(tooltip("…"))`.
pub fn tooltip(text: impl Into<SharedString>) -> impl Fn(&mut Window, &mut App) -> AnyView + 'static {
    let text = text.into();
    move |_window, cx| cx.new(|_| Tip(text.clone())).into()
}

struct Tip(SharedString);

impl Render for Tip {
    fn render(&mut self, _window: &mut Window, cx: &mut gpui::Context<Self>) -> impl IntoElement {
        let pal = Theme::palette(cx);
        div()
            .mt(px(6.0))
            .px(px(9.0))
            .py(px(5.0))
            .rounded(px(6.0))
            .bg(pal.text)
            .text_color(pal.surface)
            .text_size(px(12.0))
            .shadow_md()
            .child(self.0.clone())
    }
}

/// Shorten `s` to `max` characters with an ellipsis.
pub fn truncate(s: &str, max: usize) -> String {
    if s.chars().count() <= max {
        s.to_string()
    } else {
        let t: String = s.chars().take(max.saturating_sub(1)).collect();
        format!("{t}…")
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn truncate_adds_ellipsis() {
        assert_eq!(super::truncate("KadeeStudio", 20), "KadeeStudio");
        assert_eq!(super::truncate("KadeeStudio", 6), "Kadee…");
    }
}
