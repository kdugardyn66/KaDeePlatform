//! KaDee AI in a pane on the right: a general chat on Home (with the starred files as optional
//! context), a chat about the file in a file tab. Answers stream in; Stop cancels; each answer
//! shows its tokens and cost.

use gpui::{
    AnyElement, ClipboardItem, Context, FontWeight, IntoElement, MouseButton, MouseDownEvent, ParentElement, Styled, Window, div,
    prelude::*, px,
};
use kd_ai::{AiConfig, AiHub, ChatMessage, ModelRef, Speaker};
use kd_commands::{Command, SettingsSection};
use kd_ui::{Icon, Palette, Theme, icon, icon_button, mix, primary_button, tooltip, truncate};

use super::on_cmd;
use crate::app::{KadeeApp, MenuKind, file_name};

pub const PANE_W: f32 = 420.0;

const SUGGESTIONS: [&str; 4] = [
    "Draft a one-page project report",
    "Build a monthly household budget",
    "Outline a 10-slide product pitch",
    "Write release notes from my changelog",
];

/// "Claude Sonnet 5.5 · Anthropic".
pub fn model_label(hub: &AiHub, config: &AiConfig, m: &ModelRef) -> String {
    let info = hub.model_info(m);
    let provider = config.provider(&m.provider).map(|p| p.display_name()).unwrap_or_else(|| m.provider.clone());
    format!("{} · {provider}", info.name)
}

pub fn pane(app: &mut KadeeApp, window: &mut Window, cx: &mut Context<KadeeApp>) -> impl IntoElement {
    let pal = Theme::palette(cx);
    let key = app.chat_key();
    let file = (!key.is_empty()).then(|| std::path::PathBuf::from(&key));
    let model = app.chat_model();
    let config = app.ai_config();
    let model_text = match &model {
        Some(m) => model_label(&app.hub, &config, m),
        None => "No model ready".to_string(),
    };
    let starred_n = app.state.library.starred.len();
    let chat = app.chat(&key);
    let (empty, running, status, use_starred) = (chat.messages.is_empty(), chat.running(), chat.status.clone(), chat.use_starred);
    let messages: Vec<ChatMessage> = chat.messages.clone();
    let has_draft = !app.composer.read(cx).text().trim().is_empty();
    let focused = app.composer.read(cx).is_focused(window);

    let header = div()
        .h(px(50.0))
        .flex_none()
        .flex()
        .items_center()
        .justify_between()
        .px(px(16.0))
        .border_b_1()
        .border_color(pal.border)
        .child(
            div()
                .flex()
                .items_center()
                .gap(px(9.0))
                .child(icon(Icon::Sparkles, 17.0, pal.accent))
                .child(div().text_size(px(15.0)).font_weight(FontWeight::MEDIUM).child("KaDee AI"))
                .when_some(file.as_ref(), |d, f| {
                    d.child(div().text_size(px(12.5)).text_color(pal.text_faint).child(format!("· {}", truncate(&file_name(f), 24))))
                }),
        )
        .child(icon_button("ai-close", Icon::X, 28.0, &pal).tooltip(tooltip("Hide KaDee AI")).on_click(on_cmd(Command::ToggleAiPane, cx)));

    let can_insert = app.active_markdown().is_some();
    let mut body = div().flex().flex_col().px(px(16.0)).py(px(16.0));
    if empty {
        body = body.child(welcome(app, model.is_none(), file.as_deref(), &pal, cx));
    } else {
        let n = messages.len();
        for (i, m) in messages.iter().enumerate() {
            body = body.child(bubble(i, m, running && i + 1 == n, can_insert, &pal, cx));
        }
        if let Some(s) = status {
            body = body.child(div().text_size(px(12.5)).text_color(pal.text_faint).child(s));
        }
    }

    // composer
    let model_hover = pal.surface_hover;
    let model_button = div()
        .id("ai-model")
        .h(px(32.0))
        .px(px(10.0))
        .flex()
        .items_center()
        .gap(px(6.0))
        .min_w_0()
        .max_w(px(if key.is_empty() { 190.0 } else { 260.0 }))
        .rounded(px(7.0))
        .border_1()
        .border_color(pal.border)
        .cursor_pointer()
        .text_size(px(13.0))
        .hover(move |s| s.bg(model_hover))
        .child(icon(Icon::Cpu, 13.0, pal.text_faint))
        .child(div().flex_1().min_w_0().truncate().child(model_text))
        .child(icon(Icon::ChevronDown, 12.0, pal.text_muted))
        .on_mouse_down(
            MouseButton::Left,
            cx.listener(|this, e: &MouseDownEvent, _w, cx| {
                cx.stop_propagation();
                this.open_menu(MenuKind::Model, e.position, cx)
            }),
        );
    let starred_button = key.is_empty().then(|| {
        let (fg, bg, border) =
            if use_starred { (pal.accent, pal.accent_soft, pal.accent) } else { (pal.text_muted, pal.surface, pal.border) };
        div()
            .id("ai-starred")
            .h(px(32.0))
            .px(px(10.0))
            .flex()
            .flex_none()
            .items_center()
            .gap(px(5.0))
            .rounded(px(7.0))
            .border_1()
            .border_color(border)
            .bg(bg)
            .text_color(fg)
            .text_size(px(13.0))
            .cursor_pointer()
            .tooltip(tooltip("Answer from your starred files"))
            .child(icon(Icon::Star, 13.0, fg))
            .child(format!("Starred ({starred_n})"))
            .on_click(cx.listener(|this, _, _, cx| {
                let c = this.chat("");
                c.use_starred = !c.use_starred;
                cx.notify();
            }))
    });
    let send = if running {
        round_button("ai-stop", Icon::Stop, pal.text, pal.canvas, true, &pal)
            .tooltip(tooltip("Stop"))
            .on_click(cx.listener(|this, _, _, cx| this.stop_chat(cx)))
    } else {
        let bg = if has_draft { pal.accent } else { pal.border };
        round_button("ai-send", Icon::ArrowUp, bg, pal.on_accent, has_draft, &pal)
            .tooltip(tooltip("Send (Enter)"))
            .on_click(cx.listener(|this, _, window, cx| this.send_chat(window, cx)))
    };
    let new_chat = (!empty && !running).then(|| {
        let hover = pal.surface_hover;
        div()
            .id("ai-new")
            .h(px(32.0))
            .px(px(10.0))
            .flex()
            .items_center()
            .rounded(px(7.0))
            .text_size(px(13.0))
            .text_color(pal.text_muted)
            .cursor_pointer()
            .hover(move |s| s.bg(hover))
            .child("New chat")
            .on_click(cx.listener(|this, _, window, cx| this.new_chat(window, cx)))
    });
    let composer = div()
        .flex_none()
        .p(px(14.0))
        .child(
            div()
                .flex()
                .flex_col()
                .gap(px(10.0))
                .p(px(12.0))
                .rounded(px(14.0))
                .bg(pal.surface)
                .border_1()
                .border_color(if focused { pal.accent } else { pal.border_strong })
                .child(
                    div()
                        .id("ai-input-scroll")
                        .max_h(px(180.0))
                        .min_h(px(44.0))
                        .overflow_y_scroll()
                        .px(px(4.0))
                        .text_size(px(14.5))
                        .line_height(px(21.0))
                        .child(app.composer.clone()),
                )
                .child(
                    div()
                        .flex()
                        .items_center()
                        .gap(px(8.0))
                        .child(model_button)
                        .children(starred_button)
                        .child(div().flex_1())
                        .children(new_chat)
                        .child(send),
                ),
        )
        .child(
            div()
                .pt(px(6.0))
                .text_size(px(11.5))
                .text_color(pal.text_faint)
                .flex()
                .justify_center()
                .child("Enter to send · Shift+Enter for a new line"),
        );

    div()
        .w(px(PANE_W))
        .flex_none()
        .h_full()
        .flex()
        .flex_col()
        .bg(pal.canvas)
        .border_l_1()
        .border_color(pal.border)
        .child(header)
        .child(div().id("ai-scroll").flex_1().min_h_0().overflow_y_scroll().track_scroll(&app.ai_scroll).child(body))
        .child(composer)
}

fn round_button(id: &'static str, which: Icon, bg: gpui::Rgba, fg: gpui::Rgba, enabled: bool, pal: &Palette) -> gpui::Stateful<gpui::Div> {
    let hover = mix(bg, pal.canvas, 0.15);
    div()
        .id(id)
        .size(px(34.0))
        .flex_none()
        .flex()
        .items_center()
        .justify_center()
        .rounded(px(9.0))
        .bg(bg)
        .when(enabled, |d| d.cursor_pointer().hover(move |s| s.bg(hover)))
        .child(icon(which, 16.0, fg))
}

fn welcome(app: &KadeeApp, no_model: bool, file: Option<&std::path::Path>, pal: &Palette, cx: &mut Context<KadeeApp>) -> AnyElement {
    let sub = match file {
        Some(f) => format!("Ask about {}, or describe what to write.", file_name(f)),
        None => "Ask a question, or describe a report, spreadsheet or deck and KaDee AI drafts it.".to_string(),
    };
    let mut col = div()
        .flex()
        .flex_col()
        .items_center()
        .gap(px(6.0))
        .pt(px(28.0))
        .child(div().size(px(56.0)).rounded(px(16.0)).bg(pal.accent_soft).flex().items_center().justify_center().mb(px(8.0)).child(icon(
            Icon::Sparkles,
            28.0,
            pal.accent,
        )))
        .child(div().text_size(px(20.0)).font_weight(FontWeight::MEDIUM).child("How can I help?"))
        .child(div().text_size(px(14.0)).text_color(pal.text_muted).text_center().px(px(12.0)).child(sub));
    if no_model {
        col = col.child(
            div()
                .mt(px(20.0))
                .w_full()
                .p(px(16.0))
                .flex()
                .flex_col()
                .gap(px(6.0))
                .rounded(px(12.0))
                .bg(pal.surface)
                .border_1()
                .border_color(pal.border)
                .child(div().text_size(px(15.0)).font_weight(FontWeight::MEDIUM).child("No AI provider is ready yet"))
                .child(div().text_size(px(13.5)).text_color(pal.text_muted).child(
                    "Add an API key for Anthropic, OpenAI, Gemini or another provider — or connect a local Ollama or LM Studio server.",
                ))
                .child(
                    div().mt(px(8.0)).flex().child(
                        primary_button("ai-setup", "Set up AI providers", pal)
                            .on_click(on_cmd(Command::OpenSettings(SettingsSection::AiProviders), cx)),
                    ),
                ),
        );
    }
    if file.is_none() {
        let mut list = div().mt(px(22.0)).w_full().flex().flex_col().gap(px(8.0));
        for (i, text) in SUGGESTIONS.iter().enumerate() {
            let hover = pal.surface_hover;
            list = list.child(
                div()
                    .id(("ai-suggest", i))
                    .px(px(14.0))
                    .py(px(11.0))
                    .rounded(px(10.0))
                    .bg(pal.surface)
                    .border_1()
                    .border_color(pal.border)
                    .text_size(px(14.0))
                    .cursor_pointer()
                    .hover(move |s| s.bg(hover))
                    .child(*text)
                    .on_click(cx.listener(move |this, _, window, cx| this.set_draft(text, window, cx))),
            );
        }
        col = col.child(list);
    }
    let _ = app;
    col.into_any_element()
}

fn bubble(i: usize, m: &ChatMessage, streaming: bool, can_insert: bool, pal: &Palette, cx: &mut Context<KadeeApp>) -> AnyElement {
    match m.speaker {
        Speaker::User => div()
            .flex()
            .justify_end()
            .mb(px(14.0))
            .child(
                div()
                    .max_w(px(PANE_W * 0.82))
                    .px(px(14.0))
                    .py(px(9.0))
                    .rounded(px(14.0))
                    .bg(pal.accent_soft)
                    .text_size(px(14.5))
                    .line_height(px(21.0))
                    .child(m.text.clone()),
            )
            .into_any_element(),
        Speaker::Assistant => {
            let who = m.model.as_ref().map(|r| r.model.clone()).unwrap_or_else(|| "KaDee AI".into());
            let mut col = div().flex().flex_col().gap(px(6.0)).mb(px(18.0)).child(
                div()
                    .flex()
                    .items_center()
                    .gap(px(6.0))
                    .child(icon(Icon::Sparkles, 13.0, pal.accent))
                    .child(div().text_size(px(12.5)).text_color(pal.text_faint).child(who)),
            );
            if streaming && m.text.is_empty() {
                col = col.child(div().text_color(pal.text_faint).child("Thinking…"));
            } else if m.error {
                col = col.child(
                    div()
                        .px(px(12.0))
                        .py(px(8.0))
                        .rounded(px(8.0))
                        .bg(mix(pal.danger, pal.surface, 0.86))
                        .text_color(pal.danger)
                        .child(m.text.clone()),
                );
            } else {
                col = col.child(rich_text(&m.text, pal));
            }
            if !streaming {
                let text = m.text.clone();
                let hover = pal.surface_hover;
                col = col.child(
                    div()
                        .flex()
                        .items_center()
                        .gap(px(10.0))
                        .when_some(m.footer.clone(), |d, f| d.child(div().text_size(px(12.0)).text_color(pal.text_faint).child(f)))
                        .when(!m.error && !m.text.is_empty(), |d| {
                            d.child(
                                div()
                                    .id(("ai-copy", i))
                                    .flex()
                                    .items_center()
                                    .gap(px(4.0))
                                    .px(px(6.0))
                                    .py(px(2.0))
                                    .rounded(px(5.0))
                                    .text_size(px(12.0))
                                    .text_color(pal.text_muted)
                                    .cursor_pointer()
                                    .hover(move |s| s.bg(hover))
                                    .child(icon(Icon::Copy, 12.0, pal.text_muted))
                                    .child("Copy")
                                    .on_click(cx.listener(move |this, _, _, cx| {
                                        cx.write_to_clipboard(ClipboardItem::new_string(text.clone()));
                                        this.notify("Answer copied", cx);
                                    })),
                            )
                        })
                        .when(can_insert && !m.error && !m.text.is_empty(), |d| {
                            let text = m.text.clone();
                            d.child(
                                div()
                                    .id(("ai-insert", i))
                                    .flex()
                                    .items_center()
                                    .gap(px(4.0))
                                    .px(px(6.0))
                                    .py(px(2.0))
                                    .rounded(px(5.0))
                                    .text_size(px(12.0))
                                    .text_color(pal.accent)
                                    .cursor_pointer()
                                    .hover(move |s| s.bg(hover))
                                    .child(icon(Icon::Plus, 12.0, pal.accent))
                                    .child("Insert into document")
                                    .on_click(cx.listener(move |this, _, window, cx| this.insert_into_document(&text, window, cx))),
                            )
                        }),
                );
            }
            col.into_any_element()
        }
    }
}

/// An answer with light Markdown: headings, bullets, code blocks and paragraphs.
fn rich_text(text: &str, pal: &Palette) -> impl IntoElement {
    let mut col = div().flex().flex_col().gap(px(6.0)).text_size(px(14.5)).line_height(px(21.5));
    let mut code: Option<Vec<String>> = None;
    let mut para: Vec<String> = Vec::new();
    let flush = |para: &mut Vec<String>, col: gpui::Div| -> gpui::Div {
        if para.is_empty() {
            return col;
        }
        let t = para.join(" ");
        para.clear();
        col.child(div().child(t))
    };
    for line in text.lines() {
        if line.trim_start().starts_with("```") {
            match code.take() {
                Some(lines) => {
                    col = col.child(
                        div()
                            .px(px(12.0))
                            .py(px(9.0))
                            .rounded(px(8.0))
                            .bg(pal.surface)
                            .border_1()
                            .border_color(pal.border)
                            .font_family("JetBrains Mono, Menlo, DejaVu Sans Mono, monospace")
                            .text_size(px(13.0))
                            .line_height(px(19.0))
                            .children(lines.into_iter().map(|l| div().child(if l.is_empty() { " ".to_string() } else { l }))),
                    )
                }
                None => {
                    col = flush(&mut para, col);
                    code = Some(Vec::new());
                }
            }
            continue;
        }
        if let Some(c) = code.as_mut() {
            c.push(line.to_string());
            continue;
        }
        let t = line.trim();
        if t.is_empty() {
            col = flush(&mut para, col);
        } else if let Some(h) = t.strip_prefix("### ").or_else(|| t.strip_prefix("## ")).or_else(|| t.strip_prefix("# ")) {
            col = flush(&mut para, col);
            col = col.child(div().mt(px(4.0)).font_weight(FontWeight::SEMIBOLD).child(h.replace("**", "")));
        } else if let Some(b) = t.strip_prefix("- ").or_else(|| t.strip_prefix("* ")).or_else(|| t.strip_prefix("• ")) {
            col = flush(&mut para, col);
            col = col.child(
                div()
                    .flex()
                    .gap(px(8.0))
                    .pl(px(4.0))
                    .child(div().text_color(pal.text_faint).child("•"))
                    .child(div().flex_1().child(b.replace("**", ""))),
            );
        } else if t.chars().next().is_some_and(|c| c.is_ascii_digit()) && t.contains(". ") && t.find(". ").unwrap_or(9) <= 3 {
            col = flush(&mut para, col);
            let (n, rest) = t.split_at(t.find(". ").unwrap_or(0) + 1);
            col = col.child(
                div()
                    .flex()
                    .gap(px(8.0))
                    .pl(px(4.0))
                    .child(div().text_color(pal.text_faint).child(n.to_string()))
                    .child(div().flex_1().child(rest.trim().replace("**", ""))),
            );
        } else {
            para.push(t.replace("**", ""));
        }
    }
    if let Some(lines) = code {
        col = col.child(div().font_family("monospace").children(lines.into_iter().map(|l| div().child(l))));
    }
    flush(&mut para, col)
}

/// The rows of the model picker: "Default", then each ready provider's models.
pub fn model_menu_entries(app: &KadeeApp) -> Vec<(Option<String>, String, Option<ModelRef>)> {
    let config = app.ai_config();
    let mut out: Vec<(Option<String>, String, Option<ModelRef>)> = vec![(None, "Default for this chat".into(), None)];
    for p in config.providers.iter().filter(|p| p.usable_for_chat()) {
        out.push((Some(p.display_name()), String::new(), None));
        let mut models = app.hub.models_offered(&p.id);
        models.truncate(14);
        for m in models {
            let r = ModelRef::new(p.id.clone(), m);
            let name = app.hub.model_info(&r).name;
            out.push((None, name, Some(r)));
        }
    }
    out
}
