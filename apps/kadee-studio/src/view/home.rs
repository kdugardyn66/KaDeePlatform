//! The Home page (also Recent, Starred and a folder): greeting, quick start, Crafting Apps and
//! the documents table.

use std::path::{Path, PathBuf};

use chrono::{Datelike, Local, Timelike};
use gpui::{Context, FontWeight, IntoElement, ParentElement, Pixels, Styled, Window, div, img, prelude::*, px};
use kd_commands::{Command, Page};
use kd_store::{CRAFT_APPS, DocGroup, InstalledApp, ModuleKind};
use kd_ui::{Icon, Palette, Theme, badge, icon, icon_button, module_badge, pill, tooltip};

use super::{column, content_column, on_cmd, page_title, section_title, shell::SIDEBAR_W, table};
use crate::app::{KadeeApp, file_name};
use crate::assets::craft_image;

pub fn page(app: &mut KadeeApp, window: &mut Window, cx: &mut Context<KadeeApp>) -> impl IntoElement {
    let pal = Theme::palette(cx);
    let width = content_width(app, window);
    let col = match app.page {
        Page::Recent => {
            column().child(page_title("Recent", "Documents you opened, newest first", &pal)).child(documents(app, None, width, window, cx))
        }
        Page::Starred => {
            column().child(page_title("Starred", "Documents you marked with a star", &pal)).child(documents(app, None, width, window, cx))
        }
        Page::Folder => {
            let dir = app.folder.clone();
            column()
                .child(folder_heading(app, &dir, &pal, cx))
                .child(quick_start(app, width, &pal, cx))
                .child(div().h(px(30.0)))
                .child(documents(app, None, width, window, cx))
        }
        _ => column()
            .child(greeting(app, &pal))
            .child(quick_start(app, width, &pal, cx))
            .child(div().h(px(34.0)))
            .child(craft_strip(app, width, &pal, cx))
            .child(div().h(px(34.0)))
            .child(documents(app, Some("Documents"), width, window, cx)),
    };
    content_column("home-scroll", &pal).child(col)
}

/// The width of the middle column's content.
pub fn content_width(app: &KadeeApp, window: &Window) -> Pixels {
    let w = window.viewport_size().width
        - if app.active == 0 { px(SIDEBAR_W) } else { px(0.0) }
        - if app.ai_pane { px(super::ai::PANE_W) } else { px(0.0) }
        - px(64.0);
    w.min(px(super::CONTENT_MAX)).max(px(300.0))
}

fn greeting(app: &KadeeApp, pal: &Palette) -> impl IntoElement {
    let now = Local::now();
    let part = match now.hour() {
        5..=11 => "Good morning",
        12..=17 => "Good afternoon",
        _ => "Good evening",
    };
    let name = app.state.settings.display_name.trim();
    let title = if name.is_empty() { part.to_string() } else { format!("{part}, {name}") };
    let sub = format!("{} {} {} · Start something new or pick up where you left off.", now.format("%A"), now.day(), now.format("%B"));
    div()
        .flex()
        .flex_col()
        .gap(px(8.0))
        .mb(px(34.0))
        .child(div().text_size(px(34.0)).font_weight(FontWeight::NORMAL).text_color(pal.text).child(title))
        .child(div().text_size(px(15.5)).text_color(pal.text_muted).child(sub))
}

fn quick_start(app: &KadeeApp, width: Pixels, pal: &Palette, cx: &mut Context<KadeeApp>) -> impl IntoElement {
    let gap = 18.0;
    let n = ModuleKind::TILES.len() + 1;
    let w = f32::from(width);
    let cols = if w >= n as f32 * 170.0 + (n - 1) as f32 * gap {
        n
    } else if w >= 3.0 * 170.0 + 2.0 * gap {
        3
    } else {
        2
    };
    let mut grid = div().grid().grid_cols(cols as u16).gap(px(gap)).w_full();
    for kind in ModuleKind::TILES {
        let foot = match kind {
            ModuleKind::Pdf => "Open a PDF".to_string(),
            ModuleKind::Convert => format!("Phase {}", kind.phase()),
            _ => "New document".to_string(),
        };
        let cmd = match kind {
            ModuleKind::Markdown | ModuleKind::Dashboard | ModuleKind::Report => Command::NewDocument(kind),
            k => Command::OpenModule(k),
        };
        grid = grid.child(
            card(("tile", kind as usize), module_badge(kind, 44.0, pal), kind.short(), kind.tagline(), foot, pal).on_click(on_cmd(cmd, cx)),
        );
    }
    let open_badge = div().size(px(44.0)).rounded(px(10.0)).flex().items_center().justify_center().bg(pal.surface_hover).child(icon(
        Icon::FolderOpen,
        21.0,
        pal.text_muted,
    ));
    let open_key = app.shortcut(&Command::OpenDialog).unwrap_or_default();
    grid = grid.child(
        card(("tile", 99usize), open_badge, "Open file", "From this computer", open_key, pal).on_click(on_cmd(Command::OpenDialog, cx)),
    );
    let into = if app.page == Page::Folder { Some(format!("New documents go into “{}”", file_name(&app.folder))) } else { None };
    div().flex().flex_col().gap(px(14.0)).child(section_title("Start something new", into, pal)).child(grid)
}

/// A quick-start card.
fn card(
    id: impl Into<gpui::ElementId>,
    lead: impl IntoElement,
    title: &str,
    tagline: &str,
    foot: String,
    pal: &Palette,
) -> gpui::Stateful<gpui::Div> {
    let (hover_border, hover_bg) = (pal.border_strong, pal.surface_hover);
    let id = id.into();
    let group: gpui::SharedString = format!("card-{id:?}").into();
    div()
        .id(id)
        .group(group.clone())
        .relative()
        .h(px(152.0))
        .p(px(18.0))
        .flex()
        .flex_col()
        .rounded(px(12.0))
        .bg(pal.surface)
        .border_1()
        .border_color(pal.border)
        .cursor_pointer()
        .hover(move |s| s.border_color(hover_border).bg(hover_bg))
        .child(lead)
        .child(div().mt(px(18.0)).text_size(px(17.0)).text_color(pal.text).child(title.to_string()))
        .child(div().mt(px(4.0)).text_size(px(13.5)).line_height(px(18.0)).text_color(pal.text_muted).child(tagline.to_string()))
        .child(
            div()
                .absolute()
                .top(px(18.0))
                .right(px(16.0))
                .text_size(px(12.0))
                .text_color(pal.accent)
                .invisible()
                .group_hover(group, |s| s.visible())
                .child(foot),
        )
}

fn craft_strip(app: &KadeeApp, width: Pixels, pal: &Palette, cx: &mut Context<KadeeApp>) -> impl IntoElement {
    let n = app.installed.len();
    let sub = if n == 0 {
        "None installed yet · get them with the ArtCraft launcher".to_string()
    } else {
        format!("{n} of {} installed on this computer", CRAFT_APPS.len())
    };
    let link_hover = pal.text;
    let head = div().flex().items_center().justify_between().child(section_title("Crafting Apps", Some(sub), pal)).child(
        div()
            .id("craft-see-all")
            .text_size(px(14.5))
            .text_color(pal.accent)
            .cursor_pointer()
            .hover(move |s| s.text_color(link_hover))
            .child("See all")
            .on_click(on_cmd(Command::ShowPage(Page::CraftApps), cx)),
    );
    let body = if app.installed.is_empty() {
        div()
            .text_color(pal.text_muted)
            .text_size(px(14.0))
            .child("Installed Crafting Apps appear here with their icons; click one to open it.")
            .into_any_element()
    } else {
        // the same tiles at every width: as many per row as fit, so the rows rearrange when the
        // window narrows or the KaDee AI pane opens
        let gap = 10.0;
        let cols = ((f32::from(width) + gap) / (172.0 + gap)).floor().max(1.0) as u16;
        let mut grid = div().grid().grid_cols(cols).gap(px(gap)).w_full();
        for (i, found) in app.installed.iter().enumerate() {
            grid = grid.child(craft_tile(i, found, pal).on_click(on_cmd(Command::LaunchCraft(found.app.id.to_string()), cx)));
        }
        grid.into_any_element()
    };
    div().flex().flex_col().gap(px(14.0)).child(head).child(body)
}

/// The icon of a Crafting App (its PNG, or a badge).
pub fn craft_icon(id: &str, badge_text: &str, size: f32, pal: &Palette) -> gpui::AnyElement {
    match craft_image(id) {
        Some(src) => img(src).size(px(size)).flex_none().rounded(px(size * 0.22)).into_any_element(),
        None => badge(badge_text.to_string(), size, pal.accent, pal).into_any_element(),
    }
}

fn craft_tile(i: usize, found: &InstalledApp, pal: &Palette) -> gpui::Stateful<gpui::Div> {
    let (hover_bg, accent) = (pal.surface_hover, pal.accent);
    div()
        .id(("craft", i))
        .h(px(66.0))
        .px(px(12.0))
        .flex()
        .items_center()
        .gap(px(11.0))
        .rounded(px(9.0))
        .bg(pal.surface)
        .border_1()
        .border_color(pal.border)
        .cursor_pointer()
        .hover(move |s| s.bg(hover_bg).border_color(accent))
        .tooltip(tooltip(format!("Open {} · {} · {}", found.app.name, found.app.purpose, found.location())))
        .child(craft_icon(found.app.id, found.app.badge, 40.0, pal))
        .child(
            div()
                .flex_1()
                .flex()
                .flex_col()
                .min_w_0()
                .overflow_hidden()
                .child(div().text_size(px(15.0)).truncate().child(found.app.name))
                .child(div().text_size(px(12.5)).text_color(pal.text_faint).truncate().child(found.app.purpose)),
        )
}

/// The documents section: filter pills, search, refresh and the table.
fn documents(
    app: &mut KadeeApp,
    title: Option<&'static str>,
    width: Pixels,
    window: &Window,
    cx: &mut Context<KadeeApp>,
) -> impl IntoElement {
    let pal = Theme::palette(cx);
    let one_row = f32::from(width) >= 1180.0;
    let mut pills = div().flex().flex_wrap().items_center().gap(px(8.0));
    if let Some(t) = title {
        pills = pills.child(div().mr(px(8.0)).text_size(px(19.0)).text_color(pal.text).child(t));
    }
    pills = pills.child(pill("filter-all", "All", app.filter.is_none(), &pal).on_click(cx.listener(|this, _, _, cx| {
        this.filter = None;
        cx.notify();
    })));
    for (i, group) in DocGroup::ALL.into_iter().enumerate() {
        let selected = app.filter == Some(group);
        pills = pills.child(pill(("filter", i), group.label(), selected, &pal).on_click(cx.listener(move |this, _, _, cx| {
            this.filter = if this.filter == Some(group) { None } else { Some(group) };
            cx.notify();
        })));
    }
    let search_focused = app.search.read(cx).is_focused(window);
    let search = div()
        .flex()
        .items_center()
        .gap(px(9.0))
        .w(px(280.0))
        .h(px(38.0))
        .px(px(12.0))
        .rounded(px(8.0))
        .bg(pal.surface)
        .border_1()
        .border_color(if search_focused { pal.accent } else { pal.border })
        .child(icon(Icon::Search, 15.0, pal.text_faint))
        .child(div().flex_1().min_w_0().text_size(px(14.5)).child(app.search.clone()));
    let tools = div()
        .flex()
        .items_center()
        .gap(px(8.0))
        .child(search)
        .child(icon_button("refresh", Icon::Refresh, 32.0, &pal).tooltip(tooltip("Refresh")).on_click(on_cmd(Command::Refresh, cx)));

    let head = if one_row {
        div().flex().items_center().justify_between().gap(px(16.0)).child(pills).child(tools)
    } else {
        div().flex().flex_col().gap(px(12.0)).child(pills).child(div().flex().justify_end().child(tools))
    };

    let entries = app.visible_entries(cx);
    let folders: Vec<PathBuf> = if app.page == Page::Folder {
        let q = app.search.read(cx).text().trim().to_lowercase();
        app.folder_subdirs.iter().filter(|d| q.is_empty() || file_name(d).to_lowercase().contains(&q)).cloned().collect()
    } else {
        Vec::new()
    };
    div().flex().flex_col().gap(px(16.0)).child(head).child(table::file_table(app, &folders, &entries, width, cx))
}

/// The folder's name, its path as clickable steps from its root, and its actions.
fn folder_heading(app: &KadeeApp, dir: &Path, pal: &Palette, cx: &mut Context<KadeeApp>) -> impl IntoElement {
    let roots = app.folder_roots();
    let docs_dir = app.state.settings.documents_dir.clone();
    let root = roots.iter().filter(|r| dir.starts_with(r)).max_by_key(|r| r.components().count()).cloned();
    let mut actions = div().flex().gap(px(8.0));
    if dir.is_dir() {
        actions =
            actions.child(kd_ui::button("folder-new-here", "New folder", pal).on_click(on_cmd(Command::NewFolder(dir.to_path_buf()), cx)));
    }
    actions = actions
        .child(kd_ui::button("folder-reveal", "Show in folder", pal).on_click(on_cmd(Command::RevealInFolder(dir.to_path_buf()), cx)));
    if root.as_deref() == Some(dir) && *dir != docs_dir {
        actions = actions
            .child(kd_ui::button("folder-unpin", "Remove from list", pal).on_click(on_cmd(Command::UnpinFolder(dir.to_path_buf()), cx)));
    }
    let mut crumbs = div().flex().flex_wrap().items_center().gap(px(5.0)).text_size(px(14.0));
    match root {
        Some(root) if root.as_path() != dir => {
            let mut steps = vec![(file_name(&root), root.clone())];
            let mut acc = root.clone();
            if let Ok(rest) = dir.strip_prefix(&root) {
                for part in rest.components() {
                    acc = acc.join(part);
                    steps.push((part.as_os_str().to_string_lossy().into_owned(), acc.clone()));
                }
            }
            let n = steps.len();
            for (i, (label, path)) in steps.into_iter().enumerate() {
                if i + 1 == n {
                    crumbs = crumbs.child(div().text_color(pal.text_muted).child(label));
                } else {
                    let hover = pal.text;
                    crumbs = crumbs
                        .child(
                            div()
                                .id(("crumb", i))
                                .text_color(pal.accent)
                                .cursor_pointer()
                                .hover(move |s| s.text_color(hover))
                                .child(label)
                                .on_click(on_cmd(Command::ShowFolder(path), cx)),
                        )
                        .child(div().text_color(pal.text_faint).child("›"));
                }
            }
        }
        _ => crumbs = crumbs.child(div().text_color(pal.text_muted).child(dir.display().to_string())),
    }
    div()
        .flex()
        .flex_col()
        .gap(px(6.0))
        .mb(px(26.0))
        .child(div().flex().items_center().justify_between().child(div().text_size(px(30.0)).child(file_name(dir))).child(actions))
        .child(crumbs)
}
