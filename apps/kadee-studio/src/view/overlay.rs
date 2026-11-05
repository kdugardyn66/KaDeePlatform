//! What opens on top: pop-up menus, the file and folder questions, and the toast.

use gpui::{
    AnyElement, Context, FontWeight, IntoElement, ParentElement, SharedString, Styled, Window, anchored, deferred, div, prelude::*, px,
};
use kd_commands::{Command, Page};
use kd_store::{DocKind, ModuleKind, OfficeKind};
use kd_ui::{Theme, button, danger_button, doc_badge, module_badge, primary_button};

use super::ai::model_menu_entries;
use crate::app::{Dialog, KadeeApp, MenuKind, file_name, office_label};

enum Item {
    Cmd { label: String, cmd: Command, lead: Option<Box<gpui::Div>>, danger: bool, key: Option<String> },
    Header(String),
    Model { label: String, model: Option<kd_ai::ModelRef>, selected: bool },
    Separator,
    Pick { label: String, key: String, tag: Option<&'static str>, note: String, selected: bool, picker: crate::settings_state::Picker },
}

pub fn menu(app: &mut KadeeApp, _window: &mut Window, cx: &mut Context<KadeeApp>) -> Option<AnyElement> {
    let pal = Theme::palette(cx);
    let menu = app.menu.as_ref()?;
    let at = menu.at;
    let mut items: Vec<Item> = Vec::new();
    let cmd = |label: &str, cmd: Command| Item::Cmd { label: label.into(), cmd, lead: None, danger: false, key: None };
    let mut width = 230.0;
    match &menu.kind {
        MenuKind::New => {
            width = 260.0;
            for kind in [ModuleKind::Markdown, ModuleKind::Pdf, ModuleKind::Dashboard, ModuleKind::Report] {
                let label =
                    if kind == ModuleKind::Pdf { "Open a PDF…".to_string() } else { format!("New {}", kind.short().to_lowercase()) };
                let c = Command::NewDocument(kind);
                items.push(Item::Cmd {
                    label,
                    key: app.shortcut(&c),
                    cmd: c,
                    lead: Some(Box::new(module_badge(kind, 22.0, &pal))),
                    danger: false,
                });
            }
            items.push(Item::Separator);
            for kind in OfficeKind::ALL {
                let lead = DocKind::from_path(std::path::Path::new(&format!("x.{}", kind.extension())))
                    .map(|k| Box::new(doc_badge(&k, 22.0, &pal)));
                let c = Command::NewOffice(kind);
                items.push(Item::Cmd { label: format!("New {}", office_label(kind)), key: app.shortcut(&c), cmd: c, lead, danger: false });
            }
            items.push(Item::Separator);
            items.push(Item::Cmd {
                label: "Open file…".into(),
                key: app.shortcut(&Command::OpenDialog),
                cmd: Command::OpenDialog,
                lead: None,
                danger: false,
            });
            if !app.closed.is_empty() {
                items.push(cmd("Reopen closed tab", Command::ReopenClosedTab));
            }
        }
        MenuKind::Row(path) => {
            let exists = path.exists();
            items.push(cmd("Open", Command::OpenPath(path.clone())));
            if exists {
                items.push(cmd("Show in folder", Command::RevealInFolder(path.clone())));
            }
            items.push(cmd("Copy path", Command::CopyPath(path.clone())));
            let starred = app.state.library.is_starred(path);
            items.push(cmd(if starred { "Unstar" } else { "Star" }, Command::ToggleStar(path.clone())));
            if exists {
                items.push(Item::Separator);
                items.push(cmd("Rename…", Command::RenameFile(path.clone())));
                items.push(cmd("Duplicate", Command::DuplicateFile(path.clone())));
            }
            items.push(Item::Separator);
            if app.page == Page::Starred {
                items.push(cmd("Unstar", Command::RemoveFromRecent(path.clone())));
            } else if app.state.library.opened_at(path).is_some() {
                items.push(cmd("Remove from list", Command::RemoveFromRecent(path.clone())));
            }
            if exists {
                items.push(Item::Cmd {
                    label: "Delete file…".into(),
                    cmd: Command::DeleteFile(path.clone()),
                    lead: None,
                    danger: true,
                    key: None,
                });
            }
        }
        MenuKind::Folder { dir, root } => {
            let available = dir.is_dir();
            if available {
                items.push(cmd("New subfolder", Command::NewFolder(dir.clone())));
            }
            if root.is_none() {
                items.push(cmd("Rename…", Command::RenameFolder(dir.clone())));
            }
            if available {
                items.push(cmd("Show in folder", Command::RevealInFolder(dir.clone())));
            }
            if root.is_none() {
                items.push(Item::Separator);
                items.push(Item::Cmd {
                    label: "Delete folder…".into(),
                    cmd: Command::DeleteFolder(dir.clone()),
                    lead: None,
                    danger: true,
                    key: None,
                });
            }
            if *root == Some(true) {
                items.push(Item::Separator);
                items.push(cmd("Remove from list", Command::UnpinFolder(dir.clone())));
            }
        }
        MenuKind::Pick(picker) => {
            width = match picker {
                crate::settings_state::Picker::ShortcutAction => 360.0,
                _ => 460.0,
            };
            for row in app.picker_rows(picker) {
                if row.header {
                    items.push(Item::Header(row.label));
                } else {
                    items.push(Item::Pick {
                        label: row.label,
                        key: row.key,
                        tag: row.tag,
                        note: row.note,
                        selected: row.selected,
                        picker: picker.clone(),
                    });
                }
            }
        }
        MenuKind::Model => {
            width = 320.0;
            let key = app.chat_key();
            let picked = app.chat(&key).model.clone();
            for (header, label, model) in model_menu_entries(app) {
                match header {
                    Some(h) => items.push(Item::Header(h)),
                    None => {
                        let selected = model == picked;
                        items.push(Item::Model { label, model, selected })
                    }
                }
            }
            if items.len() == 1 {
                items.push(Item::Header("No provider is ready — add one in Settings".into()));
            }
        }
    }

    let hover = pal.surface_hover;
    let mut list = div().id("menu-list").flex().flex_col().max_h(px(420.0)).overflow_y_scroll();
    for (i, item) in items.into_iter().enumerate() {
        list = list.child(match item {
            Item::Separator => div().h(px(1.0)).my(px(5.0)).bg(pal.border).into_any_element(),
            Item::Header(t) => {
                div().px(px(10.0)).pt(px(8.0)).pb(px(3.0)).text_size(px(11.5)).text_color(pal.text_faint).child(t).into_any_element()
            }
            Item::Cmd { label, cmd, lead, danger, key } => div()
                .id(("menu", i))
                .h(px(34.0))
                .px(px(10.0))
                .flex()
                .items_center()
                .gap(px(10.0))
                .rounded(px(6.0))
                .cursor_pointer()
                .text_size(px(14.0))
                .text_color(if danger { pal.danger } else { pal.text })
                .hover(move |s| s.bg(hover))
                .children(lead.map(|b| *b))
                .child(div().flex_1().child(label))
                .when_some(key, |d, k| d.child(div().text_size(px(12.0)).text_color(pal.text_faint).child(k)))
                .on_click(cx.listener(move |this, _, window, cx| {
                    this.close_menu(cx);
                    this.run(cmd.clone(), window, cx)
                }))
                .into_any_element(),
            Item::Pick { label, key, tag, note, selected, picker } => div()
                .id(("pick", i))
                .h(px(32.0))
                .px(px(10.0))
                .flex()
                .items_center()
                .gap(px(8.0))
                .rounded(px(6.0))
                .cursor_pointer()
                .text_size(px(13.5))
                .when(selected, |d| d.bg(pal.accent_soft).text_color(pal.accent))
                .when(!selected, |d| d.hover(move |s| s.bg(hover)))
                .child(div().min_w_0().truncate().child(label))
                .when_some(tag, |d, t| {
                    d.child(div().px(px(7.0)).rounded(px(9.0)).bg(pal.accent_soft).text_color(pal.accent).text_size(px(11.0)).child(t))
                })
                .child(div().flex_1())
                .when(!note.is_empty(), |d| d.child(div().text_size(px(12.0)).text_color(pal.text_faint).child(note)))
                .on_click(cx.listener(move |this, _, _, cx| this.pick(&picker, &key, cx)))
                .into_any_element(),
            Item::Model { label, model, selected } => div()
                .id(("model", i))
                .h(px(32.0))
                .px(px(10.0))
                .flex()
                .items_center()
                .rounded(px(6.0))
                .cursor_pointer()
                .text_size(px(13.5))
                .when(selected, |d| d.bg(pal.accent_soft).text_color(pal.accent))
                .when(!selected, |d| d.hover(move |s| s.bg(hover)))
                .child(label)
                .on_click(cx.listener(move |this, _, _, cx| this.pick_model(model.clone(), cx)))
                .into_any_element(),
        });
    }
    let card = div()
        .w(px(width))
        .p(px(6.0))
        .rounded(px(10.0))
        .bg(pal.surface)
        .border_1()
        .border_color(pal.border)
        .shadow_lg()
        .occlude()
        .on_mouse_down_out(cx.listener(|this, _, _, cx| this.close_menu(cx)))
        .child(list);
    Some(deferred(anchored().position(at).snap_to_window_with_margin(px(8.0)).child(card)).with_priority(2).into_any_element())
}

pub fn dialog(app: &mut KadeeApp, _window: &mut Window, cx: &mut Context<KadeeApp>) -> Option<AnyElement> {
    let pal = Theme::palette(cx);
    let d = app.dialog.as_ref()?;
    let (title, body, action, danger, asks_name): (String, String, &str, bool, bool) = match d {
        Dialog::NewFolder { parent } => ("New folder".into(), format!("In “{}”", file_name(parent)), "Create", false, true),
        Dialog::RenameFolder { dir } => ("Rename folder".into(), dir.display().to_string(), "Rename", false, true),
        Dialog::RenameFile { path } => ("Rename".into(), path.display().to_string(), "Rename", false, true),
        Dialog::DeleteFolder { dir } => (
            "Delete folder".into(),
            format!("“{}” and everything inside it will be moved to the Trash.", file_name(dir)),
            "Delete",
            true,
            false,
        ),
        Dialog::TrashFile { path } => ("Delete file".into(), format!("Move “{}” to the Trash?", file_name(path)), "Delete", true, false),
        Dialog::Missing { path } => (
            "File unavailable".into(),
            format!(
                "“{}” can’t be found. It may be on a disconnected drive, or it was moved or deleted. Remove it from the list?",
                file_name(path)
            ),
            "Remove from list",
            true,
            false,
        ),
        Dialog::SaveTemplate { kind } => (
            "Save as template".into(),
            format!("A {} (.{}) in your templates folder. It shows on the New page.", kind.label().to_lowercase(), kind.extension()),
            "Save template",
            false,
            true,
        ),
        Dialog::Unsaved { path } => (
            "Unsaved changes".into(),
            format!(
                "“{}” has changes that are not saved. Close it anyway and lose them? To keep them, choose Cancel and save ({}).",
                file_name(path),
                if cfg!(target_os = "macos") { "⌘S" } else { "Ctrl+S" }
            ),
            "Close without saving",
            true,
            false,
        ),
    };
    let input = asks_name.then(|| {
        div()
            .h(px(38.0))
            .px(px(10.0))
            .flex()
            .items_center()
            .rounded(px(7.0))
            .bg(pal.canvas)
            .border_1()
            .border_color(pal.accent)
            .child(app.dialog_input.clone())
    });
    let confirm = if danger { danger_button("dlg-ok", action, &pal) } else { primary_button("dlg-ok", action, &pal) }
        .on_click(cx.listener(|this, _, window, cx| this.confirm_dialog(window, cx)));
    let card = div()
        .w(px(460.0))
        .p(px(22.0))
        .flex()
        .flex_col()
        .gap(px(12.0))
        .rounded(px(14.0))
        .bg(pal.surface)
        .border_1()
        .border_color(pal.border)
        .shadow_lg()
        .child(div().text_size(px(19.0)).font_weight(FontWeight::MEDIUM).child(title))
        .child(div().text_size(px(13.5)).text_color(pal.text_muted).child(body))
        .children(input)
        .child(
            div()
                .flex()
                .justify_end()
                .gap(px(10.0))
                .mt(px(6.0))
                .child(button("dlg-cancel", "Cancel", &pal).on_click(cx.listener(|this, _, window, cx| this.close_dialog(window, cx))))
                .child(confirm),
        );
    Some(
        deferred(div().id("scrim").absolute().inset_0().flex().items_center().justify_center().bg(pal.shadow()).occlude().child(card))
            .with_priority(3)
            .into_any_element(),
    )
}

pub fn toast(app: &KadeeApp, cx: &mut Context<KadeeApp>) -> Option<AnyElement> {
    let pal = Theme::palette(cx);
    let (msg, _) = app.toast.as_ref()?;
    let msg: SharedString = msg.clone();
    Some(
        deferred(
            div()
                .absolute()
                .bottom(px(if app.settings.is_some() { 108.0 } else { 28.0 }))
                .left_0()
                .right_0()
                .flex()
                .justify_center()
                .child(
                    div()
                        .px(px(16.0))
                        .py(px(10.0))
                        .rounded(px(9.0))
                        .bg(pal.text)
                        .text_color(pal.surface)
                        .text_size(px(14.0))
                        .shadow_lg()
                        .child(msg),
                ),
        )
        .with_priority(4)
        .into_any_element(),
    )
}
