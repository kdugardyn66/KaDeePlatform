//! Commands and keyboard shortcuts: every user action in KadeeStudio is a
//! [`Command`].
//!
//! The tab strip, sidebar, cards, context menus and keyboard all produce a
//! `Command`; the shell runs it in one place. Each command has a stable id
//! (`file.new.markdown`) so a command palette, the CLI, MCP and AI can drive
//! the same actions later.
//!
//! Shortcuts are sequences of one or more [`Chord`]s: `Cmd+N` is one chord,
//! `Cmd+N M` is two pressed one after the other. The [`Keymap`] holds the
//! bindings (user-editable, saved as text), the [`Matcher`] turns key presses
//! into commands. This crate knows nothing about the UI toolkit.

use kd_store::{ModuleKind, OfficeKind, ThemePref};
use std::path::PathBuf;

/// Pages of the Home tab.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Page {
    Home,
    /// The New document chooser.
    New,
    Recent,
    Starred,
    Folder,
    CraftApps,
    /// The KaDee AI chat: opens the KaDee AI pane on the right.
    Ai,
}

/// Sections of the Settings window.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SettingsSection {
    General,
    Keyboard,
    /// KaDee AI: the connected providers and local servers.
    AiProviders,
    /// KaDee AI: which model each part of KadeeStudio uses.
    AiModels,
    About,
    /// License Agreement: the MIT license.
    LicenseMit,
    /// License Agreement: the Apache License 2.0.
    LicenseApache,
}

impl SettingsSection {
    pub const ALL: [SettingsSection; 7] = [
        SettingsSection::General,
        SettingsSection::Keyboard,
        SettingsSection::AiProviders,
        SettingsSection::AiModels,
        SettingsSection::About,
        SettingsSection::LicenseMit,
        SettingsSection::LicenseApache,
    ];

    pub fn label(self) -> &'static str {
        match self {
            SettingsSection::General => "General",
            SettingsSection::Keyboard => "Keyboard",
            SettingsSection::AiProviders => "AI providers",
            SettingsSection::AiModels => "AI models",
            SettingsSection::About => "About",
            SettingsSection::LicenseMit => "License — MIT",
            SettingsSection::LicenseApache => "License — Apache-2.0",
        }
    }

    fn id(self) -> &'static str {
        match self {
            SettingsSection::General => "general",
            SettingsSection::Keyboard => "keyboard",
            SettingsSection::AiProviders => "ai-providers",
            SettingsSection::AiModels => "ai-models",
            SettingsSection::About => "about",
            SettingsSection::LicenseMit => "license-mit",
            SettingsSection::LicenseApache => "license-apache",
        }
    }
}

/// A user action.
#[derive(Debug, Clone, PartialEq)]
pub enum Command {
    /// Show the "New document" chooser.
    NewDocumentPicker,
    /// Create a document of this kind in the documents folder and open it.
    NewDocument(ModuleKind),
    /// Create a blank Word document, spreadsheet or slide deck and open it.
    NewOffice(OfficeKind),
    /// Open a module that works on existing files (Convert).
    OpenModule(ModuleKind),
    /// Show the system Open dialog.
    OpenDialog,
    /// Open this file in a tab.
    OpenPath(PathBuf),
    /// Save the document in the active tab (Save As when it was never saved).
    Save,
    /// Save the document in the active tab under another name.
    SaveAs,
    /// Print the document in the active tab.
    Print,
    /// Move a file to the Trash after asking ("Delete files", as in GenOffice); a file that
    /// cannot be found asks to take it off the list instead.
    DeleteFile(PathBuf),
    /// Ask for a new name for a file (same folder, same extension).
    RenameFile(PathBuf),
    /// Copy a file next to itself as "<name> copy".
    DuplicateFile(PathBuf),
    /// Pick a folder in the system's folder dialog and pin it to the sidebar's Folders.
    PinFolder,
    /// Take a folder off the sidebar's Folders (the folder itself is untouched).
    UnpinFolder(PathBuf),
    /// Show the documents and sub-folders of a folder (a root of the folder tree or one below).
    ShowFolder(PathBuf),
    /// Open or close a folder in the sidebar's folder tree.
    ToggleFolder(PathBuf),
    /// Ask for a name and create a folder inside this one.
    NewFolder(PathBuf),
    /// Ask for a new name for this folder.
    RenameFolder(PathBuf),
    /// Move this folder and everything in it to the Trash, after asking.
    DeleteFolder(PathBuf),
    /// Switch to the Home tab and show a page.
    ShowPage(Page),
    /// Open the Settings window at a section.
    OpenSettings(SettingsSection),
    /// Show or hide the KaDee AI pane next to the open file.
    ToggleAiPane,
    /// Start an installed Crafting App, by its id (`photocraft`).
    LaunchCraft(String),
    /// Activate tab n (0 = Home).
    ActivateTab(usize),
    NextTab,
    PreviousTab,
    /// Close tab n (Home cannot be closed).
    CloseTab(usize),
    CloseActiveTab,
    ReopenClosedTab,
    ToggleStar(PathBuf),
    RemoveFromRecent(PathBuf),
    OpenExternally(PathBuf),
    RevealInFolder(PathBuf),
    CopyPath(PathBuf),
    SetTheme(ThemePref),
    CycleTheme,
    OpenUrl(String),
    Refresh,
}

impl Command {
    /// Stable id for palettes, logs, automation and the saved key map.
    pub fn id(&self) -> String {
        match self {
            Command::NewDocumentPicker => "file.new".into(),
            Command::NewDocument(k) => format!("file.new.{}", k.id()),
            Command::NewOffice(k) => format!("file.new.{}", k.id()),
            Command::OpenModule(k) => format!("module.open.{}", k.id()),
            Command::OpenDialog => "file.open".into(),
            Command::OpenPath(_) => "file.open_path".into(),
            Command::Save => "file.save".into(),
            Command::SaveAs => "file.save_as".into(),
            Command::Print => "file.print".into(),
            Command::DeleteFile(_) => "library.delete".into(),
            Command::RenameFile(_) => "file.rename".into(),
            Command::DuplicateFile(_) => "file.duplicate".into(),
            Command::PinFolder => "folder.pin".into(),
            Command::UnpinFolder(_) => "folder.unpin".into(),
            Command::ShowFolder(_) => "folder.show".into(),
            Command::ToggleFolder(_) => "folder.toggle".into(),
            Command::NewFolder(_) => "folder.new".into(),
            Command::RenameFolder(_) => "folder.rename".into(),
            Command::DeleteFolder(_) => "folder.delete".into(),
            Command::ShowPage(p) => format!("home.show.{}", format!("{p:?}").to_lowercase()),
            Command::OpenSettings(s) => format!("settings.open.{}", s.id()),
            Command::ToggleAiPane => "ai.toggle_pane".into(),
            Command::LaunchCraft(id) => format!("craft.launch.{id}"),
            Command::ActivateTab(n) => format!("tab.go.{}", n + 1),
            Command::NextTab => "tab.next".into(),
            Command::PreviousTab => "tab.previous".into(),
            Command::CloseTab(_) => "tab.close".into(),
            Command::CloseActiveTab => "tab.close_active".into(),
            Command::ReopenClosedTab => "tab.reopen".into(),
            Command::ToggleStar(_) => "library.star".into(),
            Command::RemoveFromRecent(_) => "library.remove_recent".into(),
            Command::OpenExternally(_) => "file.open_external".into(),
            Command::RevealInFolder(_) => "file.reveal".into(),
            Command::CopyPath(_) => "file.copy_path".into(),
            Command::SetTheme(t) => format!("view.theme.{}", t.label().to_lowercase()),
            Command::CycleTheme => "view.theme.cycle".into(),
            Command::OpenUrl(_) => "app.open_url".into(),
            Command::Refresh => "home.refresh".into(),
        }
    }

    /// The bindable command with this id.
    pub fn from_id(id: &str) -> Option<Command> {
        // ids of 0.1.0 previews, when Settings and Keyboard shortcuts were pages
        match id {
            "home.show.settings" => return Some(Command::OpenSettings(SettingsSection::General)),
            "home.show.shortcuts" => return Some(Command::OpenSettings(SettingsSection::Keyboard)),
            _ => {}
        }
        bindable_commands().into_iter().map(|(c, _)| c).find(|c| c.id() == id)
    }

    /// Human-readable name of a bindable command.
    pub fn label(&self) -> String {
        bindable_commands().into_iter().find(|(c, _)| c == self).map(|(_, l)| l).unwrap_or_else(|| self.id())
    }
}

/// Every command a shortcut can be bound to, with its label, in menu order.
pub fn bindable_commands() -> Vec<(Command, String)> {
    let mut v: Vec<(Command, String)> = vec![
        (Command::NewDocumentPicker, "New document…".into()),
        (Command::NewDocument(ModuleKind::Markdown), "New Markdown document".into()),
        (Command::NewDocument(ModuleKind::Pdf), "New PDF document".into()),
        (Command::NewDocument(ModuleKind::Dashboard), "New dashboard".into()),
        (Command::NewDocument(ModuleKind::Report), "New report".into()),
        (Command::NewOffice(OfficeKind::Word), "New Word document".into()),
        (Command::NewOffice(OfficeKind::Spreadsheet), "New spreadsheet".into()),
        (Command::NewOffice(OfficeKind::SlideDeck), "New slide deck".into()),
        (Command::OpenDialog, "Open a file".into()),
        (Command::Save, "Save".into()),
        (Command::SaveAs, "Save As…".into()),
        (Command::Print, "Print…".into()),
        (Command::CloseActiveTab, "Close tab".into()),
        (Command::ReopenClosedTab, "Reopen closed tab".into()),
        (Command::NextTab, "Next tab".into()),
        (Command::PreviousTab, "Previous tab".into()),
        (Command::ActivateTab(0), "Home tab".into()),
    ];
    for n in 1..=8 {
        v.push((Command::ActivateTab(n), format!("Go to tab {}", n + 1)));
    }
    v.extend([
        (Command::ShowPage(Page::Home), "Show Home".into()),
        (Command::ShowPage(Page::New), "Show New document".into()),
        (Command::ShowPage(Page::Recent), "Show Recent".into()),
        (Command::ShowPage(Page::Starred), "Show Starred".into()),
        (Command::ShowPage(Page::CraftApps), "Show Apps".into()),
        (Command::ShowPage(Page::Ai), "Show KaDee AI".into()),
        (Command::ToggleAiPane, "Show or hide the KaDee AI pane".into()),
        (Command::ShowPage(Page::Folder), "Show the KadeeStudio folder".into()),
        (Command::PinFolder, "Add a folder to the sidebar…".into()),
        (Command::OpenSettings(SettingsSection::General), "Settings".into()),
        (Command::OpenSettings(SettingsSection::Keyboard), "Keyboard shortcuts".into()),
        (Command::OpenSettings(SettingsSection::AiProviders), "AI providers".into()),
        (Command::OpenSettings(SettingsSection::AiModels), "AI models".into()),
        (Command::OpenSettings(SettingsSection::About), "About KadeeStudio".into()),
        (Command::CycleTheme, "Switch theme (System, Light, Dark)".into()),
        (Command::SetTheme(ThemePref::Light), "Light theme".into()),
        (Command::SetTheme(ThemePref::Dark), "Dark theme".into()),
        (Command::SetTheme(ThemePref::System), "System theme".into()),
        (Command::Refresh, "Refresh the document list".into()),
    ]);
    v
}

// ------------------------------------------------------------------ operating system

/// The system KadeeStudio runs on: it decides the names of the modifier keys
/// and which one is the main shortcut key.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Os {
    Mac,
    Windows,
    Linux,
}

impl Os {
    /// The system this build runs on (BSDs and others behave like Linux).
    pub const fn current() -> Os {
        if cfg!(target_os = "macos") {
            Os::Mac
        } else if cfg!(target_os = "windows") {
            Os::Windows
        } else {
            Os::Linux
        }
    }

    /// Name of the system key next to the space bar: Cmd, Win or Super.
    pub fn meta_name(self) -> &'static str {
        match self {
            Os::Mac => "Cmd",
            Os::Windows => "Win",
            Os::Linux => "Super",
        }
    }

    /// Name of the Alt key: Option on the Mac.
    pub fn alt_name(self) -> &'static str {
        match self {
            Os::Mac => "Option",
            Os::Windows | Os::Linux => "Alt",
        }
    }

    /// The main shortcut key: Cmd on the Mac, Ctrl elsewhere.
    pub fn primary_name(self) -> &'static str {
        match self {
            Os::Mac => "Cmd",
            Os::Windows | Os::Linux => "Ctrl",
        }
    }

    /// The four modifier keys as this system names them, in display order.
    pub fn modifier_names(self) -> [&'static str; 4] {
        match self {
            Os::Mac => ["Cmd", "Ctrl", "Option", "Shift"],
            Os::Windows => ["Win", "Ctrl", "Alt", "Shift"],
            Os::Linux => ["Super", "Ctrl", "Alt", "Shift"],
        }
    }

    /// Human name of the system.
    pub fn name(self) -> &'static str {
        match self {
            Os::Mac => "macOS",
            Os::Windows => "Windows",
            Os::Linux => "Linux",
        }
    }
}

// ------------------------------------------------------------------ chords

/// One key press with the modifier keys held, as four separate keys:
/// `ctrl` (Control), `alt` (Option on the Mac), `shift`, and `meta`
/// (Cmd on the Mac, the Windows key on Windows, Super on Linux).
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Chord {
    pub ctrl: bool,
    pub alt: bool,
    pub shift: bool,
    pub meta: bool,
    /// Key name as the UI toolkit spells it: `"N"`, `"Tab"`, `"Comma"`, `"1"`.
    pub key: String,
}

impl Chord {
    pub fn new(key: &str) -> Chord {
        Chord { ctrl: false, alt: false, shift: false, meta: false, key: key.to_string() }
    }
    /// Add the main shortcut key of `os`: Cmd on the Mac, Ctrl elsewhere.
    pub fn primary(self, os: Os) -> Chord {
        if os == Os::Mac { self.meta() } else { self.ctrl() }
    }
    pub fn ctrl(mut self) -> Chord {
        self.ctrl = true;
        self
    }
    pub fn alt(mut self) -> Chord {
        self.alt = true;
        self
    }
    pub fn shift(mut self) -> Chord {
        self.shift = true;
        self
    }
    pub fn meta(mut self) -> Chord {
        self.meta = true;
        self
    }

    /// True when Ctrl, Alt/Option or Cmd/Win/Super is held (plain keys are left to text fields).
    pub fn has_modifier(&self) -> bool {
        self.ctrl || self.alt || self.meta
    }

    /// Text as saved in `home.json`. The main shortcut key of `os` is saved as
    /// `Primary`, so `Cmd+N` on a Mac becomes `Ctrl+N` on Windows and Linux.
    pub fn to_text(&self, os: Os) -> String {
        let primary_is_meta = os == Os::Mac;
        let mut parts: Vec<&str> = Vec::new();
        if self.ctrl {
            parts.push(if primary_is_meta { "Ctrl" } else { "Primary" });
        }
        if self.meta {
            parts.push(if primary_is_meta { "Primary" } else { "Meta" });
        }
        if self.alt {
            parts.push("Alt");
        }
        if self.shift {
            parts.push("Shift");
        }
        parts.push(&self.key);
        parts.join("+")
    }

    /// Parse saved or typed text. `Primary` is Cmd on the Mac and Ctrl
    /// elsewhere; `Cmd`, `Win`, `Super` and `Meta` all mean the system key.
    pub fn parse(text: &str, os: Os) -> Option<Chord> {
        let parts: Vec<&str> = text.split('+').map(str::trim).collect();
        let (mods, key) = match parts.split_last() {
            Some((k, m)) if !k.is_empty() => (m, k.to_string()),
            _ => return None,
        };
        let mut c = Chord::new(&key);
        for m in mods {
            c = match m.to_ascii_lowercase().as_str() {
                "primary" => c.primary(os),
                "cmd" | "command" | "meta" | "super" | "win" | "windows" | "logo" => c.meta(),
                "ctrl" | "control" => c.ctrl(),
                "alt" | "option" | "opt" => c.alt(),
                "shift" => c.shift(),
                _ => return None,
            };
        }
        Some(c)
    }

    /// What people see, with the names of `os`:
    /// macOS `Cmd+Shift+D`, `Ctrl+Tab`, `Cmd+Option+P`;
    /// Windows `Ctrl+Shift+D`, `Win+E`; Linux `Ctrl+Alt+T`, `Super+E`.
    pub fn display(&self, os: Os) -> String {
        let mut parts: Vec<&str> = Vec::new();
        match os {
            // Cmd first, as the main key, then Control, Option, Shift.
            Os::Mac => {
                if self.meta {
                    parts.push("Cmd");
                }
                if self.ctrl {
                    parts.push("Ctrl");
                }
                if self.alt {
                    parts.push("Option");
                }
            }
            // The system key first (Win+Shift+S), then Ctrl, Alt.
            Os::Windows | Os::Linux => {
                if self.meta {
                    parts.push(os.meta_name());
                }
                if self.ctrl {
                    parts.push("Ctrl");
                }
                if self.alt {
                    parts.push("Alt");
                }
            }
        }
        if self.shift {
            parts.push("Shift");
        }
        parts.push(key_display(&self.key));
        parts.join("+")
    }
}

fn key_display(name: &str) -> &str {
    match name {
        "Comma" => ",",
        "Period" => ".",
        "Minus" => "-",
        "Plus" => "+",
        "Equals" => "=",
        "Slash" => "/",
        "Backslash" => "\\",
        "Semicolon" => ";",
        "Quote" => "'",
        "OpenBracket" => "[",
        "CloseBracket" => "]",
        "Backtick" => "`",
        "Escape" => "Esc",
        "Up" => "↑",
        "Down" => "↓",
        "Left" => "←",
        "Right" => "→",
        other => other,
    }
}

/// A sequence as saved text: chords separated by spaces (`Primary+N M`).
pub fn sequence_to_text(keys: &[Chord], os: Os) -> String {
    keys.iter().map(|c| c.to_text(os)).collect::<Vec<_>>().join(" ")
}

/// Parse [`sequence_to_text`]; empty text is an unassigned shortcut.
pub fn parse_sequence(text: &str, os: Os) -> Option<Vec<Chord>> {
    text.split_whitespace().map(|c| Chord::parse(c, os)).collect()
}

/// A sequence as people see it: `Cmd+N M`.
pub fn display_sequence(keys: &[Chord], os: Os) -> String {
    keys.iter().map(|c| c.display(os)).collect::<Vec<_>>().join(" ")
}

// ------------------------------------------------------------------ key map

/// One shortcut: a command and the keys that run it (empty = not assigned).
#[derive(Debug, Clone, PartialEq)]
pub struct Binding {
    pub command: Command,
    pub keys: Vec<Chord>,
}

/// All shortcuts, in display order.
#[derive(Debug, Clone, PartialEq)]
pub struct Keymap {
    pub bindings: Vec<Binding>,
}

impl Keymap {
    /// The shipped shortcuts, with the main shortcut key of `os`.
    pub fn defaults(os: Os) -> Keymap {
        let p = |k: &str| vec![Chord::new(k).primary(os)];
        let ps = |k: &str| vec![Chord::new(k).primary(os).shift()];
        // Cmd+N opens the New document menu; a letter then picks the type.
        let n = |k: &str| vec![Chord::new("N").primary(os), Chord::new(k)];
        let mut bindings = vec![
            Binding { command: Command::NewDocumentPicker, keys: p("N") },
            Binding { command: Command::NewDocument(ModuleKind::Markdown), keys: n("M") },
            Binding { command: Command::NewDocument(ModuleKind::Pdf), keys: n("P") },
            Binding { command: Command::NewDocument(ModuleKind::Dashboard), keys: n("D") },
            Binding { command: Command::NewDocument(ModuleKind::Report), keys: n("R") },
            Binding { command: Command::NewOffice(OfficeKind::Word), keys: n("W") },
            Binding { command: Command::NewOffice(OfficeKind::Spreadsheet), keys: n("G") },
            Binding { command: Command::NewOffice(OfficeKind::SlideDeck), keys: n("S") },
            Binding { command: Command::NewDocument(ModuleKind::Dashboard), keys: ps("D") },
            Binding { command: Command::NewDocument(ModuleKind::Report), keys: ps("R") },
            Binding { command: Command::OpenDialog, keys: p("O") },
            Binding { command: Command::Save, keys: p("S") },
            Binding { command: Command::SaveAs, keys: vec![Chord::new("S").primary(os).alt()] },
            Binding { command: Command::Print, keys: p("P") },
            Binding { command: Command::CloseActiveTab, keys: p("W") },
            Binding { command: Command::ReopenClosedTab, keys: ps("T") },
            Binding { command: Command::NextTab, keys: vec![Chord::new("Tab").ctrl()] },
            Binding { command: Command::PreviousTab, keys: vec![Chord::new("Tab").ctrl().shift()] },
            Binding { command: Command::ActivateTab(0), keys: p("1") },
        ];
        for n in 1..=8usize {
            bindings.push(Binding { command: Command::ActivateTab(n), keys: p(&(n + 1).to_string()) });
        }
        bindings.push(Binding { command: Command::OpenSettings(SettingsSection::General), keys: p("Comma") });
        bindings.push(Binding { command: Command::ShowPage(Page::Ai), keys: ps("A") });
        bindings.push(Binding { command: Command::ToggleAiPane, keys: p("J") });
        Keymap { bindings }
    }

    /// Rebuild from saved `(command id, keys text)` pairs; unknown ids are skipped.
    pub fn from_stored<'a>(entries: impl IntoIterator<Item = (&'a str, &'a str)>, os: Os) -> Keymap {
        let bindings = entries
            .into_iter()
            .filter_map(|(id, keys)| Some(Binding { command: Command::from_id(id)?, keys: parse_sequence(keys, os)? }))
            .collect();
        Keymap { bindings }
    }

    /// `(command id, keys text)` pairs for saving.
    pub fn to_stored(&self, os: Os) -> Vec<(String, String)> {
        self.bindings.iter().map(|b| (b.command.id(), sequence_to_text(&b.keys, os))).collect()
    }

    /// The first assigned keys for a command.
    pub fn keys_for(&self, cmd: &Command) -> Option<&[Chord]> {
        self.bindings.iter().find(|b| &b.command == cmd && !b.keys.is_empty()).map(|b| b.keys.as_slice())
    }

    /// Another binding using exactly the same keys as binding `i`.
    pub fn conflict(&self, i: usize) -> Option<usize> {
        let keys = &self.bindings.get(i)?.keys;
        if keys.is_empty() {
            return None;
        }
        self.bindings.iter().enumerate().find(|(j, b)| *j != i && &b.keys == keys).map(|(j, _)| j)
    }

    /// The command bound to exactly these keys.
    pub fn command_for(&self, seq: &[Chord]) -> Option<&Command> {
        self.exact(seq)
    }

    /// Keys that can follow `prefix`, with the command each leads to
    /// (the letters of the New document menu after Cmd+N).
    pub fn next_keys(&self, prefix: &[Chord]) -> Vec<(Chord, &Command)> {
        self.bindings
            .iter()
            .filter(|b| b.keys.len() == prefix.len() + 1 && b.keys.starts_with(prefix))
            .map(|b| (b.keys[prefix.len()].clone(), &b.command))
            .collect()
    }

    fn exact(&self, seq: &[Chord]) -> Option<&Command> {
        self.bindings.iter().find(|b| b.keys == seq).map(|b| &b.command)
    }

    fn has_longer(&self, seq: &[Chord]) -> bool {
        self.bindings.iter().any(|b| b.keys.len() > seq.len() && b.keys.starts_with(seq))
    }
}

// ------------------------------------------------------------------ matcher

/// How long to wait for the next key of a sequence when the keys so far
/// already run a command of their own.
pub const SEQUENCE_TIMEOUT: f64 = 1.0;

/// What a key press did.
#[derive(Debug, Clone, PartialEq)]
pub enum Outcome {
    /// Not a shortcut: let the key through.
    None,
    /// The start of a sequence: wait for the next key.
    Pending,
    /// Run this command.
    Fire(Command),
}

/// Turns key presses into commands, sequences included.
#[derive(Debug, Default, Clone)]
pub struct Matcher {
    pending: Vec<Chord>,
    since: f64,
}

impl Matcher {
    /// Keys pressed so far in an unfinished sequence.
    pub fn pending(&self) -> &[Chord] {
        &self.pending
    }

    pub fn cancel(&mut self) {
        self.pending.clear();
    }

    /// Feed one key press at time `now` (seconds).
    pub fn feed(&mut self, keymap: &Keymap, chord: Chord, now: f64) -> Outcome {
        let mut seq = self.pending.clone();
        seq.push(chord.clone());
        if keymap.has_longer(&seq) {
            self.pending = seq;
            self.since = now;
            return Outcome::Pending;
        }
        if let Some(cmd) = keymap.exact(&seq) {
            self.pending.clear();
            return Outcome::Fire(cmd.clone());
        }
        if !self.pending.is_empty() {
            // Not a continuation: drop the sequence and treat the key on its own.
            self.pending.clear();
            return self.feed(keymap, chord, now);
        }
        Outcome::None
    }

    /// Call every frame: when the wait runs out, run what the keys so far are bound to.
    pub fn tick(&mut self, keymap: &Keymap, now: f64) -> Option<Command> {
        if self.pending.is_empty() || now - self.since < SEQUENCE_TIMEOUT {
            return None;
        }
        let seq = std::mem::take(&mut self.pending);
        keymap.exact(&seq).cloned()
    }

    /// Seconds left before [`Matcher::tick`] gives up waiting.
    pub fn time_left(&self, now: f64) -> f64 {
        (SEQUENCE_TIMEOUT - (now - self.since)).max(0.0)
    }

    /// What can follow the pending keys: the next chord and the command it leads to.
    pub fn continuations<'a>(&self, keymap: &'a Keymap) -> Vec<(Chord, &'a Command)> {
        let n = self.pending.len();
        keymap
            .bindings
            .iter()
            .filter(|b| b.keys.len() > n && b.keys.starts_with(&self.pending))
            .map(|b| (b.keys[n].clone(), &b.command))
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const MAC: Os = Os::Mac;
    const WIN: Os = Os::Windows;
    const LIN: Os = Os::Linux;

    fn seq(text: &str, os: Os) -> Vec<Chord> {
        parse_sequence(text, os).unwrap()
    }

    #[test]
    fn ids_are_stable_and_round_trip() {
        assert_eq!(Command::NewDocumentPicker.id(), "file.new");
        assert_eq!(Command::NewDocument(ModuleKind::Markdown).id(), "file.new.markdown");
        assert_eq!(Command::ActivateTab(0).id(), "tab.go.1");
        assert_eq!(Command::OpenSettings(SettingsSection::Keyboard).id(), "settings.open.keyboard");
        assert_eq!(Command::from_id("home.show.shortcuts"), Some(Command::OpenSettings(SettingsSection::Keyboard)));
        assert_eq!(Command::from_id("settings.open.about"), Some(Command::OpenSettings(SettingsSection::About)));
        for (c, _) in bindable_commands() {
            assert_eq!(Command::from_id(&c.id()), Some(c.clone()), "{}", c.id());
        }
    }

    #[test]
    fn defaults_use_each_systems_main_key() {
        let mac = Keymap::defaults(MAC);
        let show = |k: &Keymap, c: &Command, os: Os| display_sequence(k.keys_for(c).unwrap(), os);
        assert_eq!(show(&mac, &Command::NewDocumentPicker, MAC), "Cmd+N");
        assert_eq!(show(&mac, &Command::NextTab, MAC), "Ctrl+Tab");
        assert_eq!(show(&mac, &Command::PreviousTab, MAC), "Ctrl+Shift+Tab");
        // the New document menu: Cmd+N, then a letter
        for (cmd, letter) in [
            (Command::NewDocument(ModuleKind::Markdown), "M"),
            (Command::NewDocument(ModuleKind::Pdf), "P"),
            (Command::NewDocument(ModuleKind::Dashboard), "D"),
            (Command::NewDocument(ModuleKind::Report), "R"),
            (Command::NewOffice(OfficeKind::Word), "W"),
            (Command::NewOffice(OfficeKind::Spreadsheet), "G"),
            (Command::NewOffice(OfficeKind::SlideDeck), "S"),
        ] {
            assert_eq!(show(&mac, &cmd, MAC), format!("Cmd+N {letter}"));
            assert_eq!(show(&Keymap::defaults(LIN), &cmd, LIN), format!("Ctrl+N {letter}"));
        }
        let menu = mac.next_keys(&[Chord::new("N").meta()]);
        assert_eq!(menu.iter().map(|(c, _)| c.key.as_str()).collect::<String>(), "MPDRWGS");
        // Cmd+Shift+D and Cmd+Shift+R still work too
        assert!(
            mac.bindings
                .iter()
                .any(|b| b.command == Command::NewDocument(ModuleKind::Dashboard) && display_sequence(&b.keys, MAC) == "Cmd+Shift+D")
        );
        for os in [WIN, LIN] {
            let k = Keymap::defaults(os);
            assert_eq!(show(&k, &Command::NewDocumentPicker, os), "Ctrl+N");
            assert_eq!(show(&k, &Command::NextTab, os), "Ctrl+Tab");
            assert_eq!(show(&k, &Command::OpenSettings(SettingsSection::General), os), "Ctrl+,");
        }
        for os in [MAC, WIN, LIN] {
            let k = Keymap::defaults(os);
            for i in 0..k.bindings.len() {
                assert_eq!(k.conflict(i), None, "duplicate default {} on {os:?}", k.bindings[i].command.id());
            }
        }
    }

    #[test]
    fn four_separate_modifier_keys_with_system_names() {
        let all = Chord::new("K").meta().ctrl().alt().shift();
        assert_eq!(all.display(MAC), "Cmd+Ctrl+Option+Shift+K");
        assert_eq!(all.display(WIN), "Win+Ctrl+Alt+Shift+K");
        assert_eq!(all.display(LIN), "Super+Ctrl+Alt+Shift+K");
        // Cmd and Ctrl are different keys on the Mac
        assert_ne!(Chord::new("N").meta(), Chord::new("N").ctrl());
        assert_eq!(Chord::new("N").ctrl().display(MAC), "Ctrl+N");
        assert_eq!(Chord::new("E").meta().display(WIN), "Win+E");
        assert_eq!(Chord::new("T").ctrl().alt().display(LIN), "Ctrl+Alt+T");
        assert_eq!(Os::Mac.modifier_names(), ["Cmd", "Ctrl", "Option", "Shift"]);
    }

    #[test]
    fn saved_text_moves_between_systems() {
        // Cmd+N on a Mac is saved as Primary+N and loads as Ctrl+N on Windows
        let mac = seq("Primary+N M", MAC);
        assert_eq!(mac, vec![Chord::new("N").meta(), Chord::new("M")]);
        let text = sequence_to_text(&mac, MAC);
        assert_eq!(text, "Primary+N M");
        assert_eq!(display_sequence(&seq(&text, WIN), WIN), "Ctrl+N M");
        // Ctrl on a Mac stays Ctrl; the Win key stays the system key
        assert_eq!(sequence_to_text(&[Chord::new("Tab").ctrl()], MAC), "Ctrl+Tab");
        assert_eq!(sequence_to_text(&[Chord::new("E").meta()], WIN), "Meta+E");
        assert_eq!(display_sequence(&seq("Meta+E", MAC), MAC), "Cmd+E");
        assert_eq!(Chord::parse("Cmd+Option+Shift+K", MAC), Some(Chord::new("K").meta().alt().shift()));
        assert_eq!(Chord::parse("Win+Shift+S", WIN), Some(Chord::new("S").meta().shift()));
        assert_eq!(Chord::parse("Hyper+K", MAC), None);
        assert_eq!(parse_sequence("", MAC), Some(vec![]));
        assert_eq!(display_sequence(&seq("Primary+Comma", MAC), MAC), "Cmd+,");
    }

    #[test]
    fn sequences_wait_and_fall_back() {
        let k = Keymap::defaults(MAC);
        let mut m = Matcher::default();
        let cmd_n = Chord::new("N").meta();
        assert_eq!(m.feed(&k, cmd_n.clone(), 0.0), Outcome::Pending);
        assert_eq!(m.continuations(&k).len(), 7);
        assert_eq!(m.feed(&k, Chord::new("M"), 0.3), Outcome::Fire(Command::NewDocument(ModuleKind::Markdown)));
        assert_eq!(m.feed(&k, cmd_n.clone(), 1.0), Outcome::Pending);
        assert_eq!(m.tick(&k, 1.5), None);
        assert_eq!(m.tick(&k, 2.1), Some(Command::NewDocumentPicker));
        assert_eq!(m.feed(&k, cmd_n.clone(), 3.0), Outcome::Pending);
        assert_eq!(m.feed(&k, Chord::new("O").meta(), 3.2), Outcome::Fire(Command::OpenDialog));
        assert!(m.pending().is_empty());
        // on a Mac, Ctrl+N is not Cmd+N
        assert_eq!(m.feed(&k, Chord::new("N").ctrl(), 4.0), Outcome::None);
        assert_eq!(m.feed(&k, Chord::new("Q"), 5.1), Outcome::None);
        // without sequences after it, Cmd+N fires at once
        let mut single = Keymap::defaults(MAC);
        single.bindings.retain(|b| b.keys.len() < 2);
        assert_eq!(m.feed(&single, cmd_n, 6.0), Outcome::Fire(Command::NewDocumentPicker));
    }

    #[test]
    fn stored_round_trip_and_conflicts() {
        for os in [MAC, WIN, LIN] {
            let mut k = Keymap::defaults(os);
            k.bindings.push(Binding { command: Command::CycleTheme, keys: seq("Primary+Shift+D", os) });
            k.bindings.push(Binding { command: Command::Refresh, keys: vec![Chord::new("R").meta().alt()] });
            let stored = k.to_stored(os);
            let back = Keymap::from_stored(stored.iter().map(|(a, b)| (a.as_str(), b.as_str())), os);
            assert_eq!(back, k, "{os:?}");
            let theme = k.bindings.len() - 2;
            let dash = k.bindings.iter().position(|b| b.keys == seq("Primary+Shift+D", os)).unwrap();
            assert_eq!(k.conflict(theme), Some(dash));
        }
        assert_eq!(Keymap::from_stored([("no.such.command", "Primary+K")], MAC).bindings.len(), 0);
    }
}
