//! Settings, over the middle pane of Home, as in KadeeStudio 0.3.1: a section list on the left,
//! the section on the right, Cancel / Done at the bottom. Sections: General, Keyboard, AI
//! providers, AI models, About and the two licence agreements.

use gpui::{
    AnyElement, Context, FontWeight, IntoElement, MouseButton, MouseDownEvent, ParentElement, Rgba, SharedString, Styled, Window, div,
    prelude::*, px, rgb,
};
use kd_ai::{KeySource, ProviderConfig, ProviderKind, Region, Scope};
use kd_commands::{Command, Os, SettingsSection, display_sequence};
use kd_store::{CRAFT_APPS, OpenIn, ThemePref};
use kd_ui::{Icon, Palette, Theme, button, icon, icon_button, logo, mix, primary_button, tooltip};

use super::on_cmd;
use crate::app::{KadeeApp, MenuKind};
use crate::settings_state::{AiScreen, MAX_CHORDS, Picker, RECORD_IDLE, TestState, kind_badge, kind_title, model_summary};

const MIT: &str = include_str!("../../../../LICENSE-MIT");
const APACHE: &str = include_str!("../../../../LICENSE-APACHE");
const MONO: &str = "JetBrains Mono, Menlo, DejaVu Sans Mono, monospace";

fn section_icon(s: SettingsSection) -> Icon {
    match s {
        SettingsSection::General => Icon::Settings,
        SettingsSection::Keyboard => Icon::Keyboard,
        SettingsSection::AiProviders | SettingsSection::AiModels => Icon::Sparkles,
        SettingsSection::About => Icon::Info,
        SettingsSection::LicenseMit | SettingsSection::LicenseApache => Icon::FilePlus,
    }
}

pub fn page(app: &mut KadeeApp, window: &mut Window, cx: &mut Context<KadeeApp>) -> impl IntoElement {
    let pal = Theme::palette(cx);
    let section = app.settings.as_ref().map(|s| s.section).unwrap_or(SettingsSection::General);

    let mut nav =
        div().w(px(250.0)).flex_none().flex().flex_col().gap(px(4.0)).px(px(14.0)).py(px(18.0)).border_r_1().border_color(pal.border);
    for (i, s) in SettingsSection::ALL.into_iter().enumerate() {
        let on = s == section;
        let hover = pal.surface_hover;
        nav = nav.child(
            div()
                .id(("settings-nav", i))
                .h(px(44.0))
                .px(px(12.0))
                .flex()
                .items_center()
                .gap(px(14.0))
                .rounded(px(9.0))
                .cursor_pointer()
                .text_size(px(15.5))
                .when(on, |d| d.bg(pal.accent_soft))
                .when(!on, |d| d.hover(move |st| st.bg(hover)))
                .child(icon(section_icon(s), 19.0, if on { pal.accent } else { pal.text_muted }))
                .child(s.label())
                .on_click(cx.listener(move |this, _, _, cx| this.settings_section(s, cx))),
        );
    }

    let body = match section {
        SettingsSection::General => general(app, window, &pal, cx).into_any_element(),
        SettingsSection::Keyboard => keyboard(app, &pal, cx).into_any_element(),
        SettingsSection::AiProviders => providers(app, window, &pal, cx),
        SettingsSection::AiModels => models(app, &pal, cx).into_any_element(),
        SettingsSection::About => about(app, &pal, cx).into_any_element(),
        SettingsSection::LicenseMit => license("MIT License", MIT, &pal).into_any_element(),
        SettingsSection::LicenseApache => license("Apache License 2.0", APACHE, &pal).into_any_element(),
    };

    let header = div()
        .h(px(68.0))
        .flex_none()
        .flex()
        .items_center()
        .justify_between()
        .px(px(32.0))
        .border_b_1()
        .border_color(pal.border)
        .child(div().text_size(px(21.0)).child("Settings"))
        .child(
            icon_button("settings-x", Icon::X, 30.0, &pal)
                .tooltip(tooltip("Close without keeping the changes (Esc)"))
                .on_click(cx.listener(|this, _, _, cx| this.close_settings(false, cx))),
        );
    let footer = div()
        .h(px(72.0))
        .flex_none()
        .flex()
        .items_center()
        .justify_between()
        .px(px(32.0))
        .border_t_1()
        .border_color(pal.border)
        .child(div().text_size(px(13.0)).text_color(pal.text_faint).child("Changes, including AI settings, are saved when you click Done."))
        .child(
            div()
                .flex()
                .gap(px(12.0))
                .child(
                    button("settings-cancel", "Cancel", &pal)
                        .h(px(38.0))
                        .px(px(18.0))
                        .on_click(cx.listener(|this, _, _, cx| this.close_settings(false, cx))),
                )
                .child(
                    primary_button("settings-done", "Done", &pal)
                        .h(px(38.0))
                        .px(px(20.0))
                        .on_click(cx.listener(|this, _, _, cx| this.close_settings(true, cx))),
                ),
        );

    div().size_full().p(px(20.0)).child(
        div()
            .size_full()
            .flex()
            .flex_col()
            .rounded(px(14.0))
            .bg(pal.surface)
            .border_1()
            .border_color(pal.border)
            .overflow_hidden()
            .child(header)
            .child(
                div().flex().flex_1().min_h_0().child(nav).child(
                    div()
                        .id(SharedString::from(format!("settings-body-{}", section.label())))
                        .flex_1()
                        .min_w_0()
                        .h_full()
                        .overflow_y_scroll()
                        .px(px(36.0))
                        .py(px(28.0))
                        .child(body),
                ),
            )
            .child(footer),
    )
}

/// A section title and its line of explanation.
fn heading(title: impl Into<SharedString>, sub: impl Into<SharedString>, pal: &Palette) -> gpui::Div {
    let sub: SharedString = sub.into();
    div()
        .flex()
        .flex_col()
        .gap(px(6.0))
        .mb(px(22.0))
        .child(div().text_size(px(26.0)).child(title.into()))
        .when(!sub.is_empty(), |d| d.child(div().text_size(px(14.5)).line_height(px(21.0)).text_color(pal.text_muted).child(sub)))
}

/// A labelled field: label (and hint) above the control.
fn field(label: &str, hint: &str, control: impl IntoElement, pal: &Palette) -> gpui::Div {
    div()
        .flex()
        .flex_col()
        .gap(px(6.0))
        .mb(px(18.0))
        .child(div().text_size(px(14.5)).font_weight(FontWeight::MEDIUM).child(label.to_string()))
        .child(control)
        .when(!hint.is_empty(), |d| d.child(div().text_size(px(12.5)).text_color(pal.text_faint).child(hint.to_string())))
}

fn small_label(text: impl Into<SharedString>, pal: &Palette) -> gpui::Div {
    div().text_size(px(12.0)).text_color(pal.text_faint).child(text.into())
}

fn input_box(input: gpui::AnyView, focused: bool, pal: &Palette) -> gpui::Div {
    div()
        .h(px(38.0))
        .px(px(11.0))
        .flex()
        .items_center()
        .rounded(px(7.0))
        .bg(pal.canvas)
        .border_1()
        .border_color(if focused { pal.accent } else { pal.border })
        .text_size(px(14.5))
        .child(input)
}

/// A chip: keys, a status.
fn chip(text: impl Into<SharedString>, fg: Rgba, bg: Rgba, mono: bool) -> gpui::Div {
    div()
        .px(px(10.0))
        .py(px(3.0))
        .rounded(px(6.0))
        .bg(bg)
        .text_color(fg)
        .text_size(px(13.5))
        .when(mono, |d| d.font_family(MONO))
        .child(text.into())
}

/// Segmented buttons; `disabled` options are shown but cannot be picked.
fn segmented<T: Copy + PartialEq + 'static>(
    id: &'static str,
    value: T,
    options: &[(T, &'static str, bool)],
    pal: &Palette,
    cx: &mut Context<KadeeApp>,
    on: impl Fn(&mut KadeeApp, T, &mut Context<KadeeApp>) + Clone + 'static,
) -> gpui::Div {
    let mut row = div().flex().p(px(3.0)).gap(px(2.0)).rounded(px(9.0)).bg(pal.canvas);
    for (i, (v, label, enabled)) in options.iter().copied().enumerate() {
        let selected = v == value;
        let hover = pal.surface_hover;
        let on = on.clone();
        row = row.child(
            div()
                .id((id, i))
                .h(px(32.0))
                .px(px(16.0))
                .flex()
                .items_center()
                .rounded(px(7.0))
                .text_size(px(14.0))
                .when(selected, |d| d.bg(pal.surface).border_1().border_color(pal.border))
                .when(!enabled, |d| d.text_color(pal.text_faint))
                .when(enabled && !selected, |d| d.cursor_pointer().text_color(pal.text_muted).hover(move |s| s.bg(hover)))
                .child(label)
                .when(enabled, |d| d.on_click(cx.listener(move |this, _, _, cx| on(this, v, cx)))),
        );
    }
    row
}

/// A button that opens a Settings picker.
fn picker_button(
    id: impl Into<gpui::ElementId>,
    label: impl Into<SharedString>,
    width: f32,
    which: Picker,
    pal: &Palette,
    cx: &mut Context<KadeeApp>,
) -> impl IntoElement {
    let hover = pal.surface_hover;
    div()
        .id(id)
        .w(px(width))
        .h(px(38.0))
        .px(px(12.0))
        .flex()
        .items_center()
        .gap(px(8.0))
        .rounded(px(7.0))
        .bg(pal.canvas)
        .border_1()
        .border_color(pal.border)
        .cursor_pointer()
        .hover(move |s| s.bg(hover))
        .child(div().flex_1().min_w_0().truncate().text_size(px(14.0)).child(label.into()))
        .child(icon(Icon::CaretDown, 14.0, pal.text_muted))
        .on_mouse_down(
            MouseButton::Left,
            cx.listener(move |this, e: &MouseDownEvent, _w, cx| {
                cx.stop_propagation();
                this.open_menu(MenuKind::Pick(which.clone()), e.position, cx)
            }),
        )
}

// ---------------------------------------------------------------------------------- General

fn general(app: &mut KadeeApp, window: &Window, pal: &Palette, cx: &mut Context<KadeeApp>) -> impl IntoElement {
    let set = &app.state.settings;
    let themes = segmented(
        "theme",
        set.theme,
        &[(ThemePref::System, "System", true), (ThemePref::Light, "Light", true), (ThemePref::Dark, "Dark", true)],
        pal,
        cx,
        |this, t, cx| {
            this.state.settings.theme = t;
            Theme::apply(t, cx);
            cx.notify();
        },
    );
    let name_focused = app.name_input.read(cx).is_focused(window);
    let docs = app.state.settings.documents_dir.clone();
    let docs_name = docs.file_name().map(|n| n.to_string_lossy().into_owned()).unwrap_or_default();
    let open_in = |id: &'static str, value: OpenIn, modules: bool, cx: &mut Context<KadeeApp>| {
        segmented(id, value, &[(OpenIn::Tab, "Tab", true), (OpenIn::Window, "Window", false)], pal, cx, move |this, v, cx| {
            if modules {
                this.state.settings.open_modules_in = v;
            } else {
                this.state.settings.open_files_in = v;
            }
            cx.notify();
        })
    };
    let (mods_in, files_in) = (set.open_modules_in, set.open_files_in);
    div()
        .max_w(px(680.0))
        .flex()
        .flex_col()
        .child(heading("General", "", pal))
        .child(field("Theme", "", div().flex().child(themes), pal))
        .child(field(
            "Name in the greeting",
            "Shown on the start screen: “Good evening, Kris”.",
            input_box(app.name_input.clone().into(), name_focused, pal),
            pal,
        ))
        .child(field(
            "Save new documents in",
            &format!("The first folder under Folders in the sidebar, shown as “{docs_name}”."),
            div()
                .flex()
                .gap(px(10.0))
                .child(
                    div()
                        .flex_1()
                        .min_w_0()
                        .h(px(38.0))
                        .px(px(11.0))
                        .flex()
                        .items_center()
                        .rounded(px(7.0))
                        .bg(pal.canvas)
                        .border_1()
                        .border_color(pal.border)
                        .text_color(pal.text_muted)
                        .child(div().truncate().child(tilde(&docs))),
                )
                .child(
                    button("docs-change", "Change…", pal).h(px(38.0)).on_click(cx.listener(|this, _, _, cx| this.prompt_documents_dir(cx))),
                ),
            pal,
        ))
        .child(field(
            "Open each module in a new",
            "Kadee Markdown, Kadee PDF …: when the module is open already, KadeeStudio switches to it. Windows come in a later version.",
            div().flex().child(open_in("open-mod", mods_in, true, cx)),
            pal,
        ))
        .child(field(
            "Open each file in a new",
            "When the file is open already, KadeeStudio switches to its tab.",
            div().flex().child(open_in("open-file", files_in, false, cx)),
            pal,
        ))
        .child(field(
            "Team server",
            "Real-time collaboration through your own server on Docker — Phase 3.",
            div().flex().child(button("team", "Connect…", pal).text_color(pal.text_faint).cursor_default()),
            pal,
        ))
        .child(div().text_size(px(12.5)).text_color(pal.text_faint).child(format!("Settings file: {}", tilde(&app.store.file()))))
        .child(
            div()
                .flex()
                .gap(px(4.0))
                .text_size(px(12.5))
                .text_color(pal.text_faint)
                .child(format!("KadeeStudio {} ·", kd_store::VERSION))
                .child(
                    div()
                        .id("general-about")
                        .text_color(pal.accent)
                        .cursor_pointer()
                        .child("About")
                        .on_click(cx.listener(|this, _, _, cx| this.settings_section(SettingsSection::About, cx))),
                ),
        )
}

fn tilde(p: &std::path::Path) -> String {
    let s = p.display().to_string();
    match std::env::var("HOME") {
        Ok(h) if !h.is_empty() && s.starts_with(&h) => format!("~{}", &s[h.len()..]),
        _ => s,
    }
}

// ---------------------------------------------------------------------------------- Keyboard

fn keyboard(app: &mut KadeeApp, pal: &Palette, cx: &mut Context<KadeeApp>) -> impl IntoElement {
    let os = Os::current();
    let rec = app.settings.as_ref().and_then(|s| s.recording.as_ref());
    let recording_index = rec.map(|r| r.index);
    let rec_text =
        rec.map(|r| if r.chords.is_empty() { "Press keys…".to_string() } else { format!("{} …", display_sequence(&r.chords, os)) });
    let busy = recording_index.is_some();

    let note = match os {
        Os::Mac => "Cmd and Ctrl are different keys; Cmd is the main shortcut key.",
        Os::Windows => "Ctrl is the main shortcut key. Windows keeps most Win+letter combinations for itself.",
        Os::Linux => "Ctrl is the main shortcut key. Super is the key with the Windows logo on most PC keyboards.",
    };
    let mut legend = div()
        .flex()
        .flex_wrap()
        .items_center()
        .gap(px(10.0))
        .mb(px(18.0))
        .child(div().text_size(px(14.5)).text_color(pal.text_muted).child(format!("Modifier keys on {}:", os.name())));
    for name in os.modifier_names() {
        legend = legend.child(chip(name, pal.text, pal.surface_hover, true).border_1().border_color(pal.border));
    }
    legend = legend.child(div().text_size(px(13.0)).text_color(pal.text_faint).child(note));

    let action_button = |id: (&'static str, usize), label: &'static str, cx: &mut Context<KadeeApp>| {
        let b = button(id, label, pal);
        let _ = &cx;
        if busy { b.text_color(pal.text_faint).cursor_default() } else { b.text_size(px(14.0)) }
    };
    let mut rows = div().flex().flex_col().child(
        div()
            .flex()
            .items_center()
            .h(px(36.0))
            .child(div().w(px(330.0)).flex_none().child(small_label("ACTION", pal)))
            .child(div().flex_1().child(small_label("SHORTCUT", pal))),
    );
    for (i, b) in app.keymap.bindings.iter().enumerate() {
        let shortcut: AnyElement = if recording_index == Some(i) {
            chip(rec_text.clone().unwrap_or_default(), pal.accent, pal.accent_soft, true)
                .border_1()
                .border_color(pal.accent)
                .into_any_element()
        } else if b.keys.is_empty() {
            div().text_size(px(14.0)).italic().text_color(pal.text_faint).child("Not assigned").into_any_element()
        } else {
            let conflict = app.keymap.conflict(i).map(|j| format!("Also used by “{}”", app.keymap.bindings[j].command.label()));
            div()
                .flex()
                .flex_col()
                .gap(px(3.0))
                .child(
                    div()
                        .flex()
                        .child(chip(display_sequence(&b.keys, os), pal.text, pal.surface_hover, true).border_1().border_color(pal.border)),
                )
                .when_some(conflict, |d, c| d.child(div().text_size(px(12.0)).text_color(pal.danger).child(c)))
                .into_any_element()
        };
        let record_label = if b.keys.is_empty() { "Record" } else { "Change" };
        let mut buttons = div().flex().gap(px(10.0)).child(
            action_button(("sc-rec", i), record_label, cx)
                .when(!busy, |d| d.on_click(cx.listener(move |this, _, window, cx| this.start_recording(i, false, window, cx)))),
        );
        if !b.keys.is_empty() {
            buttons = buttons.child(
                action_button(("sc-clear", i), "Clear", cx)
                    .when(!busy, |d| d.on_click(cx.listener(move |this, _, _, cx| this.clear_shortcut(i, cx)))),
            );
        }
        buttons = buttons.child(
            action_button(("sc-remove", i), "Remove", cx)
                .tooltip(tooltip("Remove this shortcut from the list"))
                .when(!busy, |d| d.on_click(cx.listener(move |this, _, _, cx| this.remove_shortcut(i, cx)))),
        );
        rows = rows.child(
            div()
                .flex()
                .items_center()
                .min_h(px(54.0))
                .py(px(6.0))
                .child(div().w(px(330.0)).flex_none().text_size(px(15.0)).child(b.command.label()))
                .child(div().flex_1().min_w_0().child(shortcut))
                .child(buttons),
        );
    }

    let choice = app.settings.as_ref().map(|s| s.add_choice).unwrap_or(0);
    let choice_label = kd_commands::bindable_commands().get(choice).map(|(_, l)| l.clone()).unwrap_or_default();
    div()
        .flex()
        .flex_col()
        .child(heading(
            "Keyboard shortcuts",
            format!(
                "Click Record and press the keys. For a sequence, press them one after the other, such as {}+N then M. Enter saves at once, Esc cancels, Backspace starts over; after {:.1} s without a key the shortcut is saved.",
                os.primary_name(),
                RECORD_IDLE.as_secs_f32()
            ),
            pal,
        ))
        .child(legend)
        .child(div().px(px(20.0)).py(px(12.0)).rounded(px(12.0)).bg(pal.canvas).border_1().border_color(pal.border).child(rows))
        .child(div().mt(px(26.0)).text_size(px(18.0)).child("Add a shortcut"))
        .child(div().mt(px(6.0)).text_size(px(14.5)).text_color(pal.text_muted).child(format!(
            "Pick any action, then record its keys (up to {MAX_CHORDS} in a sequence). An action can have several shortcuts."
        )))
        .child(
            div()
                .mt(px(12.0))
                .flex()
                .items_center()
                .gap(px(12.0))
                .child(picker_button("add-action", choice_label, 360.0, Picker::ShortcutAction, pal, cx))
                .child(
                    button("add-record", "Add and record", pal)
                        .h(px(38.0))
                        .when(!busy, |d| d.on_click(cx.listener(|this, _, window, cx| this.add_shortcut(window, cx)))),
                ),
        )
        .child(
            div()
                .mt(px(30.0))
                .flex()
                .items_center()
                .gap(px(14.0))
                .child(button("sc-reset", "Reset to defaults", pal).h(px(38.0)).on_click(cx.listener(|this, _, _, cx| this.reset_shortcuts(cx))))
                .child(div().text_size(px(13.0)).text_color(pal.text_faint).child("Your shortcuts are saved with your settings on this computer.")),
        )
}

// ---------------------------------------------------------------------------------- AI providers

/// A provider's coloured square with its initials.
fn kind_square(kind: ProviderKind, size: f32) -> gpui::Div {
    let (hex, initials) = kind_badge(kind);
    div()
        .size(px(size))
        .flex_none()
        .flex()
        .items_center()
        .justify_center()
        .rounded(px(size * 0.24))
        .bg(rgb(hex))
        .text_color(gpui::white())
        .text_size(px(size * 0.40))
        .child(initials)
}

/// "Ready", "Needs an API key", "Key from $ANTHROPIC_API_KEY", "Local · no key needed", "Off";
/// 1 good, 2 bad, 0 muted.
fn status(p: &ProviderConfig) -> (String, u8) {
    if !p.enabled {
        return ("Off".into(), 0);
    }
    if p.base_url().is_empty() {
        return ("Needs an endpoint".into(), 2);
    }
    match p.key_source() {
        KeySource::None if p.kind.needs_key() => ("Needs an API key".into(), 2),
        KeySource::Env(v) => (format!("Key from ${v}"), 1),
        KeySource::Stored => ("Ready".into(), 1),
        KeySource::None if p.kind.is_local() => ("Local · no key needed".into(), 1),
        KeySource::None => ("Ready".into(), 1),
    }
}

fn status_chip(p: &ProviderConfig, pal: &Palette) -> gpui::Div {
    let (text, tone) = status(p);
    let fg = match tone {
        1 => pal.success,
        2 => pal.danger,
        _ => pal.text_faint,
    };
    chip(text, fg, mix(pal.surface, fg, 0.14), false).rounded(px(11.0)).text_size(px(12.5))
}

fn back_link(id: &'static str, label: &'static str, screen: AiScreen, pal: &Palette, cx: &mut Context<KadeeApp>) -> impl IntoElement {
    let hover = pal.text;
    div()
        .id(id)
        .mb(px(14.0))
        .text_size(px(14.0))
        .text_color(pal.accent)
        .cursor_pointer()
        .hover(move |s| s.text_color(hover))
        .child(format!("‹  {label}"))
        .on_click(cx.listener(move |this, _, _, cx| this.ai_screen(screen.clone(), cx)))
}

fn provider_row(
    id: impl Into<gpui::ElementId>,
    kind: ProviderKind,
    title: String,
    sub: String,
    right: Option<gpui::Div>,
    height: f32,
    pal: &Palette,
) -> gpui::Stateful<gpui::Div> {
    let (hover_bg, hover_border) = (pal.surface_hover, pal.accent);
    div()
        .id(id)
        .h(px(height))
        .px(px(14.0))
        .mb(px(8.0))
        .flex()
        .items_center()
        .gap(px(14.0))
        .rounded(px(10.0))
        .bg(pal.surface)
        .border_1()
        .border_color(pal.border)
        .cursor_pointer()
        .hover(move |s| s.bg(hover_bg).border_color(hover_border))
        .child(kind_square(kind, if height > 56.0 { 38.0 } else { 32.0 }))
        .child(
            div()
                .flex_1()
                .min_w_0()
                .flex()
                .flex_col()
                .gap(px(2.0))
                .child(div().text_size(px(15.5)).truncate().child(title))
                .child(div().text_size(px(13.5)).text_color(pal.text_muted).truncate().child(sub)),
        )
        .children(right)
}

fn providers(app: &mut KadeeApp, window: &Window, pal: &Palette, cx: &mut Context<KadeeApp>) -> AnyElement {
    let Some(screen) = app.settings.as_ref().map(|s| s.screen.clone()) else { return div().into_any_element() };
    let col = div().max_w(px(860.0)).flex().flex_col();
    match screen {
        AiScreen::List => {
            let ai = app.settings.as_ref().map(|s| s.ai.clone()).unwrap_or_default();
            let mut col = col.child(
                div()
                    .flex()
                    .items_start()
                    .justify_between()
                    .gap(px(20.0))
                    .child(heading("AI providers", "Connect the services and local servers KaDee AI can use.", pal))
                    .child(
                        primary_button("add-provider", "Add provider", pal)
                            .h(px(38.0))
                            .on_click(cx.listener(|this, _, _, cx| this.ai_screen(AiScreen::AddKind, cx))),
                    ),
            );
            if ai.providers.is_empty() {
                col = col.child(
                    div().text_size(px(14.0)).text_color(pal.text_muted).child("No providers yet. Click Add provider to connect one."),
                );
            }
            for region in Region::ALL {
                let rows: Vec<&ProviderConfig> = ai.providers.iter().filter(|p| p.region() == region).collect();
                if rows.is_empty() {
                    continue;
                }
                col = col.child(small_label(region.label().to_uppercase(), pal).mt(px(10.0)).mb(px(8.0)));
                for p in rows {
                    let sub = match p.kind {
                        ProviderKind::Custom | ProviderKind::OpenAiCompatible => p.base_url(),
                        k => k.blurb().to_string(),
                    };
                    let id = p.id.clone();
                    col = col.child(
                        provider_row(
                            SharedString::from(format!("prov-{}", p.id)),
                            p.kind,
                            p.display_name(),
                            sub,
                            Some(status_chip(p, pal)),
                            66.0,
                            pal,
                        )
                        .on_click(cx.listener(move |this, _, _, cx| this.ai_screen(AiScreen::Edit(id.clone()), cx))),
                    );
                }
            }
            col.into_any_element()
        }
        AiScreen::AddKind => {
            let mut col = col.child(back_link("back-list", "AI providers", AiScreen::List, pal, cx)).child(heading(
                "Add a provider",
                "Choose the service, then enter its key or endpoint.",
                pal,
            ));
            for region in Region::ALL {
                col = col.child(small_label(region.label().to_uppercase(), pal).mt(px(10.0)).mb(px(8.0)));
                for kind in ProviderKind::in_region(region) {
                    col = col.child(
                        provider_row(
                            SharedString::from(format!("kind-{kind:?}")),
                            kind,
                            kind_title(kind).to_string(),
                            kind.blurb().to_string(),
                            None,
                            58.0,
                            pal,
                        )
                        .on_click(cx.listener(move |this, _, _, cx| this.choose_kind(kind, cx))),
                    );
                }
            }
            col.into_any_element()
        }
        AiScreen::AddForm(p) => col
            .child(back_link("back-kind", "Choose another provider", AiScreen::AddKind, pal, cx))
            .child(heading(format!("Add {}", p.kind.label()), "", pal))
            .child(form(app, &p, window, pal, cx))
            .child(
                div()
                    .flex()
                    .gap(px(12.0))
                    .mt(px(6.0))
                    .child(
                        primary_button("form-add", "Add provider", pal)
                            .h(px(38.0))
                            .on_click(cx.listener(|this, _, _, cx| this.add_form_provider(cx))),
                    )
                    .child(
                        button("form-cancel", "Cancel", pal)
                            .h(px(38.0))
                            .on_click(cx.listener(|this, _, _, cx| this.ai_screen(AiScreen::List, cx))),
                    ),
            )
            .into_any_element(),
        AiScreen::Edit(id) => {
            let Some(p) = app.settings.as_ref().and_then(|s| s.ai.provider(&id)).cloned() else {
                return col.child(back_link("back-list", "AI providers", AiScreen::List, pal, cx)).into_any_element();
            };
            col.child(back_link("back-list", "AI providers", AiScreen::List, pal, cx))
                .child(heading(p.display_name(), "", pal))
                .child(form(app, &p, window, pal, cx))
                .child(
                    div().flex().mt(px(6.0)).child(
                        kd_ui::danger_outline_button("prov-remove", "Remove provider", pal)
                            .h(px(38.0))
                            .on_click(cx.listener(move |this, _, _, cx| this.remove_provider(&id, cx))),
                    ),
                )
                .into_any_element()
        }
    }
}

/// The fields of one provider, with Test connection.
fn form(app: &mut KadeeApp, p: &ProviderConfig, window: &Window, pal: &Palette, cx: &mut Context<KadeeApp>) -> impl IntoElement {
    let focused = |i: &gpui::Entity<kd_ui::TextInput>, cx: &Context<KadeeApp>| i.read(cx).is_focused(window);
    let mut col = div()
        .flex()
        .flex_col()
        .child(
            div().flex().items_center().gap(px(12.0)).mb(px(12.0)).child(kind_square(p.kind, 38.0)).child(
                div()
                    .flex()
                    .flex_col()
                    .gap(px(4.0))
                    .child(div().text_size(px(15.0)).font_weight(FontWeight::MEDIUM).child(p.kind.vendor()))
                    .child(div().flex().child(status_chip(p, pal))),
            ),
        )
        .child(div().mb(px(16.0)).text_size(px(13.5)).line_height(px(20.0)).text_color(pal.text_muted).child(p.kind.help()));

    // Use this provider
    let on = p.enabled;
    col = col.child(
        div()
            .id("prov-enabled")
            .mb(px(18.0))
            .flex()
            .items_center()
            .gap(px(10.0))
            .cursor_pointer()
            .child(
                div()
                    .size(px(18.0))
                    .rounded(px(4.0))
                    .border_1()
                    .border_color(if on { pal.accent } else { pal.border_strong })
                    .when(on, |d| d.bg(pal.accent))
                    .flex()
                    .items_center()
                    .justify_center()
                    .when(on, |d| d.child(icon(Icon::Check, 13.0, pal.on_accent))),
            )
            .child(div().text_size(px(14.5)).child("Use this provider"))
            .on_click(cx.listener(|this, _, _, cx| this.update_form(|p| p.enabled = !p.enabled, cx))),
    );

    if p.kind == ProviderKind::Custom {
        col = col.child(field("Name", "", input_box(app.prov_name_input.clone().into(), focused(&app.prov_name_input, cx), pal), pal));
    }
    if p.kind.endpoint_editable() {
        col = col.child(field("Endpoint", "", input_box(app.url_input.clone().into(), focused(&app.url_input, cx), pal), pal));
    }
    if p.kind == ProviderKind::Custom {
        col = col
            .child(field("API", "", picker_button("pick-protocol", p.protocol().label(), 300.0, Picker::Protocol, pal, cx), pal))
            .child(field("Region", "", picker_button("pick-region", p.region().label(), 300.0, Picker::Region, pal, cx), pal));
    }
    if p.kind.needs_key() || matches!(p.kind, ProviderKind::OpenAiCompatible | ProviderKind::Custom) {
        let visible = app.key_visible;
        let eye = icon_button("key-eye", if visible { Icon::EyeOff } else { Icon::Eye }, 32.0, pal)
            .tooltip(tooltip(if visible { "Hide the key" } else { "Show the key" }))
            .on_click(cx.listener(|this, _, _, cx| {
                this.key_visible = !this.key_visible;
                let masked = !this.key_visible;
                this.key_input.update(cx, |i, cx| i.set_masked(masked, cx));
            }));
        let note = match p.key_source() {
            KeySource::Stored => "Stored in ai.toml in your settings folder (plain text).".to_string(),
            KeySource::Env(v) => format!("Using ${v} from the environment."),
            KeySource::None => p.kind.key_env().first().map(|v| format!("Or set ${v} in the environment.")).unwrap_or_default(),
        };
        col = col.child(field(
            "API key",
            &note,
            div()
                .flex()
                .items_center()
                .gap(px(6.0))
                .child(input_box(app.key_input.clone().into(), focused(&app.key_input, cx), pal).flex_1())
                .child(eye),
            pal,
        ));
    }
    if p.service() == kd_ai::Service::Chat {
        let current = match p.default_model.as_deref() {
            Some(m) => app.hub.catalog().info(p.kind, m).name,
            None => "Automatic".into(),
        };
        col = col.child(field("Default model", "", picker_button("pick-prov-model", current, 380.0, Picker::ProviderModel, pal, cx), pal));
        if p.kind.lists_own_models() {
            col = col.child(field("Models", "", input_box(app.models_input.clone().into(), focused(&app.models_input, cx), pal), pal));
        }
    }

    // Test connection
    let (test, note): (Option<(String, Rgba)>, Option<String>) = match app.settings.as_ref().and_then(|s| s.tests.get(&p.id)) {
        Some(TestState::Running(_)) => (Some(("Testing…".into(), pal.text_muted)), None),
        Some(TestState::Ok(list, note)) => {
            (Some((format!("Connected · {} model{}", list.len(), if list.len() == 1 { "" } else { "s" }), pal.success)), note.clone())
        }
        Some(TestState::Err(e)) => (Some((e.clone(), pal.danger)), None),
        None => (None, None),
    };
    col.child(
        div()
            .flex()
            .items_center()
            .gap(px(12.0))
            .mb(px(8.0))
            .child(
                button("prov-test", "Test connection", pal).h(px(38.0)).on_click(cx.listener(|this, _, _, cx| this.test_form_provider(cx))),
            )
            .when_some(test, |d, (t, c)| d.child(div().flex_1().min_w_0().text_size(px(13.5)).text_color(c).child(t))),
    )
    .when_some(note, |d, n| d.child(div().text_size(px(12.5)).text_color(pal.text_muted).child(n)))
    .child(div().h(px(14.0)))
}

// ---------------------------------------------------------------------------------- AI models

fn models(app: &mut KadeeApp, pal: &Palette, cx: &mut Context<KadeeApp>) -> impl IntoElement {
    let Some(ai) = app.settings.as_ref().map(|s| s.ai.clone()) else { return div() };
    let summary = |m: Option<&kd_ai::ModelRef>, none: &str| m.map(|m| model_summary(&app.hub, &ai, m)).unwrap_or_else(|| none.to_string());
    let default_label = summary(ai.default_model.as_ref(), "Automatic (first ready provider)");
    let mut col = div()
        .max_w(px(760.0))
        .flex()
        .flex_col()
        .child(heading("AI models", "Which model KaDee AI uses, in general and for each part of KadeeStudio.", pal))
        .child(field(
            "Default model",
            "Used wherever a part of KadeeStudio has no model of its own.",
            picker_button("pick-default", default_label, 460.0, Picker::DefaultModel, pal, cx),
            pal,
        ))
        .child(small_label("PER PART OF KADEESTUDIO", pal).mt(px(8.0)).mb(px(8.0)));
    for scope in Scope::ALL {
        let label = summary(ai.models.get(scope.id()), "Use the default");
        col = col.child(
            div().flex().items_center().mb(px(8.0)).child(div().w(px(200.0)).flex_none().text_size(px(14.5)).child(scope.label())).child(
                picker_button(
                    SharedString::from(format!("pick-scope-{}", scope.id())),
                    label,
                    460.0,
                    Picker::ScopeModel(scope.id().to_string()),
                    pal,
                    cx,
                ),
            ),
        );
    }
    let added: Vec<(String, String, String)> =
        ai.providers.iter().flat_map(|p| p.models.iter().map(move |m| (p.id.clone(), p.display_name(), m.clone()))).collect();
    col = col.child(small_label("MODELS YOU ADDED", pal).mt(px(18.0)).mb(px(8.0)));
    if added.is_empty() {
        col = col.child(
            div()
                .text_size(px(13.5))
                .text_color(pal.text_muted)
                .child("None yet. List a provider's own models under AI providers › Models."),
        );
    }
    for (i, (pid, pname, m)) in added.into_iter().enumerate() {
        col = col.child(
            div()
                .flex()
                .items_center()
                .gap(px(10.0))
                .h(px(38.0))
                .child(div().text_size(px(14.5)).child(m.clone()))
                .child(div().flex_1().text_size(px(12.5)).text_color(pal.text_faint).child(pname))
                .child(button(("model-remove", i), "Remove", pal).on_click(cx.listener(move |this, _, _, cx| {
                    if let Some(s) = this.settings.as_mut()
                        && let Some(p) = s.ai.provider_mut(&pid)
                    {
                        p.models.retain(|x| *x != m);
                        if p.default_model.as_deref() == Some(m.as_str()) {
                            p.default_model = None;
                        }
                    }
                    cx.notify();
                }))),
        );
    }
    col
}

// ---------------------------------------------------------------------------------- About

fn about(app: &KadeeApp, pal: &Palette, cx: &mut Context<KadeeApp>) -> impl IntoElement {
    let os = format!("{} {}", std::env::consts::OS, std::env::consts::ARCH);
    let info = |l: &'static str, v: String| {
        div()
            .flex()
            .gap(px(12.0))
            .child(div().w(px(130.0)).flex_none().text_color(pal.text_faint).child(l))
            .child(div().flex_1().min_w_0().child(v))
    };
    let installed =
        if app.installed.is_empty() { "None".to_string() } else { app.installed.iter().map(|a| a.app.name).collect::<Vec<_>>().join(", ") };
    div()
        .max_w(px(760.0))
        .flex()
        .flex_col()
        .gap(px(18.0))
        .child(
            div()
                .flex()
                .items_center()
                .gap(px(16.0))
                .child(logo(64.0, pal))
                .child(
                    div()
                        .flex_1()
                        .min_w_0()
                        .flex()
                        .flex_col()
                        .gap(px(3.0))
                        .child(div().text_size(px(24.0)).child("KadeeStudio"))
                        .child(div().text_size(px(14.0)).text_color(pal.text_muted).child(format!("Version {} · KaDee Platform, standalone · Rust and GPUI", kd_store::VERSION))),
                ),
        )
        .child(
            div()
                .flex()
                .flex_col()
                .gap(px(6.0))
                .text_size(px(14.0))
                .child(info("Build", format!("{} · {}", env!("KADEE_COMMIT"), env!("KADEE_BUILD_DATE"))))
                .child(info("System", os))
                .child(info("Renderer", if cfg!(target_os = "macos") { "GPUI · Metal".into() } else if cfg!(windows) { "GPUI · DirectX".into() } else { "GPUI · Vulkan".into() }))
                .child(info("Settings", tilde(app.store.dir())))
                .child(info("Crafting Apps", format!("{installed} ({} known)", CRAFT_APPS.len())))
                .child(info("Licence", "MIT OR Apache-2.0".into())),
        )
        .child(div().text_size(px(13.0)).text_color(pal.text_muted).child(
            "The start screen is ported from KadeeStudio 0.3.1; KaDee AI from its kadee-ai layer (after SolarOffice's). Icons in the style of Lucide (ISC). Crafting App icons by their makers.",
        ))
        .child(
            div()
                .flex()
                .flex_wrap()
                .gap(px(8.0))
                .child(button("about-src", "Source code", pal).on_click(on_cmd(Command::OpenUrl(kd_store::SOURCE_REPO.into()), cx)))
                .child(button("about-studio", "KadeeStudio 0.3", pal).on_click(on_cmd(Command::OpenUrl(kd_store::STUDIO_REPO.into()), cx)))
                .child(button("about-mit", "MIT", pal).on_click(cx.listener(|this, _, _, cx| this.settings_section(SettingsSection::LicenseMit, cx))))
                .child(button("about-apache", "Apache-2.0", pal).on_click(cx.listener(|this, _, _, cx| this.settings_section(SettingsSection::LicenseApache, cx)))),
        )
}

fn license(title: &'static str, text: &'static str, pal: &Palette) -> impl IntoElement {
    div().flex().flex_col().child(heading(title, "", pal)).child(
        div()
            .font_family(MONO)
            .text_size(px(12.5))
            .line_height(px(18.0))
            .text_color(pal.text_muted)
            .children(text.lines().map(|l| div().child(if l.is_empty() { " ".to_string() } else { l.to_string() }))),
    )
}
