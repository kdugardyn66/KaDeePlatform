//! Settings, in the middle pane: General, Keyboard, AI providers, AI models, About and the
//! licences. Changes apply and are saved at once.

use gpui::{Context, FontWeight, IntoElement, ParentElement, Styled, Window, div, prelude::*, px};
use kd_ai::{KeySource, ModelRef, ProviderConfig, Region};
use kd_commands::{Command, Os, SettingsSection, bindable_commands};
use kd_store::{CRAFT_APPS, ThemePref};
use kd_ui::{Icon, Palette, Theme, button, icon, logo, pill, primary_button};

use super::ai::{model_label, model_menu_entries};
use super::on_cmd;
use crate::app::{KadeeApp, TestState};

const MIT: &str = include_str!("../../../../LICENSE-MIT");
const APACHE: &str = include_str!("../../../../LICENSE-APACHE");

pub fn page(app: &mut KadeeApp, window: &mut Window, cx: &mut Context<KadeeApp>) -> impl IntoElement {
    let pal = Theme::palette(cx);
    let section = app.settings.as_ref().map(|s| s.section).unwrap_or(SettingsSection::General);
    let mut nav = div().w(px(210.0)).flex_none().flex().flex_col().gap(px(2.0));
    for (i, s) in SettingsSection::ALL.into_iter().enumerate() {
        let on = s == section;
        let hover = pal.surface_hover;
        nav = nav.child(
            div()
                .id(("settings-nav", i))
                .h(px(36.0))
                .px(px(12.0))
                .flex()
                .items_center()
                .rounded(px(7.0))
                .cursor_pointer()
                .text_size(px(14.5))
                .when(on, |d| d.bg(pal.accent_soft).text_color(pal.accent))
                .when(!on, |d| d.hover(move |st| st.bg(hover)))
                .child(s.label())
                .on_click(on_cmd(Command::OpenSettings(s), cx)),
        );
    }
    let body = match section {
        SettingsSection::General => general(app, window, &pal, cx).into_any_element(),
        SettingsSection::Keyboard => keyboard(app, &pal).into_any_element(),
        SettingsSection::AiProviders => providers(app, window, &pal, cx).into_any_element(),
        SettingsSection::AiModels => models(app, &pal, cx).into_any_element(),
        SettingsSection::About => about(app, &pal, cx).into_any_element(),
        SettingsSection::LicenseMit => license("MIT License", MIT, &pal).into_any_element(),
        SettingsSection::LicenseApache => license("Apache License 2.0", APACHE, &pal).into_any_element(),
    };
    div().id("settings-scroll").size_full().overflow_y_scroll().px(px(32.0)).pt(px(34.0)).pb(px(48.0)).child(
        div()
            .w_full()
            .max_w(px(1100.0))
            .mx_auto()
            .flex()
            .flex_col()
            .gap(px(22.0))
            .child(div().flex().items_center().justify_between().child(div().text_size(px(30.0)).child("Settings")).child(
                primary_button("settings-done", "Done", &pal).on_click(cx.listener(|this, _, _, cx| {
                    this.settings = None;
                    cx.notify();
                })),
            ))
            .child(div().flex().gap(px(28.0)).items_start().child(nav).child(div().flex_1().min_w_0().child(body))),
    )
}

fn card(pal: &Palette) -> gpui::Div {
    div().flex().flex_col().p(px(20.0)).gap(px(16.0)).rounded(px(12.0)).bg(pal.surface).border_1().border_color(pal.border)
}

fn field(label: &'static str, hint: Option<String>, control: impl IntoElement, pal: &Palette) -> gpui::Div {
    div()
        .flex()
        .gap(px(20.0))
        .items_start()
        .child(
            div()
                .w(px(200.0))
                .flex_none()
                .flex()
                .flex_col()
                .gap(px(3.0))
                .child(div().text_size(px(14.5)).child(label))
                .when_some(hint, |d, h| d.child(div().text_size(px(12.5)).text_color(pal.text_faint).child(h))),
        )
        .child(div().flex_1().min_w_0().child(control))
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

fn general(app: &mut KadeeApp, window: &Window, pal: &Palette, cx: &mut Context<KadeeApp>) -> impl IntoElement {
    let theme = app.state.settings.theme;
    let mut themes = div().flex().gap(px(8.0));
    for (i, t) in ThemePref::ALL.into_iter().enumerate() {
        themes = themes.child(pill(("theme-pill", i), t.label(), theme == t, pal).on_click(on_cmd(Command::SetTheme(t), cx)));
    }
    let docs = app.state.settings.documents_dir.clone();
    let name_focused = app.name_input.read(cx).is_focused(window);
    card(pal)
        .child(field(
            "Your name",
            Some("Used in the greeting".into()),
            input_box(app.name_input.clone().into(), name_focused, pal).max_w(px(360.0)),
            pal,
        ))
        .child(field("Theme", None, themes, pal))
        .child(field(
            "Documents folder",
            Some("New documents are saved here".into()),
            div()
                .flex()
                .flex_col()
                .gap(px(10.0))
                .child(div().text_size(px(14.0)).text_color(pal.text_muted).child(docs.display().to_string()))
                .child(
                    div()
                        .flex()
                        .gap(px(8.0))
                        .child(button("docs-change", "Change…", pal).on_click(cx.listener(|this, _, _, cx| this.prompt_documents_dir(cx))))
                        .child(button("docs-show", "Show in folder", pal).on_click(on_cmd(Command::RevealInFolder(docs.clone()), cx))),
                ),
            pal,
        ))
        .child(field(
            "Settings folder",
            Some("home.json, library.sqlite3 and ai.toml".into()),
            div()
                .flex()
                .flex_col()
                .gap(px(10.0))
                .child(div().text_size(px(14.0)).text_color(pal.text_muted).child(app.store.dir().display().to_string()))
                .child(div().flex().child(
                    button("cfg-show", "Show in folder", pal).on_click(on_cmd(Command::RevealInFolder(app.store.dir().to_path_buf()), cx)),
                )),
            pal,
        ))
}

fn keyboard(app: &KadeeApp, pal: &Palette) -> impl IntoElement {
    let mut list = div().flex().flex_col();
    for (i, (cmd, label)) in bindable_commands().into_iter().enumerate() {
        let keys = app.shortcut(&cmd).unwrap_or_default();
        list = list.child(
            div()
                .h(px(38.0))
                .flex()
                .items_center()
                .justify_between()
                .when(i > 0, |d| d.border_t_1().border_color(pal.border))
                .child(div().text_size(px(14.0)).child(label))
                .child(if keys.is_empty() {
                    div().text_size(px(13.0)).text_color(pal.text_faint).child("—")
                } else {
                    div()
                        .px(px(8.0))
                        .py(px(3.0))
                        .rounded(px(5.0))
                        .bg(pal.surface_hover)
                        .text_size(px(13.0))
                        .text_color(pal.text_muted)
                        .child(keys)
                }),
        );
    }
    card(pal)
        .child(div().text_size(px(13.5)).text_color(pal.text_muted).child(format!(
            "Shortcuts on {}. Press them one after the other for a sequence (such as {} then M). Editing shortcuts returns in 0.2; shortcuts changed in KadeeStudio 0.3 are used here.",
            Os::current().name(),
            app.shortcut(&Command::NewDocumentPicker).unwrap_or_else(|| "Ctrl+N".into())
        )))
        .child(list)
}

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

fn providers(app: &mut KadeeApp, window: &Window, pal: &Palette, cx: &mut Context<KadeeApp>) -> impl IntoElement {
    let config = app.ai_config();
    let expanded = app.settings.as_ref().and_then(|s| s.expanded.clone());
    let mut col = div().flex().flex_col().gap(px(18.0)).child(div().text_size(px(13.5)).text_color(pal.text_muted).child(
        "Nothing is sent anywhere until you add a provider. Keys are kept in ai.toml in your settings folder, readable by you only, or read from the provider's usual environment variable.",
    ));
    for region in Region::ALL {
        let list: Vec<&ProviderConfig> = config.providers.iter().filter(|p| p.region() == region).collect();
        if list.is_empty() {
            continue;
        }
        let mut c = div().flex().flex_col().rounded(px(12.0)).bg(pal.surface).border_1().border_color(pal.border).overflow_hidden();
        for (i, p) in list.into_iter().enumerate() {
            let open = expanded.as_deref() == Some(p.id.as_str());
            let (text, tone) = status(p);
            let tone_color = match tone {
                1 => pal.success,
                2 => pal.danger,
                _ => pal.text_faint,
            };
            let id = p.id.clone();
            let id2 = p.id.clone();
            let hover = pal.surface_hover;
            let row = div()
                .id(gpui::ElementId::Name(format!("prov-{}", p.id).into()))
                .h(px(56.0))
                .px(px(16.0))
                .flex()
                .items_center()
                .gap(px(12.0))
                .cursor_pointer()
                .hover(move |s| s.bg(hover))
                .when(i > 0, |d| d.border_t_1().border_color(pal.border))
                .child(icon(if p.kind.is_local() { Icon::Cpu } else { Icon::Key }, 16.0, pal.text_muted))
                .child(div().flex_1().min_w_0().flex().flex_col().child(div().text_size(px(14.5)).child(p.display_name())).child(
                    div().text_size(px(12.0)).text_color(pal.text_faint).child({
                        let c = p.country();
                        if c.is_empty() { p.kind.vendor().to_string() } else { format!("{} · {c}", p.kind.vendor()) }
                    }),
                ))
                .child(div().text_size(px(12.5)).text_color(tone_color).child(text))
                .child(icon(if open { Icon::ChevronDown } else { Icon::ChevronRight }, 14.0, pal.text_faint))
                .on_click(cx.listener(move |this, _, _, cx| {
                    let next = if this.settings.as_ref().and_then(|s| s.expanded.as_deref()) == Some(id.as_str()) {
                        None
                    } else {
                        Some(id.clone())
                    };
                    this.expand_provider(next, cx)
                }));
            c = c.child(row);
            if open {
                let test = app.settings.as_ref().and_then(|s| s.tests.get(&p.id)).map(|t| match t {
                    TestState::Running(_) => ("Testing…".to_string(), pal.text_muted),
                    TestState::Ok(n, note) => (
                        format!(
                            "Connected · {n} model{}{}",
                            if *n == 1 { "" } else { "s" },
                            note.as_deref().map(|n| format!(" · {n}")).unwrap_or_default()
                        ),
                        pal.success,
                    ),
                    TestState::Err(e) => (e.clone(), pal.danger),
                });
                let key_focused = app.key_input.read(cx).is_focused(window);
                let url_focused = app.url_input.read(cx).is_focused(window);
                let visible = app.key_visible;
                let eye = kd_ui::icon_button("key-eye", if visible { Icon::EyeOff } else { Icon::Eye }, 28.0, pal).on_click(cx.listener(
                    |this, _, _, cx| {
                        this.key_visible = !this.key_visible;
                        let masked = !this.key_visible;
                        this.key_input.update(cx, |i, cx| i.set_masked(masked, cx));
                    },
                ));
                let mut form =
                    div().flex().flex_col().gap(px(14.0)).px(px(16.0)).pb(px(18.0)).pt(px(6.0)).border_t_1().border_color(pal.border);
                form = form.child(div().text_size(px(13.0)).text_color(pal.text_muted).child(p.kind.help()));
                if p.kind.needs_key() || p.api_key.is_some() {
                    let env = p.kind.key_env().first().map(|v| format!("or set ${v}")).unwrap_or_default();
                    form = form.child(field(
                        "API key",
                        Some(env),
                        div()
                            .flex()
                            .items_center()
                            .gap(px(6.0))
                            .child(input_box(app.key_input.clone().into(), key_focused, pal).flex_1())
                            .child(eye),
                        pal,
                    ));
                }
                if p.kind.endpoint_editable() {
                    form = form.child(field(
                        "Endpoint",
                        Some(p.kind.endpoint_hint().to_string()),
                        input_box(app.url_input.clone().into(), url_focused, pal),
                        pal,
                    ));
                }
                let pid = p.id.clone();
                form = form.child(
                    div()
                        .flex()
                        .items_center()
                        .gap(px(10.0))
                        .child(
                            primary_button("prov-save", "Save", pal).on_click(cx.listener(|this, _, _, cx| this.save_provider_fields(cx))),
                        )
                        .child(
                            button("prov-test", "Test connection", pal)
                                .on_click(cx.listener(move |this, _, _, cx| this.test_provider(&pid, cx))),
                        )
                        .child(
                            button("prov-toggle", if p.enabled { "Turn off" } else { "Turn on" }, pal)
                                .on_click(cx.listener(move |this, _, _, cx| this.toggle_provider(&id2, cx))),
                        )
                        .when_some(test, |d, (t, c)| d.child(div().flex_1().min_w_0().text_size(px(13.0)).text_color(c).child(t))),
                );
                c = c.child(form);
            }
        }
        col = col.child(
            div()
                .flex()
                .flex_col()
                .gap(px(8.0))
                .child(div().text_size(px(12.5)).text_color(pal.text_faint).child(region.label().to_uppercase()))
                .child(c),
        );
    }
    // the kinds that are not set up yet (chat providers; DeepL and FLUX come with their modules)
    let mut add = div().flex().flex_wrap().gap(px(8.0));
    let mut any = false;
    for (i, kind) in kd_ai::ProviderKind::ALL.into_iter().enumerate() {
        let custom = matches!(kind, kd_ai::ProviderKind::OpenAiCompatible | kd_ai::ProviderKind::Custom);
        if kind.service() != kd_ai::Service::Chat || (!custom && config.providers.iter().any(|p| p.kind == kind)) {
            continue;
        }
        any = true;
        add = add.child(
            button(("prov-add", i), format!("+ {}", kind.label()), pal)
                .on_click(cx.listener(move |this, _, _, cx| this.add_provider(kind, cx))),
        );
    }
    if any {
        col = col.child(
            div()
                .flex()
                .flex_col()
                .gap(px(8.0))
                .child(div().text_size(px(12.5)).text_color(pal.text_faint).child("ADD A PROVIDER"))
                .child(add),
        );
    }
    col
}

fn models(app: &mut KadeeApp, pal: &Palette, cx: &mut Context<KadeeApp>) -> impl IntoElement {
    let config = app.ai_config();
    let current = config.default_model.clone();
    let mut list = div().flex().flex_col().gap(px(2.0));
    let hover = pal.surface_hover;
    for (i, (header, label, model)) in model_menu_entries(app).into_iter().enumerate() {
        if let Some(h) = header {
            list = list.child(div().pt(px(12.0)).pb(px(4.0)).text_size(px(12.5)).text_color(pal.text_faint).child(h));
            continue;
        }
        let label = if model.is_none() { "Automatic — the first ready provider's recommended model".to_string() } else { label };
        let on = model == current;
        list = list.child(
            div()
                .id(("model-row", i))
                .h(px(36.0))
                .px(px(12.0))
                .flex()
                .items_center()
                .gap(px(10.0))
                .rounded(px(7.0))
                .cursor_pointer()
                .when(on, |d| d.bg(pal.accent_soft).text_color(pal.accent))
                .when(!on, |d| d.hover(move |s| s.bg(hover)))
                .child(icon(if on { Icon::Check } else { Icon::Cpu }, 14.0, if on { pal.accent } else { pal.text_faint }))
                .child(label)
                .on_click(cx.listener(move |this, _, _, cx| this.set_default_model(model.clone(), cx))),
        );
    }
    let now = match app.hub.model_for(kd_ai::Scope::Home) {
        Some(m) => format!("KaDee AI on the start screen uses {}.", model_label(&app.hub, &config, &m)),
        None => "No provider is ready: add one under AI providers.".to_string(),
    };
    card(pal)
        .child(div().text_size(px(15.5)).font_weight(FontWeight::MEDIUM).child("Default model"))
        .child(div().text_size(px(13.5)).text_color(pal.text_muted).child(now))
        .child(list)
}

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
    let _ = ModelRef::parse;
    card(pal)
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
                        .child(div().text_size(px(22.0)).child("KadeeStudio"))
                        .child(div().text_size(px(13.5)).text_color(pal.text_muted).child(format!(
                            "Version {} · KaDee Platform, standalone · Rust and GPUI",
                            kd_store::VERSION
                        ))),
                ),
        )
        .child(
            div()
                .flex()
                .flex_col()
                .gap(px(6.0))
                .text_size(px(14.0))
                .child(info("System", os))
                .child(info("Renderer", if cfg!(target_os = "macos") { "GPUI · Metal".into() } else if cfg!(windows) { "GPUI · DirectX".into() } else { "GPUI · Vulkan".into() }))
                .child(info("Settings", app.store.dir().display().to_string()))
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
                .child(button("about-mit", "MIT", pal).on_click(on_cmd(Command::OpenSettings(SettingsSection::LicenseMit), cx)))
                .child(button("about-apache", "Apache-2.0", pal).on_click(on_cmd(Command::OpenSettings(SettingsSection::LicenseApache), cx))),
        )
}

fn license(title: &'static str, text: &'static str, pal: &Palette) -> impl IntoElement {
    card(pal).child(div().text_size(px(17.0)).child(title)).child(
        div()
            .font_family("JetBrains Mono, Menlo, DejaVu Sans Mono, monospace")
            .text_size(px(12.5))
            .line_height(px(18.0))
            .text_color(pal.text_muted)
            .children(text.lines().map(|l| div().child(if l.is_empty() { " ".to_string() } else { l.to_string() }))),
    )
}
