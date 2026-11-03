//! The documents table: Name, Opened, Modified, Size; a star and a … menu on hover.

use std::path::PathBuf;

use gpui::{Context, IntoElement, MouseButton, MouseDownEvent, ParentElement, Pixels, Styled, div, prelude::*, px};
use kd_commands::{Command, Page};
use kd_store::{FileEntry, human_size, now, relative_time};
use kd_ui::{Icon, Palette, Theme, doc_badge, icon, tooltip};

use super::on_cmd;
use crate::app::{KadeeApp, MenuKind, SortKey, file_name};

const OPENED_W: f32 = 150.0;
const MODIFIED_W: f32 = 150.0;
const SIZE_W: f32 = 96.0;
const ACTIONS_W: f32 = 64.0;

pub fn file_table(
    app: &KadeeApp,
    folders: &[PathBuf],
    entries: &[FileEntry],
    width: Pixels,
    cx: &mut Context<KadeeApp>,
) -> impl IntoElement {
    let pal = Theme::palette(cx);
    let narrow = f32::from(width) < 720.0;
    let header = div()
        .h(px(42.0))
        .flex()
        .items_center()
        .px(px(16.0))
        .border_b_1()
        .border_color(pal.border)
        .text_size(px(12.5))
        .text_color(pal.text_faint)
        .child(div().w(px(46.0)).flex_none())
        .child(sort_head(app, "NAME", SortKey::Name, None, &pal, cx))
        .when(!narrow, |d| d.child(sort_head(app, "OPENED", SortKey::Opened, Some(OPENED_W), &pal, cx)))
        .child(sort_head(app, "MODIFIED", SortKey::Modified, Some(MODIFIED_W), &pal, cx))
        .when(!narrow, |d| d.child(sort_head(app, "SIZE", SortKey::Size, Some(SIZE_W), &pal, cx)))
        .child(div().w(px(ACTIONS_W)).flex_none());

    let mut body = div().flex().flex_col().px(px(8.0)).py(px(4.0));
    let t = now();
    for (i, dir) in folders.iter().enumerate() {
        body = body.child(folder_row(i, dir, narrow, &pal, cx));
    }
    for (i, e) in entries.iter().enumerate() {
        body = body.child(row(app, i, e, t, narrow, &pal, cx));
    }
    let empty = folders.is_empty() && entries.is_empty();
    let n = entries.len();
    let note = match app.page {
        Page::Folder if empty => "This folder is empty".to_string(),
        _ if empty && !app.search.read(cx).text().trim().is_empty() => "No documents match the search".to_string(),
        Page::Starred if empty => "Star a document with ☆ on its row".to_string(),
        Page::Recent if empty => "Documents you open appear here".to_string(),
        _ if empty => "No documents yet — start something new above".to_string(),
        _ => format!("{n} document{}", if n == 1 { "" } else { "s" }),
    };
    div()
        .flex()
        .flex_col()
        .gap(px(14.0))
        .child(
            div()
                .flex()
                .flex_col()
                .rounded(px(12.0))
                .bg(pal.surface)
                .border_1()
                .border_color(pal.border)
                .overflow_hidden()
                .child(header)
                .when(!empty, |d| d.child(body))
                .when(empty, |d| {
                    d.child(div().h(px(90.0)).flex().items_center().justify_center().text_color(pal.text_faint).child(note.clone()))
                }),
        )
        .when(!empty, |d| d.child(div().text_size(px(13.5)).text_color(pal.text_faint).child(note)))
}

fn sort_head(
    app: &KadeeApp,
    label: &'static str,
    key: SortKey,
    w: Option<f32>,
    pal: &Palette,
    cx: &mut Context<KadeeApp>,
) -> impl IntoElement {
    let on = app.sort == key;
    let hover = pal.text_muted;
    let mut d = div()
        .id(label)
        .flex()
        .items_center()
        .gap(px(4.0))
        .cursor_pointer()
        .hover(move |s| s.text_color(hover))
        .child(label)
        .when(on, |d| d.child(icon(if app.sort_desc { Icon::CaretDown } else { Icon::CaretUp }, 13.0, pal.text_faint)))
        .on_click(cx.listener(move |this, _, _, cx| this.set_sort(key, cx)));
    d = match w {
        Some(w) => d.w(px(w)).flex_none(),
        None => d.flex_1().min_w_0(),
    };
    d
}

/// The name column: badge, name and "app · folder".
fn name_cell(lead: gpui::AnyElement, name: String, sub: String, missing: bool, pal: &Palette) -> impl IntoElement {
    div().flex_1().min_w_0().flex().items_center().gap(px(14.0)).child(lead).child(
        div()
            .flex_1()
            .flex()
            .flex_col()
            .min_w_0()
            .overflow_hidden()
            .child(div().text_size(px(15.0)).truncate().text_color(if missing { pal.text_faint } else { pal.text }).child(name))
            .child(div().text_size(px(12.5)).truncate().text_color(pal.text_faint).child(sub)),
    )
}

fn row(app: &KadeeApp, i: usize, e: &FileEntry, t: u64, narrow: bool, pal: &Palette, cx: &mut Context<KadeeApp>) -> impl IntoElement {
    let lib = &app.state.library;
    let starred = lib.is_starred(&e.path);
    let opened = lib.opened_at(&e.path).map(|o| relative_time(o, t)).unwrap_or_else(|| "—".into());
    let modified = if e.modified > 0 { relative_time(e.modified, t) } else { "—".into() };
    let parent = e.path.parent().map(file_name).unwrap_or_default();
    let mut sub = format!("{} · {parent}", e.kind.app_name());
    if !e.exists {
        sub = format!("Not found · {sub}");
    }
    let group: gpui::SharedString = format!("row-{i}").into();
    let hover = pal.surface_hover;
    let path = e.path.clone();
    let menu_kind = MenuKind::Row(path.clone());
    let mk2 = menu_kind.clone();
    let (star_on, faint) = (pal.star, pal.text_faint);

    let star = div()
        .id(("star", i))
        .size(px(26.0))
        .flex()
        .items_center()
        .justify_center()
        .rounded(px(5.0))
        .cursor_pointer()
        .when(!starred, |d| d.invisible().group_hover(group.clone(), |s| s.visible()))
        .hover({
            let b = pal.border;
            move |s| s.bg(b)
        })
        .tooltip(tooltip(if starred { "Unstar" } else { "Star" }))
        .child(icon(if starred { Icon::StarFilled } else { Icon::Star }, 15.0, if starred { star_on } else { faint }))
        .on_click(on_cmd(Command::ToggleStar(path.clone()), cx));
    let more = div()
        .id(("more", i))
        .size(px(26.0))
        .flex()
        .items_center()
        .justify_center()
        .rounded(px(5.0))
        .cursor_pointer()
        .invisible()
        .group_hover(group.clone(), |s| s.visible())
        .hover({
            let b = pal.border;
            move |s| s.bg(b)
        })
        .child(icon(Icon::More, 15.0, pal.text_muted))
        .on_mouse_down(
            MouseButton::Left,
            cx.listener(move |this, ev: &MouseDownEvent, _w, cx| {
                cx.stop_propagation();
                this.open_menu(menu_kind.clone(), ev.position, cx)
            }),
        );

    div()
        .id(("doc", i))
        .group(group)
        .h(px(56.0))
        .flex()
        .items_center()
        .px(px(8.0))
        .rounded(px(8.0))
        .cursor_pointer()
        .text_size(px(14.5))
        .text_color(pal.text_muted)
        .hover(move |s| s.bg(hover))
        .child(name_cell(doc_badge(&e.kind, 32.0, pal).into_any_element(), e.name.clone(), sub, !e.exists, pal))
        .when(!narrow, |d| d.child(div().w(px(OPENED_W)).flex_none().child(opened)))
        .child(div().w(px(MODIFIED_W)).flex_none().child(modified))
        .when(!narrow, |d| d.child(div().w(px(SIZE_W)).flex_none().child(if e.exists { human_size(e.size) } else { "—".into() })))
        .child(div().w(px(ACTIONS_W)).flex_none().flex().justify_end().gap(px(2.0)).child(star).child(more))
        .on_click(on_cmd(Command::OpenPath(path), cx))
        .on_mouse_down(
            MouseButton::Right,
            cx.listener(move |this, ev: &MouseDownEvent, _w, cx| {
                cx.stop_propagation();
                this.open_menu(mk2.clone(), ev.position, cx)
            }),
        )
}

fn folder_row(i: usize, dir: &PathBuf, narrow: bool, pal: &Palette, cx: &mut Context<KadeeApp>) -> impl IntoElement {
    let hover = pal.surface_hover;
    let lead = div()
        .size(px(32.0))
        .rounded(px(8.0))
        .flex()
        .items_center()
        .justify_center()
        .bg(pal.surface_hover)
        .child(icon(Icon::Folder, 16.0, pal.text_muted))
        .into_any_element();
    let modified = std::fs::metadata(dir)
        .and_then(|m| m.modified())
        .ok()
        .and_then(|m| m.duration_since(std::time::UNIX_EPOCH).ok())
        .map(|d| relative_time(d.as_secs(), now()))
        .unwrap_or_else(|| "—".into());
    let menu_kind = MenuKind::Folder { dir: dir.clone(), root: None };
    div()
        .id(("subdir", i))
        .h(px(56.0))
        .flex()
        .items_center()
        .px(px(8.0))
        .rounded(px(8.0))
        .cursor_pointer()
        .text_size(px(14.5))
        .text_color(pal.text_muted)
        .hover(move |s| s.bg(hover))
        .child(name_cell(lead, file_name(dir), "Folder".into(), false, pal))
        .when(!narrow, |d| d.child(div().w(px(OPENED_W)).flex_none().child("—")))
        .child(div().w(px(MODIFIED_W)).flex_none().child(modified))
        .when(!narrow, |d| d.child(div().w(px(SIZE_W)).flex_none().child("—")))
        .child(div().w(px(ACTIONS_W)).flex_none())
        .on_click(on_cmd(Command::ShowFolder(dir.clone()), cx))
        .on_mouse_down(
            MouseButton::Right,
            cx.listener(move |this, ev: &MouseDownEvent, _w, cx| {
                cx.stop_propagation();
                this.open_menu(menu_kind.clone(), ev.position, cx)
            }),
        )
}
