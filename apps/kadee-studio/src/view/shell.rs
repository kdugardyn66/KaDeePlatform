//! The tab strip and the sidebar.

use std::path::{Path, PathBuf};

use gpui::{AnyElement, Context, FontWeight, IntoElement, MouseButton, MouseDownEvent, ParentElement, Styled, Window, div, prelude::*, px};
use kd_commands::{Command, Page, SettingsSection};
use kd_store::{CRAFT_APPS, DocKind, ThemePref};
use kd_ui::{Icon, Palette, Theme, doc_badge, icon, icon_button, logo, tooltip, truncate};

use super::on_cmd;
use crate::app::{KadeeApp, MenuKind, file_name};

pub const SIDEBAR_W: f32 = 268.0;

pub fn tab_strip(app: &mut KadeeApp, _window: &mut Window, cx: &mut Context<KadeeApp>) -> impl IntoElement {
    let pal = Theme::palette(cx);
    let mut tabs = div().flex().items_end().gap(px(4.0)).h_full().min_w_0().overflow_hidden();
    tabs = tabs.child(tab(0, None, "Home".into(), app.active == 0, false, &pal, cx));
    for (i, path) in app.tabs.iter().enumerate() {
        let Some(kind) = DocKind::from_path(path) else { continue };
        tabs = tabs.child(tab(i + 1, Some(kind), file_name(path), app.active == i + 1, true, &pal, cx));
    }
    let new_tip = match app.shortcut(&Command::NewDocumentPicker) {
        Some(k) => format!("New document ({k})"),
        None => "New document".into(),
    };
    let plus = icon_button("tab-new", Icon::Plus, 30.0, &pal).mb(px(5.0)).tooltip(tooltip(new_tip)).on_mouse_down(
        MouseButton::Left,
        cx.listener(|this, e: &MouseDownEvent, _w, cx| {
            cx.stop_propagation();
            this.open_menu(MenuKind::New, e.position, cx)
        }),
    );

    let (theme_icon, theme_tip) = match app.state.settings.theme {
        ThemePref::System => (Icon::Contrast, "Theme: System — click for Light"),
        ThemePref::Light => (Icon::Sun, "Theme: Light — click for Dark"),
        ThemePref::Dark => (Icon::Moon, "Theme: Dark — click for System"),
    };
    let ai_tip = match app.shortcut(&Command::ToggleAiPane) {
        Some(k) => format!("KaDee AI ({k})"),
        None => "KaDee AI".into(),
    };
    let ai_on = app.ai_pane;
    let accent = pal.accent;
    div()
        .h(px(52.0))
        .flex_none()
        .flex()
        .items_end()
        .justify_between()
        .pl(px(10.0))
        .pr(px(12.0))
        .bg(pal.chrome)
        .border_b_1()
        .border_color(pal.border)
        .child(div().flex().items_end().h_full().min_w_0().gap(px(4.0)).child(tabs).child(plus))
        .child(
            div()
                .flex()
                .items_center()
                .gap(px(4.0))
                .mb(px(8.0))
                .child(icon_button("theme", theme_icon, 30.0, &pal).tooltip(tooltip(theme_tip)).on_click(on_cmd(Command::CycleTheme, cx)))
                .child(
                    icon_button("ai-toggle", Icon::Sparkles, 30.0, &pal)
                        .when(ai_on, |d| d.border_1().border_color(accent).text_color(accent))
                        .tooltip(tooltip(ai_tip))
                        .on_click(on_cmd(Command::ToggleAiPane, cx)),
                ),
        )
}

fn tab(
    n: usize,
    kind: Option<DocKind>,
    label: String,
    active: bool,
    closable: bool,
    pal: &Palette,
    cx: &mut Context<KadeeApp>,
) -> AnyElement {
    let hover = pal.surface_hover;
    let lead = match &kind {
        None => icon(Icon::Home, 16.0, if active { pal.accent } else { pal.text_muted }).into_any_element(),
        Some(k) => doc_badge(k, 18.0, pal).into_any_element(),
    };
    let mut t = div()
        .id(("tab", n))
        .group("tab")
        .h(px(42.0))
        .min_w(px(110.0))
        .max_w(px(260.0))
        .flex()
        .items_center()
        .gap(px(9.0))
        .pl(px(14.0))
        .pr(px(if closable { 8.0 } else { 18.0 }))
        .rounded_t(px(9.0))
        .cursor_pointer()
        .text_size(px(14.0))
        .child(lead)
        .child(div().flex_1().min_w_0().truncate().child(truncate(&label, 26)))
        .on_click(on_cmd(Command::ActivateTab(n), cx))
        .on_mouse_down(
            MouseButton::Middle,
            cx.listener(move |this, _: &MouseDownEvent, window, cx| {
                if n > 0 {
                    this.run(Command::CloseTab(n), window, cx)
                }
            }),
        );
    t = if active {
        t.bg(pal.canvas).text_color(pal.text).border_1().border_b_0().border_color(pal.border).mb(px(-1.0))
    } else {
        t.text_color(pal.text_muted).mb(px(3.0)).h(px(38.0)).rounded_b(px(9.0)).hover(move |s| s.bg(hover))
    };
    if closable {
        let close = div()
            .id(("tab-close", n))
            .size(px(20.0))
            .flex_none()
            .flex()
            .items_center()
            .justify_center()
            .rounded(px(5.0))
            .text_color(pal.text_muted)
            .when(!active, |d| d.invisible().group_hover("tab", |s| s.visible()))
            .hover({
                let b = pal.border;
                move |s| s.bg(b)
            })
            .child(gpui::svg().path(Icon::X.path()).size(px(12.0)).text_color(pal.text_muted))
            .on_click(on_cmd(Command::CloseTab(n), cx));
        t = t.child(close);
    }
    t.into_any_element()
}

pub fn sidebar(app: &mut KadeeApp, _window: &mut Window, cx: &mut Context<KadeeApp>) -> impl IntoElement {
    let pal = Theme::palette(cx);
    let lib = &app.state.library;
    let in_settings = app.settings.is_some();
    let on = |p: Page| !in_settings && app.page == p;
    let (recent_n, starred_n) = (lib.recent.len(), lib.starred.len());
    let folder_on_screen = on(Page::Folder);

    let brand = div().flex().items_center().gap(px(12.0)).px(px(4.0)).mb(px(22.0)).child(logo(36.0, &pal)).child(
        div()
            .flex()
            .flex_col()
            .child(div().text_size(px(17.0)).font_weight(FontWeight::MEDIUM).child("KadeeStudio"))
            .child(div().text_size(px(12.5)).text_color(pal.text_faint).child(format!("Office suite · v{}", kd_store::VERSION))),
    );

    let nav = div()
        .flex()
        .flex_col()
        .gap(px(2.0))
        .child(
            nav_item("nav-home", Icon::Home, "Home", None, on(Page::Home) || on(Page::Ai), &pal)
                .on_click(on_cmd(Command::ShowPage(Page::Home), cx)),
        )
        .child(nav_item("nav-new", Icon::FilePlus, "New", None, on(Page::New), &pal).on_click(on_cmd(Command::ShowPage(Page::New), cx)))
        .child(nav_item("nav-open", Icon::FolderOpen, "Open…", None, false, &pal).on_click(on_cmd(Command::OpenDialog, cx)))
        .child(
            nav_item("nav-recent", Icon::Clock, "Recent", Some(recent_n), on(Page::Recent), &pal)
                .on_click(on_cmd(Command::ShowPage(Page::Recent), cx)),
        )
        .child(
            nav_item("nav-starred", Icon::Star, "Starred", Some(starred_n), on(Page::Starred), &pal)
                .on_click(on_cmd(Command::ShowPage(Page::Starred), cx)),
        )
        .child(
            nav_item("nav-craft", Icon::Grid, "Apps", Some(CRAFT_APPS.len()), on(Page::CraftApps), &pal)
                .on_click(on_cmd(Command::ShowPage(Page::CraftApps), cx)),
        );

    let target = if folder_on_screen { app.folder.clone() } else { app.state.settings.documents_dir.clone() };
    let folders_head = div()
        .flex()
        .items_center()
        .justify_between()
        .px(px(6.0))
        .mt(px(22.0))
        .mb(px(6.0))
        .child(div().text_size(px(12.0)).text_color(pal.text_faint).child("FOLDERS"))
        .child(
            div()
                .flex()
                .gap(px(2.0))
                .child(
                    icon_button("folder-pin", Icon::FolderPlus, 24.0, &pal)
                        .tooltip(tooltip("Add folder…"))
                        .on_click(on_cmd(Command::PinFolder, cx)),
                )
                .child(
                    icon_button("folder-new", Icon::Plus, 24.0, &pal)
                        .tooltip(tooltip(format!("New folder in “{}”", file_name(&target))))
                        .on_click(on_cmd(Command::NewFolder(target.clone()), cx)),
                ),
        );

    let docs_dir = app.state.settings.documents_dir.clone();
    let mut tree = div().id("folder-tree").flex().flex_col().flex_1().min_h_0().overflow_y_scroll();
    for root in app.folder_roots() {
        let removable = root != docs_dir;
        let label = file_name(&root);
        tree = tree.children(tree_node(app, &root, &label, 0, Some(removable), folder_on_screen, &pal, cx));
    }

    let status = div()
        .flex()
        .flex_col()
        .gap(px(2.0))
        .p(px(12.0))
        .rounded(px(9.0))
        .bg(pal.surface)
        .border_1()
        .border_color(pal.border)
        .child(
            div()
                .flex()
                .items_center()
                .gap(px(9.0))
                .child(div().size(px(8.0)).rounded_full().bg(pal.text_faint))
                .child(div().text_size(px(14.0)).child("Working locally")),
        )
        .child(div().pl(px(17.0)).text_size(px(12.5)).text_color(pal.text_faint).child("Team server arrives in Phase 3"));

    div()
        .w(px(SIDEBAR_W))
        .flex_none()
        .h_full()
        .flex()
        .flex_col()
        .pt(px(18.0))
        .pb(px(14.0))
        .pl(px(14.0))
        .pr(px(10.0))
        .bg(pal.canvas)
        .child(brand)
        .child(nav)
        .child(folders_head)
        .child(tree)
        .child(
            div().flex().flex_col().gap(px(6.0)).pt(px(12.0)).child(status).child(
                nav_item("nav-settings", Icon::Settings, "Settings", None, in_settings, &pal)
                    .on_click(on_cmd(Command::OpenSettings(SettingsSection::General), cx)),
            ),
        )
}

/// A sidebar row.
fn nav_item(
    id: &'static str,
    which: Icon,
    label: &'static str,
    count: Option<usize>,
    selected: bool,
    pal: &Palette,
) -> gpui::Stateful<gpui::Div> {
    let hover = pal.surface_hover;
    let fg = if selected { pal.accent } else { pal.text_muted };
    div()
        .id(id)
        .h(px(40.0))
        .flex()
        .flex_none()
        .items_center()
        .gap(px(12.0))
        .px(px(12.0))
        .rounded(px(8.0))
        .cursor_pointer()
        .text_size(px(15.0))
        .when(selected, |d| d.bg(pal.accent_soft))
        .when(!selected, |d| d.hover(move |s| s.bg(hover)))
        .child(icon(which, 17.0, fg))
        .child(div().flex_1().text_color(if selected { pal.accent } else { pal.text }).child(label))
        .when_some(count, |d, n| d.child(div().text_size(px(13.0)).text_color(pal.text_faint).child(n.to_string())))
}

/// One folder of the tree and, when open, the folders inside it.
#[allow(clippy::too_many_arguments)]
fn tree_node(
    app: &KadeeApp,
    dir: &Path,
    label: &str,
    depth: usize,
    root: Option<bool>,
    on_folder_page: bool,
    pal: &Palette,
    cx: &mut Context<KadeeApp>,
) -> Vec<gpui::AnyElement> {
    let available = dir.is_dir();
    let open = available && app.is_expanded(dir);
    let children: Vec<PathBuf> = if available && open { kd_store::list_subfolders(dir) } else { Vec::new() };
    let has_children = available && (root.is_some() || !kd_store::list_subfolders(dir).is_empty());
    let selected = on_folder_page && app.folder == dir;
    let hover = pal.surface_hover;
    let id = gpui::ElementId::Name(format!("folder:{}", dir.display()).into());
    let chevron_id = gpui::ElementId::Name(format!("chevron:{}", dir.display()).into());
    let more_id = gpui::ElementId::Name(format!("more:{}", dir.display()).into());
    let group: gpui::SharedString = format!("tree:{}", dir.display()).into();
    let fg = if selected { pal.accent } else { pal.text_muted };

    let chevron = div()
        .id(chevron_id)
        .size(px(18.0))
        .flex_none()
        .flex()
        .items_center()
        .justify_center()
        .rounded(px(4.0))
        .when(has_children, |d| d.child(icon(if open { Icon::ChevronDown } else { Icon::ChevronRight }, 13.0, pal.text_faint)))
        .on_click(on_cmd(Command::ToggleFolder(dir.to_path_buf()), cx));
    let menu_kind = MenuKind::Folder { dir: dir.to_path_buf(), root };
    let mk2 = menu_kind.clone();
    let more = div()
        .id(more_id)
        .size(px(22.0))
        .flex_none()
        .flex()
        .items_center()
        .justify_center()
        .rounded(px(5.0))
        .invisible()
        .group_hover(group.clone(), |s| s.visible())
        .hover({
            let b = pal.border;
            move |s| s.bg(b)
        })
        .child(icon(Icon::More, 14.0, pal.text_muted))
        .on_mouse_down(
            MouseButton::Left,
            cx.listener(move |this, e: &MouseDownEvent, _w, cx| {
                cx.stop_propagation();
                this.open_menu(menu_kind.clone(), e.position, cx)
            }),
        );
    let row = div()
        .id(id)
        .group(group)
        .h(px(34.0))
        .flex()
        .flex_none()
        .items_center()
        .gap(px(6.0))
        .pl(px(4.0 + depth as f32 * 14.0))
        .pr(px(4.0))
        .rounded(px(7.0))
        .cursor_pointer()
        .when(selected, |d| d.bg(pal.accent_soft))
        .when(!selected, |d| d.hover(move |s| s.bg(hover)))
        .child(chevron)
        .child(icon(Icon::Folder, 15.0, fg))
        .child(
            div()
                .flex_1()
                .min_w_0()
                .truncate()
                .text_size(px(14.5))
                .text_color(if available { pal.text } else { pal.text_faint })
                .child(label.to_string()),
        )
        .when(!available, |d| d.child(div().text_size(px(11.0)).text_color(pal.text_faint).child("Not available")))
        .child(more)
        .when(available, |d| d.on_click(on_cmd(Command::ShowFolder(dir.to_path_buf()), cx)))
        .on_mouse_down(
            MouseButton::Right,
            cx.listener(move |this, e: &MouseDownEvent, _w, cx| {
                cx.stop_propagation();
                this.open_menu(mk2.clone(), e.position, cx)
            }),
        );
    let mut out = vec![row.into_any_element()];
    for child in children {
        let name = file_name(&child);
        out.extend(tree_node(app, &child, &name, depth + 1, None, on_folder_page, pal, cx));
    }
    out
}
