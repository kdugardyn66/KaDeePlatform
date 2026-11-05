//! The New document page, the Crafting Apps page and the file tab.

use std::path::Path;

use gpui::{Context, FontWeight, IntoElement, ParentElement, Styled, div, prelude::*, px};
use kd_commands::Command;
use kd_store::{CRAFT_APPS, DocKind, ModuleKind, OfficeKind, human_size, now, relative_time};
use kd_ui::{Icon, Palette, Theme, button, doc_badge, icon, module_badge, primary_button};

use super::home::craft_icon;
use super::{column, content_column, on_cmd, page_title, section_title};
use crate::app::{KadeeApp, file_name, office_label};

pub fn new_page(app: &mut KadeeApp, cx: &mut Context<KadeeApp>) -> impl IntoElement {
    let pal = Theme::palette(cx);
    let dir = app.state.settings.documents_dir.clone();
    let mut kadee = div().grid().grid_cols(2).gap(px(12.0));
    for kind in [ModuleKind::Markdown, ModuleKind::Dashboard, ModuleKind::Report, ModuleKind::Pdf] {
        let cmd = Command::NewDocument(kind);
        let key = app.shortcut(&cmd);
        let label = if kind == ModuleKind::Pdf { "Open a PDF…".to_string() } else { format!("New {}", kind.short().to_lowercase()) };
        kadee = kadee.child(
            new_row(("new", kind as usize), module_badge(kind, 38.0, &pal), label, kind.tagline().to_string(), key, &pal)
                .on_click(on_cmd(cmd, cx)),
        );
    }
    let mut office = div().grid().grid_cols(2).gap(px(12.0));
    for (i, kind) in OfficeKind::ALL.into_iter().enumerate() {
        let cmd = Command::NewOffice(kind);
        let key = app.shortcut(&cmd);
        let badge = match DocKind::from_path(Path::new(&format!("x.{}", kind.extension()))) {
            Some(dk) => doc_badge(&dk, 38.0, &pal),
            None => kd_ui::badge("DOC", 38.0, pal.accent, &pal),
        };
        let sub = format!(".{} · opens in {}", kind.extension(), kind.app());
        office =
            office.child(new_row(("office", i), badge, format!("New {}", office_label(kind)), sub, key, &pal).on_click(on_cmd(cmd, cx)));
    }
    let templates = templates_section(app, &pal, cx);
    content_column("new-scroll", &pal).child(
        column()
            .child(page_title("New document", format!("New documents go into “{}”", file_name(&dir)), &pal))
            .child(section_title("KadeeStudio", None, &pal))
            .child(div().h(px(12.0)))
            .child(kadee)
            .child(div().h(px(30.0)))
            .child(templates)
            .child(div().h(px(30.0)))
            .child(section_title("Office documents", Some("blank files for WordCraft, GridCraft and DeckCraft".into()), &pal))
            .child(div().h(px(12.0)))
            .child(office)
            .child(div().h(px(30.0)))
            .child(div().flex().gap(px(10.0)).child(button("new-open", "Open file…", &pal).on_click(on_cmd(Command::OpenDialog, cx)))),
    )
}

fn new_row(
    id: impl Into<gpui::ElementId>,
    lead: gpui::Div,
    title: String,
    sub: String,
    key: Option<String>,
    pal: &Palette,
) -> gpui::Stateful<gpui::Div> {
    let (hover_bg, hover_border) = (pal.surface_hover, pal.border_strong);
    div()
        .id(id)
        .h(px(70.0))
        .px(px(16.0))
        .flex()
        .items_center()
        .gap(px(14.0))
        .rounded(px(10.0))
        .bg(pal.surface)
        .border_1()
        .border_color(pal.border)
        .cursor_pointer()
        .hover(move |s| s.bg(hover_bg).border_color(hover_border))
        .child(lead)
        .child(
            div()
                .flex_1()
                .flex()
                .flex_col()
                .min_w_0()
                .child(div().text_size(px(15.0)).child(title))
                .child(div().text_size(px(13.0)).text_color(pal.text_muted).child(sub)),
        )
        .when_some(key, |d, k| {
            d.child(
                div()
                    .px(px(7.0))
                    .py(px(2.0))
                    .rounded(px(5.0))
                    .bg(pal.surface_hover)
                    .text_size(px(12.0))
                    .text_color(pal.text_muted)
                    .child(k),
            )
        })
}

pub fn craft_apps(app: &mut KadeeApp, window: &mut gpui::Window, cx: &mut Context<KadeeApp>) -> impl IntoElement {
    let pal = Theme::palette(cx);
    let n = app.installed.len();
    let width = super::home::content_width(app, window);
    let cols: u16 = if f32::from(width) > 900.0 { 3 } else { 2 };
    let mut grid = div().grid().grid_cols(cols).gap(px(16.0)).w_full();
    // room for text in a card, in characters (GPUI clips; the ellipsis is added here, as in 0.3.1)
    let text_w = (f32::from(width) - 16.0 * (cols as f32 - 1.0)) / cols as f32 - 18.0 * 2.0 - 56.0 - 16.0;
    let fit = |px_per_char: f32| ((text_w / px_per_char).floor() as usize).max(8);
    for (i, a) in CRAFT_APPS.iter().enumerate() {
        let found = app.installed_app(a.id).cloned();
        let (chip, fg, bg) = match &found {
            Some(f) => (
                f.version.as_deref().map(|v| format!("Installed · {v}")).unwrap_or_else(|| "Installed".into()),
                pal.accent,
                pal.accent_soft,
            ),
            None => ("Not installed".to_string(), pal.text_faint, pal.surface_hover),
        };
        let exts = a.extensions.iter().map(|e| format!(".{e}")).collect::<Vec<_>>().join(" ");
        let line = if exts.is_empty() { a.maker.to_string() } else { format!("{} · {exts}", a.maker) };
        let line = kd_ui::truncate(&line, fit(6.6));
        let purpose = kd_ui::truncate(a.purpose, fit(7.6));
        let (hover_bg, accent) = (pal.surface_hover, pal.accent);
        let tip = match &found {
            Some(f) => format!("Open {} · {} · {}", a.name, f.source, f.location()),
            None => format!("{} is not installed. Get it with the ArtCraft Launcher.", a.name),
        };
        let mut card = div()
            .id(("app-card", i))
            .relative()
            .h(px(112.0))
            .px(px(18.0))
            .flex()
            .items_center()
            .gap(px(16.0))
            .rounded(px(12.0))
            .bg(pal.surface)
            .border_1()
            .border_color(pal.border)
            .tooltip(kd_ui::tooltip(tip))
            .child(div().when(found.is_none(), |d| d.opacity(0.45)).child(craft_icon(a.id, a.badge, 56.0, &pal)))
            .child(
                div()
                    .flex_1()
                    .min_w_0()
                    .flex()
                    .flex_col()
                    .gap(px(3.0))
                    .child(div().pr(px(130.0)).text_size(px(17.0)).truncate().child(a.name))
                    .child(div().text_size(px(14.0)).text_color(pal.text_muted).truncate().child(purpose))
                    .child(div().mt(px(5.0)).text_size(px(12.5)).text_color(pal.text_faint).truncate().child(line)),
            )
            .child(
                div()
                    .absolute()
                    .top(px(14.0))
                    .right(px(14.0))
                    .px(px(9.0))
                    .py(px(2.0))
                    .rounded(px(11.0))
                    .bg(bg)
                    .text_color(fg)
                    .text_size(px(12.5))
                    .child(chip),
            );
        if found.is_some() {
            card = card
                .cursor_pointer()
                .hover(move |s| s.bg(hover_bg).border_color(accent))
                .on_click(on_cmd(Command::LaunchCraft(a.id.to_string()), cx));
        }
        grid = grid.child(card);
    }
    let link = |id: &'static str, label: &'static str, pal: &Palette| {
        let hover = pal.text;
        div().id(id).text_color(pal.accent).cursor_pointer().hover(move |s| s.text_color(hover)).child(label)
    };
    content_column("craft-scroll", &pal).child(
        column()
            .child(page_title(
                "Apps",
                format!(
                    "Open-source Rust apps from the ArtCraft team, plus SolidCraft from Tangency Labs and MdCraft. {n} of {} are installed on this computer; click one to open it. Their files already appear under All; KadeeStudio embeds them in Phase {}.",
                    CRAFT_APPS.len(),
                    kd_store::CRAFT_PHASE
                ),
                &pal,
            ))
            .child(grid)
            .child(
                div()
                    .mt(px(24.0))
                    .flex()
                    .items_center()
                    .gap(px(8.0))
                    .text_size(px(15.0))
                    .child(div().text_color(pal.text_muted).child("Install or update them with the"))
                    .child(link("craft-launcher", "ArtCraft Launcher", &pal).on_click(on_cmd(Command::OpenUrl(kd_store::LAUNCHER_REPO.into()), cx)))
                    .child(div().text_color(pal.text_faint).child("·"))
                    .child(link("craft-again", "Look again", &pal).on_click(on_cmd(Command::Refresh, cx))),
            ),
    )
}

/// A document's tab: its details, the app that edits it and a preview of text files, until the
/// editors arrive.
pub fn file_tab(app: &mut KadeeApp, path: &Path, cx: &mut Context<KadeeApp>) -> impl IntoElement {
    let pal = Theme::palette(cx);
    let Some(kind) = DocKind::from_path(path) else { return div().into_any_element() };
    let meta = std::fs::metadata(path).ok();
    let size = meta.as_ref().map(|m| human_size(m.len())).unwrap_or_else(|| "—".into());
    let modified = meta
        .as_ref()
        .and_then(|m| m.modified().ok())
        .and_then(|m| m.duration_since(std::time::UNIX_EPOCH).ok())
        .map(|d| relative_time(d.as_secs(), now()))
        .unwrap_or_else(|| "—".into());
    let starred = app.state.library.is_starred(path);
    let craft = app.craft_for(&kind).cloned();
    let p = path.to_path_buf();

    let mut actions = div().flex().flex_wrap().gap(px(10.0));
    if let Some(c) = &craft {
        let name = c.app.name;
        let c2 = c.clone();
        let p2 = p.clone();
        actions =
            actions.child(primary_button("ft-craft", format!("Open in {name}"), &pal).on_click(cx.listener(move |this, _, _, cx| {
                match c2.launch(Some(&p2)) {
                    Ok(()) => this.notify(format!("Opening in {name}…"), cx),
                    Err(e) => this.notify(format!("Could not open {name}: {e}"), cx),
                }
            })));
    }
    actions = actions
        .child(button("ft-default", "Open in default app", &pal).on_click(on_cmd(Command::OpenExternally(p.clone()), cx)))
        .child(button("ft-reveal", "Show in folder", &pal).on_click(on_cmd(Command::RevealInFolder(p.clone()), cx)))
        .child(button("ft-star", if starred { "Unstar" } else { "Star" }, &pal).on_click(on_cmd(Command::ToggleStar(p.clone()), cx)))
        .child(button("ft-ai", "Ask KaDee AI", &pal).on_click(cx.listener(|this, _, window, cx| {
            this.ai_pane = true;
            this.composer.read(cx).focus(window);
            cx.notify();
        })));

    let editor_note = format!("The {} editor arrives in Phase {} of the KaDee Platform.", kind.app_name(), kind.phase());
    let info = |label: &'static str, value: String| {
        div()
            .flex()
            .gap(px(12.0))
            .child(div().w(px(90.0)).text_color(pal.text_faint).child(label))
            .child(div().flex_1().min_w_0().overflow_hidden().child(value))
    };
    let details = div()
        .flex()
        .flex_col()
        .gap(px(8.0))
        .p(px(18.0))
        .rounded(px(12.0))
        .bg(pal.surface)
        .border_1()
        .border_color(pal.border)
        .text_size(px(14.0))
        .child(info("Kind", format!("{} · {}", kind.group.label(), kind.app_name())))
        .child(info("Size", size))
        .child(info("Modified", modified))
        .child(info("Location", path.parent().map(|d| d.display().to_string()).unwrap_or_default()));

    let preview = text_preview(path).map(|text| {
        div().flex().flex_col().gap(px(10.0)).child(section_title("Preview", None, &pal)).child(
            div()
                .p(px(18.0))
                .rounded(px(12.0))
                .bg(pal.surface)
                .border_1()
                .border_color(pal.border)
                .font_family("JetBrains Mono, Menlo, DejaVu Sans Mono, monospace")
                .text_size(px(13.0))
                .line_height(px(20.0))
                .text_color(pal.text_muted)
                .children(text.lines().take(400).map(|l| div().child(if l.is_empty() { " ".to_string() } else { l.to_string() }))),
        )
    });

    content_column("file-scroll", &pal)
        .child(
            column()
                .gap(px(22.0))
                .child(
                    div().flex().items_center().gap(px(16.0)).child(doc_badge(&kind, 56.0, &pal)).child(
                        div()
                            .flex()
                            .flex_col()
                            .gap(px(4.0))
                            .min_w_0()
                            .child(div().text_size(px(26.0)).font_weight(FontWeight::NORMAL).child(file_name(path)))
                            .child(
                                div()
                                    .flex()
                                    .items_center()
                                    .gap(px(6.0))
                                    .text_size(px(13.5))
                                    .text_color(pal.text_muted)
                                    .child(icon(Icon::Info, 14.0, pal.text_faint))
                                    .child(editor_note),
                            ),
                    ),
                )
                .child(actions)
                .child(details)
                .children(preview),
        )
        .into_any_element()
}

/// The text of a small text document (Markdown, CSV, JSON …), for the preview.
fn text_preview(path: &Path) -> Option<String> {
    let ext = path.extension()?.to_str()?.to_ascii_lowercase();
    let texty = matches!(ext.as_str(), "md" | "markdown" | "mdd" | "mds" | "txt" | "csv" | "json" | "xml" | "kdash" | "kreport" | "svg");
    if !texty || std::fs::metadata(path).ok()?.len() > 512 * 1024 {
        return None;
    }
    std::fs::read_to_string(path).ok()
}

/// Markdown templates: the built-in documents and slide decks, and the user's own (`.mdd`, `.mds`).
fn templates_section(app: &mut KadeeApp, pal: &Palette, cx: &mut Context<KadeeApp>) -> impl IntoElement {
    use kd_formats::md::{library, templates};
    let card = |i: usize, t: &'static templates::Template, cx: &mut Context<KadeeApp>| {
        let colors = library::theme(t.theme).map(|th| th.colors).unwrap_or([0x1f2328, 0x2f5dd6, 0xf2f2f2]);
        let (hover_bg, hover_border) = (pal.surface_hover, pal.border_strong);
        let deck = t.kind == templates::Kind::Presentation;
        let preview = div()
            .h(px(64.0))
            .rounded(px(6.0))
            .border_1()
            .border_color(pal.border)
            .bg(if deck { gpui::rgb(colors[0]) } else { gpui::rgb(0xffffff) })
            .px(px(10.0))
            .py(px(8.0))
            .flex()
            .flex_col()
            .justify_between()
            .child(
                div()
                    .text_size(px(13.0))
                    .font_weight(FontWeight::BOLD)
                    .text_color(gpui::rgb(if deck { colors[2] } else { colors[0] }))
                    .child(kd_ui::truncate(t.name, 26)),
            )
            .child(div().flex().gap(px(3.0)).children(colors.map(|c| div().flex_1().h(px(5.0)).rounded(px(1.0)).bg(gpui::rgb(c)))));
        div()
            .id(("tpl", i))
            .p(px(10.0))
            .flex()
            .flex_col()
            .gap(px(8.0))
            .rounded(px(10.0))
            .bg(pal.surface)
            .border_1()
            .border_color(pal.border)
            .cursor_pointer()
            .hover(move |s| s.bg(hover_bg).border_color(hover_border))
            .child(preview)
            .child(div().text_size(px(13.5)).font_weight(FontWeight::MEDIUM).child(t.name))
            .child(div().text_size(px(12.0)).text_color(pal.text_muted).line_height(px(16.0)).child(t.description))
            .on_click(cx.listener(move |this, _, window, cx| this.new_from_template(t.name, t.source, window, cx)))
    };
    let mut docs = div().grid().grid_cols(4).gap(px(12.0));
    let mut decks = div().grid().grid_cols(4).gap(px(12.0));
    for (i, t) in templates::BUILT_IN.iter().enumerate() {
        if t.kind == templates::Kind::Document {
            docs = docs.child(card(i, t, cx));
        } else {
            decks = decks.child(card(i, t, cx));
        }
    }
    let mine = app.user_templates();
    let mut my_rows = div().flex().flex_col().gap(px(8.0));
    if mine.is_empty() {
        my_rows = my_rows.child(
            div()
                .text_size(px(13.0))
                .text_color(pal.text_muted)
                .child("None yet. Make one below, or save any Markdown document with File › Save as template in Kadee Markdown."),
        );
    }
    for (i, t) in mine.into_iter().enumerate() {
        let (path_use, path_edit) = (t.path.clone(), t.path.clone());
        let name = t.name.clone();
        let ext = t.kind.extension();
        my_rows = my_rows.child(
            div()
                .id(("my-tpl", i))
                .h(px(48.0))
                .px(px(14.0))
                .flex()
                .items_center()
                .gap(px(12.0))
                .rounded(px(8.0))
                .bg(pal.surface)
                .border_1()
                .border_color(pal.border)
                .child(kd_ui::badge(ext.to_uppercase(), 30.0, pal.accent, pal))
                .child(
                    div()
                        .flex_1()
                        .flex()
                        .flex_col()
                        .child(div().text_size(px(13.5)).child(t.name.clone()))
                        .child(div().text_size(px(11.5)).text_color(pal.text_faint).child(t.kind.label())),
                )
                .child(primary_button(("my-tpl-use", i), "Use", pal).on_click(cx.listener(move |this, _, window, cx| {
                    match std::fs::read_to_string(&path_use) {
                        Ok(src) => this.new_from_template(&name, &src, window, cx),
                        Err(e) => this.notify(format!("Could not read the template: {e}"), cx),
                    }
                })))
                .child(
                    button(("my-tpl-edit", i), "Edit", pal)
                        .on_click(cx.listener(move |this, _, window, cx| this.open_path(path_edit.clone(), window, cx))),
                ),
        );
    }
    let dir = app.my_templates_dir();
    div()
        .flex()
        .flex_col()
        .child(section_title("Markdown documents", Some("from a template".into()), pal))
        .child(div().h(px(12.0)))
        .child(docs)
        .child(div().h(px(26.0)))
        .child(section_title("Markdown slide decks", Some("from a template".into()), pal))
        .child(div().h(px(12.0)))
        .child(decks)
        .child(div().h(px(26.0)))
        .child(section_title("My templates", Some(".mdd documents and .mds slide decks".into()), pal))
        .child(div().h(px(12.0)))
        .child(my_rows)
        .child(div().h(px(12.0)))
        .child(
            div()
                .flex()
                .gap(px(10.0))
                .child(
                    button("tpl-new-doc", "New document template", pal).on_click(
                        cx.listener(|this, _, window, cx| this.new_template(kd_formats::md::templates::Kind::Document, window, cx)),
                    ),
                )
                .child(button("tpl-new-deck", "New slide deck template", pal).on_click(
                    cx.listener(|this, _, window, cx| this.new_template(kd_formats::md::templates::Kind::Presentation, window, cx)),
                ))
                .child(button("tpl-folder", "Show templates folder", pal).on_click(cx.listener(move |this, _, _, cx| {
                    let _ = std::fs::create_dir_all(&dir);
                    cx.reveal_path(&dir);
                    let _ = this;
                }))),
        )
}
