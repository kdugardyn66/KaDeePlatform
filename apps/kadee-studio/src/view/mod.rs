//! What the window draws. Views read [`KadeeApp`] and send [`Command`]s; they never change
//! state themselves.

mod ai;
mod home;
mod overlay;
mod pages;
mod settings;
mod shell;
mod table;

use gpui::{App, ClickEvent, Context, FontWeight, IntoElement, ParentElement, Render, SharedString, Styled, Window, div, prelude::*, px};
use kd_commands::{Command, Page};
use kd_ui::Theme;

use crate::app::KadeeApp;
use crate::keys;

/// The widest the middle column gets.
pub const CONTENT_MAX: f32 = 1240.0;

impl Render for KadeeApp {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let pal = Theme::palette(cx);
        let home = self.active == 0;
        let center = if home {
            if self.settings.is_some() {
                settings::page(self, window, cx).into_any_element()
            } else {
                match self.page {
                    Page::New => pages::new_page(self, cx).into_any_element(),
                    Page::CraftApps => pages::craft_apps(self, window, cx).into_any_element(),
                    _ => home::page(self, window, cx).into_any_element(),
                }
            }
        } else {
            match self.tabs.get(self.active - 1).cloned() {
                Some(path) => match self.md.get(&path) {
                    Some(editor) => editor.clone().into_any_element(),
                    None => pages::file_tab(self, &path, cx).into_any_element(),
                },
                None => div().into_any_element(),
            }
        };

        div()
            .id("kadee-root")
            .key_context(keys::CONTEXT)
            .track_focus(&self.focus)
            .on_action(cx.listener(Self::on_run_command))
            .on_action(cx.listener(Self::on_dismiss))
            .on_key_down(cx.listener(Self::on_key_down))
            .size_full()
            .relative()
            .flex()
            .flex_col()
            .bg(pal.canvas)
            .text_color(pal.text)
            .text_size(px(14.0))
            .font_family(ui_font())
            .child(shell::tab_strip(self, window, cx))
            .child(
                div()
                    .flex()
                    .flex_1()
                    .min_h_0()
                    .when(home, |d| d.child(shell::sidebar(self, window, cx)))
                    .child(div().flex_1().min_w_0().h_full().flex().flex_col().child(center))
                    .when(self.ai_pane, |d| d.child(ai::pane(self, window, cx))),
            )
            .children(overlay::menu(self, window, cx))
            .children(overlay::dialog(self, window, cx))
            .children(overlay::toast(self, cx))
    }
}

/// The interface font: the system's.
pub fn ui_font() -> SharedString {
    if cfg!(target_os = "macos") { ".SystemUIFont".into() } else { "Inter, Cantarell, Noto Sans, DejaVu Sans".into() }
}

/// A click handler that runs `cmd`.
pub fn on_cmd(cmd: Command, cx: &mut Context<KadeeApp>) -> impl Fn(&ClickEvent, &mut Window, &mut App) + 'static {
    cx.listener(move |this, _: &ClickEvent, window, cx| {
        cx.stop_propagation();
        this.run(cmd.clone(), window, cx)
    })
}

/// A section title with an optional muted note.
pub fn section_title(title: impl Into<SharedString>, note: Option<String>, pal: &kd_ui::Palette) -> gpui::Div {
    div()
        .flex()
        .items_baseline()
        .gap(px(10.0))
        .child(div().text_size(px(17.0)).font_weight(FontWeight::MEDIUM).text_color(pal.text).child(title.into()))
        .when_some(note, |d, n| d.child(div().text_size(px(13.5)).text_color(pal.text_faint).child(n)))
}

/// A page title and subtitle (Recent, Starred, Crafting Apps …).
pub fn page_title(title: impl Into<SharedString>, sub: impl Into<SharedString>, pal: &kd_ui::Palette) -> gpui::Div {
    div()
        .flex()
        .flex_col()
        .gap(px(4.0))
        .mb(px(22.0))
        .child(div().text_size(px(30.0)).font_weight(FontWeight::NORMAL).text_color(pal.text).child(title.into()))
        .child(div().text_size(px(14.5)).text_color(pal.text_muted).child(sub.into()))
}

/// The scrolling middle column, centred, at most [`CONTENT_MAX`] wide.
pub fn content_column(id: &'static str, pal: &kd_ui::Palette) -> gpui::Stateful<gpui::Div> {
    let _ = pal;
    div().id(id).size_full().overflow_y_scroll().flex().flex_col().items_center().px(px(32.0)).pt(px(34.0)).pb(px(48.0))
}

/// The inner column of [`content_column`].
pub fn column() -> gpui::Div {
    div().w_full().max_w(px(CONTENT_MAX)).flex().flex_col()
}
