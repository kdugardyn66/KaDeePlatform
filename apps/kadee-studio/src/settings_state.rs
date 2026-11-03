//! Settings' state and actions, as in KadeeStudio 0.3.1: Settings open over the middle pane;
//! changes show at once; **Done** (or going to another page) keeps them, **Cancel**, × or Esc
//! puts back what was there. AI settings are edited in a draft that is saved on Done.
//!
//! Keyboard: Change / Add and record take the next keys pressed (up to [`MAX_CHORDS`] in a
//! sequence). Enter saves at once, Esc cancels, Backspace starts over; after [`RECORD_IDLE`]
//! without a key the shortcut is saved.

use std::collections::HashMap;
use std::sync::mpsc::Receiver;
use std::time::{Duration, Instant};

use gpui::{Context, KeyDownEvent, Keystroke, Window};
use kd_ai::{AiConfig, AiHub, ModelRef, Protocol, ProviderConfig, ProviderKind, Region};
use kd_commands::{Binding, Chord, Keymap, Os, SettingsSection, bindable_commands};
use kd_store::{Settings, StoredShortcut};
use kd_ui::Theme;

use crate::app::KadeeApp;
use crate::keys;

/// After the last key of a recording, save it when nothing else is pressed for this long.
pub const RECORD_IDLE: Duration = Duration::from_millis(1500);
/// Longest sequence a recording accepts.
pub const MAX_CHORDS: usize = 3;

/// What a connection test answers: the provider's models and an account note.
type TestResult = Result<(Vec<String>, Option<String>), kd_ai::AiError>;

/// A connection test in Settings › AI providers.
pub enum TestState {
    Running(Receiver<TestResult>),
    Ok(Vec<String>, Option<String>),
    Err(String),
}

/// The screen inside AI providers.
#[derive(Debug, Clone, PartialEq, Default)]
pub enum AiScreen {
    #[default]
    List,
    /// Editing the provider with this id.
    Edit(String),
    /// Adding a provider, step 1: choose the kind.
    AddKind,
    /// Adding a provider, step 2: its details.
    AddForm(Box<ProviderConfig>),
}

/// A shortcut being recorded.
pub struct Recording {
    pub index: usize,
    pub chords: Vec<Chord>,
    pub last: Instant,
    /// A binding added just for this recording: removed again on Esc.
    pub is_new: bool,
}

/// Settings while they are open.
pub struct SettingsUi {
    pub section: SettingsSection,
    /// What was there when Settings opened (put back by Cancel).
    pub before: Settings,
    pub before_keymap: Keymap,
    /// The AI settings being edited (saved on Done).
    pub ai: AiConfig,
    pub screen: AiScreen,
    pub tests: HashMap<String, TestState>,
    pub recording: Option<Recording>,
    /// The action chosen under "Add a shortcut".
    pub add_choice: usize,
}

/// A drop-down list in Settings.
#[derive(Debug, Clone, PartialEq)]
pub enum Picker {
    /// "Add a shortcut": the action.
    ShortcutAction,
    /// AI models: the default model.
    DefaultModel,
    /// AI models: the model of one part of KadeeStudio (scope id).
    ScopeModel(String),
    /// A provider's default model.
    ProviderModel,
    /// A custom provider's API.
    Protocol,
    /// A custom provider's region.
    Region,
}

/// One row of a picker.
pub struct PickRow {
    pub header: bool,
    pub key: String,
    pub label: String,
    pub tag: Option<&'static str>,
    pub note: String,
    pub selected: bool,
}

impl PickRow {
    pub fn item(key: impl Into<String>, label: impl Into<String>, selected: bool) -> PickRow {
        PickRow { header: false, key: key.into(), label: label.into(), tag: None, note: String::new(), selected }
    }
    fn header(label: impl Into<String>) -> PickRow {
        PickRow { header: true, key: String::new(), label: label.into(), tag: None, note: String::new(), selected: false }
    }
}

/// gpui's key name → kd-commands' (`n` → `N`, `,` → `Comma`, `tab` → `Tab`).
pub fn chord_of(k: &Keystroke) -> Option<Chord> {
    let key = match k.key.as_str() {
        "" | "shift" | "control" | "alt" | "platform" | "function" | "cmd" | "ctrl" => return None,
        "," => "Comma".to_string(),
        "." => "Period".to_string(),
        "-" => "Minus".to_string(),
        "=" => "Equals".to_string(),
        "/" => "Slash".to_string(),
        "\\" => "Backslash".to_string(),
        ";" => "Semicolon".to_string(),
        "'" => "Quote".to_string(),
        "[" => "OpenBracket".to_string(),
        "]" => "CloseBracket".to_string(),
        "`" => "Backtick".to_string(),
        s if s.chars().count() == 1 => s.to_uppercase(),
        s => {
            let mut c = s.chars();
            let f = c.next()?;
            f.to_uppercase().collect::<String>() + c.as_str()
        }
    };
    let m = &k.modifiers;
    Some(Chord { ctrl: m.control, alt: m.alt, shift: m.shift, meta: m.platform, key })
}

/// A provider kind's badge: colour and initials (KadeeStudio's).
pub fn kind_badge(kind: ProviderKind) -> (u32, &'static str) {
    match kind {
        ProviderKind::AlephAlpha => (0x1F2937, "AA"),
        ProviderKind::BlackForestLabs => (0x111111, "BF"),
        ProviderKind::DeepL => (0x0F2B46, "DL"),
        ProviderKind::Mistral => (0xFA520F, "M"),
        ProviderKind::Alibaba => (0xFF6A00, "Qw"),
        ProviderKind::DeepSeek => (0x4D6BFE, "DS"),
        ProviderKind::Moonshot => (0x16162A, "K"),
        ProviderKind::Sakana => (0xC81E3A, "S"),
        ProviderKind::Zai => (0x2D3A8C, "Z"),
        ProviderKind::Anthropic => (0xD97757, "A"),
        ProviderKind::Cohere => (0x39594D, "Co"),
        ProviderKind::Gemini => (0x4285F4, "G"),
        ProviderKind::Meta => (0x0467DF, "M"),
        ProviderKind::Microsoft => (0x0078D4, "MS"),
        ProviderKind::OpenAi => (0x10A37F, "O"),
        ProviderKind::SpaceXAi => (0x000000, "X"),
        ProviderKind::LmStudio => (0x6B46C1, "LM"),
        ProviderKind::Ollama => (0x4B5563, "Ol"),
        ProviderKind::OpenAiCompatible => (0x64748B, "</>"),
        ProviderKind::Custom => (0x64748B, "+"),
    }
}

/// The card title on the Add screen: the company, or "Custom provider".
pub fn kind_title(kind: ProviderKind) -> &'static str {
    match kind {
        ProviderKind::Custom => "Custom provider",
        ProviderKind::OpenAiCompatible => "OpenAI-compatible",
        k => k.vendor(),
    }
}

/// Providers in region order, keeping their order inside a region.
pub fn by_region(providers: &[ProviderConfig]) -> Vec<&ProviderConfig> {
    let mut v: Vec<&ProviderConfig> = providers.iter().collect();
    v.sort_by_key(|p| Region::ALL.iter().position(|r| *r == p.region()).unwrap_or(9));
    v
}

/// The models to offer for `p`: its own list, the catalogue's, a test's result and the hub's.
pub fn offered(hub: &AiHub, tests: &HashMap<String, TestState>, p: &ProviderConfig) -> Vec<String> {
    let mut out = p.models.clone();
    let mut push = |m: &str| {
        if !out.iter().any(|x| x == m) {
            out.push(m.to_string());
        }
    };
    for m in hub.catalog().for_kind(p.kind) {
        push(&m.id);
    }
    if let Some(TestState::Ok(list, _)) = tests.get(&p.id) {
        for m in list {
            push(m);
        }
    }
    for m in hub.models_offered(&p.id) {
        push(&m);
    }
    out
}

/// "Claude Sonnet 5.5 · Anthropic · $$$".
pub fn model_summary(hub: &AiHub, config: &AiConfig, m: &ModelRef) -> String {
    let info = hub.model_info(m);
    let provider = config.provider(&m.provider).map(|p| p.display_name()).unwrap_or_default();
    match info.price_tier() {
        "" if info.input == Some(0.0) => format!("{} · {provider} · free", info.name),
        "" => format!("{} · {provider}", info.name),
        t => format!("{} · {provider} · {t}", info.name),
    }
}

/// Rows for choosing a model: every usable chat provider's models, grouped by provider.
pub fn model_rows(
    hub: &AiHub,
    config: &AiConfig,
    tests: &HashMap<String, TestState>,
    value: Option<&ModelRef>,
    none: &str,
) -> Vec<PickRow> {
    let mut out = vec![PickRow::item("", none, value.is_none())];
    for p in by_region(&config.providers).into_iter().filter(|p| p.usable_for_chat()) {
        let models = offered(hub, tests, p);
        if models.is_empty() {
            continue;
        }
        let default = p.default_model.clone().or_else(|| p.models.first().cloned()).or_else(|| models.first().cloned());
        out.push(PickRow::header(p.display_name()));
        for m in models {
            let r = ModelRef::new(p.id.clone(), m.clone());
            let info = hub.model_info(&r);
            out.push(PickRow {
                header: false,
                key: r.to_string(),
                label: info.name.clone(),
                tag: (default.as_deref() == Some(m.as_str())).then_some("default"),
                note: info.price_tier().to_string(),
                selected: value == Some(&r),
            });
        }
    }
    out
}

impl KadeeApp {
    pub fn ai_config(&self) -> AiConfig {
        self.hub.config()
    }

    /// Open Settings at `section`; remembers what was there for Cancel.
    pub fn open_settings(&mut self, section: SettingsSection, window: &mut Window, cx: &mut Context<Self>) {
        self.active = 0;
        match self.settings.as_mut() {
            Some(s) => {
                s.section = section;
                if !matches!(section, SettingsSection::AiProviders) {
                    s.screen = AiScreen::List;
                }
            }
            None => {
                self.name_input.update(cx, |i, cx| i.set_text(self.state.settings.display_name.clone(), cx));
                self.settings = Some(SettingsUi {
                    section,
                    before: self.state.settings.clone(),
                    before_keymap: self.keymap.clone(),
                    ai: self.hub.config(),
                    screen: AiScreen::List,
                    tests: HashMap::new(),
                    recording: None,
                    add_choice: 0,
                });
            }
        }
        window.focus(&self.focus);
        cx.notify();
    }

    /// Close Settings: Done keeps the changes, Cancel puts back what was there.
    pub fn close_settings(&mut self, keep: bool, cx: &mut Context<Self>) {
        let Some(s) = self.settings.take() else { return };
        if s.recording.is_some() {
            keys::bind(&self.keymap, cx);
        }
        if keep {
            if s.ai != self.hub.config()
                && let Err(e) = self.hub.set_config(s.ai)
            {
                self.notify(format!("Could not save the AI settings: {e}"), cx);
            }
        } else {
            let folder_changed = s.before.documents_dir != self.state.settings.documents_dir;
            self.state.settings = s.before;
            self.keymap = s.before_keymap;
            keys::bind(&self.keymap, cx);
            Theme::apply(self.state.settings.theme, cx);
            if folder_changed {
                self.refresh();
            }
            self.notify("Settings not changed", cx);
        }
        self.save(cx);
        cx.notify();
    }

    pub fn settings_section(&mut self, section: SettingsSection, cx: &mut Context<Self>) {
        if let Some(s) = self.settings.as_mut() {
            s.section = section;
            s.screen = AiScreen::List;
        }
        cx.notify();
    }

    // ---- Keyboard ----------------------------------------------------------------------------

    /// Keep the key map in the settings and bind it.
    fn keymap_changed(&mut self, cx: &mut Context<Self>) {
        self.state.settings.shortcuts =
            Some(self.keymap.to_stored(Os::current()).into_iter().map(|(command, keys)| StoredShortcut { command, keys }).collect());
        if self.settings.as_ref().is_none_or(|s| s.recording.is_none()) {
            keys::bind(&self.keymap, cx);
        }
        cx.notify();
    }

    pub fn is_recording(&self) -> bool {
        self.settings.as_ref().is_some_and(|s| s.recording.is_some())
    }

    pub fn start_recording(&mut self, index: usize, is_new: bool, window: &mut Window, cx: &mut Context<Self>) {
        let Some(s) = self.settings.as_mut() else { return };
        s.recording = Some(Recording { index, chords: Vec::new(), last: Instant::now(), is_new });
        // While recording, every key goes to the recording: no shortcut and no text field runs.
        cx.clear_key_bindings();
        window.focus(&self.focus);
        // save after a pause
        cx.spawn(async move |this, cx| {
            loop {
                cx.background_executor().timer(Duration::from_millis(100)).await;
                let go_on = this
                    .update(cx, |app, cx| {
                        let Some(rec) = app.settings.as_ref().and_then(|s| s.recording.as_ref()) else { return false };
                        if !rec.chords.is_empty() && rec.last.elapsed() >= RECORD_IDLE {
                            app.finish_recording(true, cx);
                            return false;
                        }
                        true
                    })
                    .unwrap_or(false);
                if !go_on {
                    break;
                }
            }
        })
        .detach();
        cx.notify();
    }

    /// A key while recording.
    pub fn record_key(&mut self, e: &KeyDownEvent, cx: &mut Context<Self>) {
        let Some(rec) = self.settings.as_mut().and_then(|s| s.recording.as_mut()) else { return };
        cx.stop_propagation();
        let Some(chord) = chord_of(&e.keystroke) else { return };
        let plain = !chord.has_modifier() && !chord.shift;
        match chord.key.as_str() {
            "Escape" if plain => return self.finish_recording(false, cx),
            "Enter" if plain && !rec.chords.is_empty() => return self.finish_recording(true, cx),
            "Backspace" if plain => {
                rec.chords.clear();
                rec.last = Instant::now();
            }
            _ => {
                rec.chords.push(chord);
                rec.last = Instant::now();
                if rec.chords.len() >= MAX_CHORDS {
                    return self.finish_recording(true, cx);
                }
            }
        }
        cx.notify();
    }

    fn finish_recording(&mut self, save: bool, cx: &mut Context<Self>) {
        let Some(r) = self.settings.as_mut().and_then(|s| s.recording.take()) else { return };
        if save && !r.chords.is_empty() {
            if let Some(b) = self.keymap.bindings.get_mut(r.index) {
                b.keys = r.chords;
            }
            let msg = match self.keymap.conflict(r.index) {
                Some(j) => format!("Saved — also used by “{}”", self.keymap.bindings[j].command.label()),
                None => "Shortcut saved".to_string(),
            };
            self.keymap_changed(cx);
            self.notify(msg, cx);
        } else {
            if r.is_new && r.index < self.keymap.bindings.len() {
                self.keymap.bindings.remove(r.index);
            }
            self.keymap_changed(cx);
        }
    }

    pub fn clear_shortcut(&mut self, i: usize, cx: &mut Context<Self>) {
        if let Some(b) = self.keymap.bindings.get_mut(i) {
            b.keys.clear();
        }
        self.keymap_changed(cx);
    }

    pub fn remove_shortcut(&mut self, i: usize, cx: &mut Context<Self>) {
        if i < self.keymap.bindings.len() {
            self.keymap.bindings.remove(i);
        }
        self.keymap_changed(cx);
    }

    pub fn add_shortcut(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let choice = self.settings.as_ref().map(|s| s.add_choice).unwrap_or(0);
        if let Some((command, _)) = bindable_commands().into_iter().nth(choice) {
            self.keymap.bindings.push(Binding { command, keys: Vec::new() });
            let index = self.keymap.bindings.len() - 1;
            self.start_recording(index, true, window, cx);
        }
    }

    pub fn reset_shortcuts(&mut self, cx: &mut Context<Self>) {
        self.keymap = Keymap::defaults(Os::current());
        self.state.settings.shortcuts = None;
        keys::bind(&self.keymap, cx);
        self.notify("Shortcuts reset to the defaults", cx);
    }

    // ---- AI providers ------------------------------------------------------------------------

    /// The provider on the form: the one being added, or the one being edited.
    pub fn form_provider(&self) -> Option<ProviderConfig> {
        let s = self.settings.as_ref()?;
        match &s.screen {
            AiScreen::AddForm(p) => Some((**p).clone()),
            AiScreen::Edit(id) => s.ai.provider(id).cloned(),
            _ => None,
        }
    }

    /// Change the provider on the form.
    pub fn update_form(&mut self, f: impl FnOnce(&mut ProviderConfig), cx: &mut Context<Self>) {
        let Some(s) = self.settings.as_mut() else { return };
        match &mut s.screen {
            AiScreen::AddForm(p) => f(p),
            AiScreen::Edit(id) => {
                let id = id.clone();
                if let Some(p) = s.ai.provider_mut(&id) {
                    f(p)
                }
            }
            _ => {}
        }
        cx.notify();
    }

    /// Show a screen of AI providers, loading the form's fields.
    pub fn ai_screen(&mut self, screen: AiScreen, cx: &mut Context<Self>) {
        let Some(s) = self.settings.as_mut() else { return };
        s.screen = screen;
        if let Some(p) = self.form_provider() {
            let hint = if p.kind.needs_key() { "Paste your key" } else { "Only if the server needs one" };
            let url_hint = p.kind.endpoint_hint().to_string();
            self.key_input.update(cx, |i, cx| {
                i.set_text(p.api_key.clone().unwrap_or_default(), cx);
                i.set_placeholder(hint, cx);
                i.set_masked(true, cx);
            });
            self.url_input.update(cx, |i, cx| {
                i.set_text(p.base_url.clone().unwrap_or_default(), cx);
                i.set_placeholder(if url_hint.is_empty() { "https://…/v1".to_string() } else { url_hint }, cx);
            });
            self.prov_name_input.update(cx, |i, cx| i.set_text(p.name.clone().unwrap_or_default(), cx));
            self.models_input.update(cx, |i, cx| i.set_text(p.models.join(", "), cx));
            self.key_visible = false;
        }
        cx.notify();
    }

    /// The form's text fields changed: copy them into the provider.
    pub fn form_fields_changed(&mut self, cx: &mut Context<Self>) {
        let key = self.key_input.read(cx).text().trim().to_string();
        let url = self.url_input.read(cx).text().trim().to_string();
        let name = self.prov_name_input.read(cx).text().trim().to_string();
        let models: Vec<String> =
            self.models_input.read(cx).text().split(',').map(|s| s.trim().to_string()).filter(|s| !s.is_empty()).collect();
        self.update_form(
            move |p| {
                if p.kind.needs_key() || matches!(p.kind, ProviderKind::OpenAiCompatible | ProviderKind::Custom) {
                    p.api_key = (!key.is_empty()).then_some(key);
                }
                if p.kind.endpoint_editable() {
                    p.base_url = (!url.is_empty()).then_some(url);
                }
                if p.kind == ProviderKind::Custom {
                    p.name = (!name.is_empty()).then_some(name);
                }
                if p.kind.lists_own_models() {
                    p.models = models;
                }
            },
            cx,
        );
    }

    /// Add a provider of this kind: the form to fill in.
    pub fn choose_kind(&mut self, kind: ProviderKind, cx: &mut Context<Self>) {
        let Some(s) = self.settings.as_ref() else { return };
        let mut p = ProviderConfig::new(kind);
        p.id = s.ai.free_id(kind.default_id());
        self.ai_screen(AiScreen::AddForm(Box::new(p)), cx);
    }

    /// Add the provider on the form to the draft and show it.
    pub fn add_form_provider(&mut self, cx: &mut Context<Self>) {
        let Some(s) = self.settings.as_mut() else { return };
        let AiScreen::AddForm(p) = std::mem::take(&mut s.screen) else { return };
        let id = p.id.clone();
        s.ai.providers.push(*p);
        self.ai_screen(AiScreen::Edit(id.clone()), cx);
        let name = self.settings.as_ref().and_then(|s| s.ai.provider(&id)).map(|p| p.display_name()).unwrap_or_default();
        self.notify(format!("{name} added — click Done to keep it"), cx);
    }

    /// Remove a provider from the draft, with the models that pointed at it.
    pub fn remove_provider(&mut self, id: &str, cx: &mut Context<Self>) {
        let Some(s) = self.settings.as_mut() else { return };
        let name = s.ai.provider(id).map(|p| p.display_name()).unwrap_or_default();
        s.ai.providers.retain(|p| p.id != id);
        if s.ai.default_model.as_ref().is_some_and(|m| m.provider == id) {
            s.ai.default_model = None;
        }
        s.ai.models.retain(|_, m| m.provider != id);
        s.tests.remove(id);
        s.screen = AiScreen::List;
        self.notify(format!("{name} removed — click Done to keep the change"), cx);
    }

    /// Ask the provider on the form for its models.
    pub fn test_form_provider(&mut self, cx: &mut Context<Self>) {
        let Some(p) = self.form_provider() else { return };
        let Some(s) = self.settings.as_mut() else { return };
        let (tx, rx) = std::sync::mpsc::channel();
        let probe = AiHub::in_memory(AiConfig { providers: vec![p.clone()], ..s.ai.clone() });
        let pid = p.id.clone();
        std::thread::spawn(move || {
            let r = probe.fetch_models(&pid).map(|list| (list, probe.account_note(&pid)));
            let _ = tx.send(r);
        });
        s.tests.insert(p.id.clone(), TestState::Running(rx));
        cx.spawn(async move |this, cx| {
            loop {
                cx.background_executor().timer(Duration::from_millis(120)).await;
                let more = this
                    .update(cx, |app, cx| {
                        let Some(s) = app.settings.as_mut() else { return false };
                        let mut changed = false;
                        for t in s.tests.values_mut() {
                            if let TestState::Running(rx) = t
                                && let Ok(r) = rx.try_recv()
                            {
                                changed = true;
                                *t = match r {
                                    Ok((list, note)) => TestState::Ok(list, note),
                                    Err(e) => TestState::Err(e.to_string()),
                                };
                            }
                        }
                        if changed {
                            cx.notify();
                        }
                        s.tests.values().any(|t| matches!(t, TestState::Running(_)))
                    })
                    .unwrap_or(false);
                if !more {
                    break;
                }
            }
        })
        .detach();
        cx.notify();
    }

    // ---- pickers -----------------------------------------------------------------------------

    /// The rows of a Settings picker.
    pub fn picker_rows(&self, picker: &Picker) -> Vec<PickRow> {
        let Some(s) = self.settings.as_ref() else { return Vec::new() };
        match picker {
            Picker::ShortcutAction => bindable_commands()
                .into_iter()
                .enumerate()
                .map(|(i, (_, label))| PickRow::item(i.to_string(), label, i == s.add_choice))
                .collect(),
            Picker::DefaultModel => model_rows(&self.hub, &s.ai, &s.tests, s.ai.default_model.as_ref(), "Automatic (first ready provider)"),
            Picker::ScopeModel(scope) => model_rows(&self.hub, &s.ai, &s.tests, s.ai.models.get(scope), "Use the default"),
            Picker::ProviderModel => {
                let Some(p) = self.form_provider() else { return Vec::new() };
                let models = offered(&self.hub, &s.tests, &p);
                let info_of = |m: &str| self.hub.catalog().info(p.kind, m);
                let fallback = p.models.first().cloned().or_else(|| models.first().cloned());
                let auto = fallback.map(|m| format!("Automatic ({})", info_of(&m).name)).unwrap_or_else(|| "Automatic".into());
                let mut out = vec![PickRow::item("", auto, p.default_model.is_none())];
                for m in models {
                    let info = info_of(&m);
                    out.push(PickRow {
                        header: false,
                        key: m.clone(),
                        label: info.name.clone(),
                        tag: info.recommended.then_some("recommended"),
                        note: info.price_tier().to_string(),
                        selected: p.default_model.as_deref() == Some(m.as_str()),
                    });
                }
                out
            }
            Picker::Protocol => {
                let current = self.form_provider().map(|p| p.protocol());
                Protocol::CHAT.iter().map(|x| PickRow::item(x.label(), x.label(), current == Some(*x))).collect()
            }
            Picker::Region => {
                let current = self.form_provider().map(|p| p.region());
                Region::ALL.iter().map(|r| PickRow::item(r.short(), r.label(), current == Some(*r))).collect()
            }
        }
    }

    /// A row of a Settings picker was chosen.
    pub fn pick(&mut self, picker: &Picker, key: &str, cx: &mut Context<Self>) {
        self.close_menu(cx);
        match picker {
            Picker::ShortcutAction => {
                if let (Some(s), Ok(i)) = (self.settings.as_mut(), key.parse::<usize>()) {
                    s.add_choice = i;
                }
            }
            Picker::DefaultModel => {
                if let Some(s) = self.settings.as_mut() {
                    s.ai.default_model = ModelRef::parse(key);
                }
            }
            Picker::ScopeModel(scope) => {
                if let Some(s) = self.settings.as_mut() {
                    match ModelRef::parse(key) {
                        Some(m) => s.ai.models.insert(scope.clone(), m),
                        None => s.ai.models.remove(scope),
                    };
                }
            }
            Picker::ProviderModel => {
                let k = key.to_string();
                self.update_form(move |p| p.default_model = (!k.is_empty()).then_some(k), cx)
            }
            Picker::Protocol => {
                let k = key.to_string();
                self.update_form(move |p| p.protocol = Protocol::CHAT.into_iter().find(|x| x.label() == k), cx)
            }
            Picker::Region => {
                let k = key.to_string();
                self.update_form(move |p| p.region = Region::ALL.into_iter().find(|r| r.short() == k), cx)
            }
        }
        cx.notify();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use gpui::Modifiers;

    fn ks(key: &str, m: Modifiers) -> Keystroke {
        Keystroke { modifiers: m, key: key.into(), key_char: None }
    }

    #[test]
    fn keystrokes_become_chords_that_round_trip() {
        let c = chord_of(&ks("n", Modifiers { platform: true, ..Default::default() })).unwrap();
        assert_eq!(c, Chord::new("N").meta());
        let c = chord_of(&ks(",", Modifiers { control: true, ..Default::default() })).unwrap();
        assert_eq!(c, Chord::new("Comma").ctrl());
        assert_eq!(keys::gpui_keystroke(&c), "ctrl-,");
        let c = chord_of(&ks("tab", Modifiers { control: true, shift: true, ..Default::default() })).unwrap();
        assert_eq!(keys::gpui_keystroke(&c), "ctrl-shift-tab");
        assert!(chord_of(&ks("shift", Modifiers::default())).is_none());
    }

    #[test]
    fn every_kind_has_a_badge_and_title() {
        for k in ProviderKind::ALL {
            assert!(!kind_badge(k).1.is_empty());
            assert!(!kind_title(k).is_empty());
        }
    }
}
