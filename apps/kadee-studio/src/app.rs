//! The window's state and the one place every [`Command`] runs.
//!
//! Clicks, menus and shortcuts all end in [`KadeeApp::run`]; views only read the state and send
//! commands, as in KadeeStudio 0.3 ("every action is a Command").

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

use gpui::{
    App, AppContext, ClipboardItem, Context, Entity, FocusHandle, PathPromptOptions, Pixels, Point, ScrollHandle, SharedString,
    Subscription, Task, Window,
};
use kd_ai::{AiHub, AppKind, ChatState, ModelRef};
use kd_commands::{Command, Keymap, Os, Page};
use kd_formats::md::templates::{self, Kind as TemplateKind};
use kd_markdown::{MarkdownEvent, MarkdownView};
use kd_store::{
    DocGroup, DocKind, FileEntry, HomeState, InstalledApp, ModuleKind, OfficeKind, Store, ThemePref, create_new_document,
    create_office_document, list_documents, list_subfolders, now,
};
use kd_ui::{InputEvent, TextInput, Theme};

use crate::keys::{self, RunCommand};

const TOAST: Duration = Duration::from_millis(3200);

/// Columns of the documents table.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SortKey {
    Name,
    Opened,
    Modified,
    Size,
}

/// A pop-up menu and where it opens.
#[derive(Debug, Clone, PartialEq)]
pub enum MenuKind {
    /// The + in the tab strip: new documents.
    New,
    /// A document row's … menu.
    Row(PathBuf),
    /// A folder's … menu in the sidebar.
    Folder { dir: PathBuf, root: Option<bool> },
    /// The model picker of the KaDee AI composer.
    Model,
    /// A drop-down list in Settings.
    Pick(crate::settings_state::Picker),
}

pub struct OpenMenu {
    pub kind: MenuKind,
    pub at: Point<Pixels>,
}

/// A question about files and folders, as GenOffice asks them.
#[derive(Debug, Clone, PartialEq)]
pub enum Dialog {
    NewFolder {
        parent: PathBuf,
    },
    RenameFolder {
        dir: PathBuf,
    },
    RenameFile {
        path: PathBuf,
    },
    DeleteFolder {
        dir: PathBuf,
    },
    TrashFile {
        path: PathBuf,
    },
    Missing {
        path: PathBuf,
    },
    /// Closing a tab with unsaved changes.
    Unsaved {
        path: PathBuf,
    },
    /// File › Save as template: the template's name.
    SaveTemplate {
        kind: TemplateKind,
    },
}

pub use crate::settings_state::SettingsUi;

pub struct KadeeApp {
    pub focus: FocusHandle,
    pub store: Store,
    pub state: HomeState,
    pub keymap: Keymap,
    pub page: Page,
    pub settings: Option<SettingsUi>,
    /// The folder shown on the Folder page.
    pub folder: PathBuf,
    pub folder_docs: Vec<FileEntry>,
    pub folder_subdirs: Vec<PathBuf>,
    /// Open documents; tab 0 is Home.
    pub tabs: Vec<PathBuf>,
    pub active: usize,
    pub closed: Vec<PathBuf>,
    pub ai_pane: bool,
    pub filter: Option<DocGroup>,
    pub sort: SortKey,
    pub sort_desc: bool,
    /// Documents in the documents folder.
    pub docs: Vec<FileEntry>,
    pub installed: Vec<InstalledApp>,
    pub hub: AiHub,
    pub chats: HashMap<String, ChatState>,
    pub ai_scroll: ScrollHandle,
    pub menu: Option<OpenMenu>,
    menu_closed: Option<(MenuKind, Instant)>,
    pub dialog: Option<Dialog>,
    /// The Kadee Markdown editors of open `.md` tabs.
    pub md: HashMap<PathBuf, Entity<MarkdownView>>,
    pub toast: Option<(SharedString, Instant)>,
    // inputs
    pub search: Entity<TextInput>,
    pub composer: Entity<TextInput>,
    pub dialog_input: Entity<TextInput>,
    pub name_input: Entity<TextInput>,
    pub key_input: Entity<TextInput>,
    pub url_input: Entity<TextInput>,
    pub prov_name_input: Entity<TextInput>,
    pub models_input: Entity<TextInput>,
    pub key_visible: bool,
    /// home.json waits while Settings are open (Done or Cancel saves it).
    save_pending: bool,
    /// The chat the composer's text belongs to.
    composer_key: String,
    ai_task: Option<Task<()>>,
    _toast_task: Option<Task<()>>,
    _subscriptions: Vec<Subscription>,
}

impl KadeeApp {
    pub fn new(window: &mut Window, cx: &mut Context<Self>) -> Self {
        let store = Store::platform();
        let (mut state, _first) = store.load_or_init();
        if state.library.prune_starred() {
            let _ = store.save(&state);
        }
        let os = Os::current();
        let keymap = match &state.settings.shortcuts {
            Some(list) => Keymap::from_stored(list.iter().map(|s| (s.command.as_str(), s.keys.as_str())), os),
            None => Keymap::defaults(os),
        };
        keys::bind(&keymap, cx);
        Theme::apply(state.settings.theme, cx);

        let hub = AiHub::load(store.dir().to_path_buf());
        let search = cx.new(|cx| TextInput::new(cx, "Search documents"));
        let composer = cx.new(|cx| TextInput::new(cx, "Ask anything…").multiline());
        let dialog_input = cx.new(|cx| TextInput::new(cx, "Name"));
        let name_input = cx.new(|cx| TextInput::new(cx, "Your name"));
        let key_input = cx.new(|cx| TextInput::new(cx, "Paste your key").masked(true));
        let url_input = cx.new(|cx| TextInput::new(cx, "http://localhost:11434/v1"));
        let prov_name_input = cx.new(|cx| TextInput::new(cx, "My provider"));
        let models_input = cx.new(|cx| TextInput::new(cx, "model-one, model-two  (Test connection lists what the server has)"));
        name_input.update(cx, |i, cx| i.set_text(state.settings.display_name.clone(), cx));

        let subscriptions = vec![
            cx.observe_window_appearance(window, |this, _window, cx| Theme::apply(this.state.settings.theme, cx)),
            cx.subscribe_in(&search, window, |_this, _input, e: &InputEvent, _window, cx| {
                if *e == InputEvent::Changed {
                    cx.notify();
                }
            }),
            cx.subscribe_in(&composer, window, |this, input, e: &InputEvent, window, cx| match e {
                InputEvent::Changed => {
                    let text = input.read(cx).text().to_string();
                    let key = this.composer_key.clone();
                    this.chat(&key).draft = text;
                    cx.notify();
                }
                InputEvent::Submit => this.send_chat(window, cx),
                InputEvent::Escape => window.focus(&this.focus),
            }),
            cx.subscribe_in(&dialog_input, window, |this, _input, e: &InputEvent, window, cx| match e {
                InputEvent::Submit => this.confirm_dialog(window, cx),
                InputEvent::Escape => this.close_dialog(window, cx),
                InputEvent::Changed => {}
            }),
            cx.subscribe_in(&name_input, window, |this, input, e: &InputEvent, _window, cx| {
                if *e == InputEvent::Changed {
                    this.state.settings.display_name = input.read(cx).text().trim().to_string();
                    cx.notify();
                }
            }),
            cx.subscribe_in(&key_input, window, |this, _input, e: &InputEvent, _window, cx| {
                if *e == InputEvent::Changed {
                    this.form_fields_changed(cx);
                }
            }),
            cx.subscribe_in(&url_input, window, |this, _input, e: &InputEvent, _window, cx| {
                if *e == InputEvent::Changed {
                    this.form_fields_changed(cx);
                }
            }),
            cx.subscribe_in(&prov_name_input, window, |this, _input, e: &InputEvent, _window, cx| {
                if *e == InputEvent::Changed {
                    this.form_fields_changed(cx);
                }
            }),
            cx.subscribe_in(&models_input, window, |this, _input, e: &InputEvent, _window, cx| {
                if *e == InputEvent::Changed {
                    this.form_fields_changed(cx);
                }
            }),
        ];

        let folder = state.settings.documents_dir.clone();
        let mut app = KadeeApp {
            focus: cx.focus_handle(),
            store,
            state,
            keymap,
            page: Page::Home,
            settings: None,
            folder,
            folder_docs: Vec::new(),
            folder_subdirs: Vec::new(),
            tabs: Vec::new(),
            active: 0,
            closed: Vec::new(),
            ai_pane: false,
            filter: None,
            sort: SortKey::Modified,
            sort_desc: true,
            docs: Vec::new(),
            installed: Vec::new(),
            hub,
            chats: HashMap::new(),
            ai_scroll: ScrollHandle::new(),
            menu: None,
            menu_closed: None,
            dialog: None,
            md: HashMap::new(),
            toast: None,
            search,
            composer,
            dialog_input,
            name_input,
            key_input,
            url_input,
            prov_name_input,
            models_input,
            key_visible: false,
            save_pending: false,
            composer_key: String::new(),
            ai_task: None,
            _toast_task: None,
            _subscriptions: subscriptions,
        };
        app.refresh();
        app.detect_apps(cx);
        if let Some(page) = start_page() {
            app.page = page;
        }
        app
    }

    // ---- state helpers -----------------------------------------------------------------------

    pub fn shortcut(&self, cmd: &Command) -> Option<String> {
        keys::shortcut_text(&self.keymap, cmd)
    }

    pub fn save(&mut self, cx: &mut Context<Self>) {
        if self.settings.is_some() {
            self.save_pending = true;
            return;
        }
        self.save_pending = false;
        if let Err(e) = self.store.save(&self.state) {
            self.notify(format!("Could not save settings: {e}"), cx);
        }
    }

    pub fn notify(&mut self, msg: impl Into<SharedString>, cx: &mut Context<Self>) {
        self.toast = Some((msg.into(), Instant::now()));
        self._toast_task = Some(cx.spawn(async move |this, cx| {
            cx.background_executor().timer(TOAST).await;
            let _ = this.update(cx, |app, cx| {
                if app.toast.as_ref().is_some_and(|(_, t)| t.elapsed() >= TOAST) {
                    app.toast = None;
                    cx.notify();
                }
            });
        }));
        cx.notify();
    }

    /// Read the documents folder (and the folder on screen) again.
    pub fn refresh(&mut self) {
        let dir = self.state.settings.documents_dir.clone();
        self.docs = list_documents(&dir);
        if self.page == Page::Folder {
            self.folder_docs = list_documents(&self.folder);
            self.folder_subdirs = list_subfolders(&self.folder);
        }
    }

    /// Look for the installed Crafting Apps without holding up the window.
    pub fn detect_apps(&mut self, cx: &mut Context<Self>) {
        let task = cx.background_spawn(async move { kd_store::detect_installed() });
        cx.spawn(async move |this, cx| {
            let found = task.await;
            let _ = this.update(cx, |app, cx| {
                app.installed = found;
                cx.notify();
            });
        })
        .detach();
    }

    /// The roots of the sidebar's folder tree: the documents folder, then the added folders.
    pub fn folder_roots(&self) -> Vec<PathBuf> {
        let mut v = vec![self.state.settings.documents_dir.clone()];
        for f in &self.state.settings.folders {
            if !v.contains(f) {
                v.push(f.clone());
            }
        }
        v
    }

    pub fn is_expanded(&self, dir: &Path) -> bool {
        self.state.settings.expanded_folders.iter().any(|d| d == dir)
    }

    /// The documents of the page on screen, filtered, searched and sorted.
    pub fn visible_entries(&self, cx: &App) -> Vec<FileEntry> {
        let lib = &self.state.library;
        let mut entries: Vec<FileEntry> = match self.page {
            Page::Recent => lib.recent.iter().filter_map(|r| FileEntry::from_path(&r.path)).collect(),
            Page::Starred => lib.starred.iter().filter_map(|p| FileEntry::from_path(p)).collect(),
            Page::Folder => self.folder_docs.clone(),
            _ => {
                // Home: the documents folder plus anything opened from elsewhere.
                let mut v = self.docs.clone();
                for r in &lib.recent {
                    if !v.iter().any(|e| e.path == r.path)
                        && let Some(e) = FileEntry::from_path(&r.path)
                    {
                        v.push(e);
                    }
                }
                v
            }
        };
        if let Some(group) = self.filter {
            entries.retain(|e| e.kind.group == group);
        }
        let q = self.search.read(cx).text().trim().to_lowercase();
        if !q.is_empty() {
            entries.retain(|e| e.name.to_lowercase().contains(&q));
        }
        if self.page != Page::Recent || self.sort != SortKey::Opened {
            let opened = |e: &FileEntry| lib.opened_at(&e.path).unwrap_or(0);
            entries.sort_by(|a, b| {
                let o = match self.sort {
                    SortKey::Name => a.name.to_lowercase().cmp(&b.name.to_lowercase()),
                    SortKey::Opened => opened(a).cmp(&opened(b)),
                    SortKey::Modified => a.modified.cmp(&b.modified),
                    SortKey::Size => a.size.cmp(&b.size),
                };
                if self.sort_desc { o.reverse() } else { o }
            });
        }
        entries
    }

    pub fn set_sort(&mut self, key: SortKey, cx: &mut Context<Self>) {
        if self.sort == key {
            self.sort_desc = !self.sort_desc;
        } else {
            self.sort = key;
            self.sort_desc = key != SortKey::Name;
        }
        cx.notify();
    }

    /// Where a new document goes: the folder on screen, else the documents folder.
    fn target_dir(&self) -> PathBuf {
        if self.page == Page::Folder && self.folder.is_dir() { self.folder.clone() } else { self.state.settings.documents_dir.clone() }
    }

    pub fn installed_app(&self, id: &str) -> Option<&InstalledApp> {
        self.installed.iter().find(|a| a.app.id == id)
    }

    /// The installed Crafting App that edits this kind of document.
    pub fn craft_for(&self, kind: &DocKind) -> Option<&InstalledApp> {
        match kind.opener {
            kd_store::Opener::Craft(a) => self.installed_app(a.id),
            kd_store::Opener::Module(ModuleKind::Markdown) => self.installed_app("mdcraft"),
            kd_store::Opener::Module(ModuleKind::Pdf) => self.installed_app("pdfcraft"),
            _ => None,
        }
    }

    // ---- menus and dialogs ------------------------------------------------------------------

    pub fn open_menu(&mut self, kind: MenuKind, at: Point<Pixels>, cx: &mut Context<Self>) {
        // a click on the button of the menu that just closed (by that same click) closes it
        if let Some((k, t)) = &self.menu_closed
            && *k == kind
            && t.elapsed() < Duration::from_millis(250)
        {
            self.menu_closed = None;
            return;
        }
        self.menu = Some(OpenMenu { kind, at });
        cx.notify();
    }

    pub fn close_menu(&mut self, cx: &mut Context<Self>) {
        if let Some(m) = self.menu.take() {
            self.menu_closed = Some((m.kind, Instant::now()));
        }
        cx.notify();
    }

    fn open_dialog(&mut self, dialog: Dialog, window: &mut Window, cx: &mut Context<Self>) {
        let name = match &dialog {
            Dialog::NewFolder { .. } => Some("New folder".to_string()),
            Dialog::RenameFolder { dir } => Some(file_name(dir)),
            Dialog::RenameFile { path } => Some(path.file_stem().map(|s| s.to_string_lossy().into_owned()).unwrap_or_default()),
            Dialog::SaveTemplate { .. } => Some(
                self.active_markdown()
                    .and_then(|v| v.read(cx).path.clone())
                    .and_then(|p| p.file_stem().map(|s| s.to_string_lossy().into_owned()))
                    .unwrap_or_else(|| "My template".into()),
            ),
            _ => None,
        };
        if let Some(name) = name {
            self.dialog_input.update(cx, |i, cx| {
                i.set_text(name, cx);
            });
            self.dialog_input.read(cx).focus(window);
        } else {
            window.focus(&self.focus);
        }
        self.menu = None;
        self.dialog = Some(dialog);
        cx.notify();
    }

    pub fn close_dialog(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        self.dialog = None;
        window.focus(&self.focus);
        cx.notify();
    }

    pub fn confirm_dialog(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let Some(dialog) = self.dialog.clone() else { return };
        let name = self.dialog_input.read(cx).text().trim().to_string();
        let result: Result<(), String> = match &dialog {
            Dialog::NewFolder { parent } => kd_store::create_folder(parent, &name).map(|_| ()).map_err(|e| e.to_string()),
            Dialog::RenameFolder { dir } => match kd_store::rename_folder(dir, &name) {
                Ok(to) => {
                    self.state.library.rename_dir(dir, &to);
                    for d in self.state.settings.expanded_folders.iter_mut().chain(self.state.settings.folders.iter_mut()) {
                        if let Ok(rest) = d.strip_prefix(dir) {
                            *d = to.join(rest);
                        }
                    }
                    if self.folder.starts_with(dir) {
                        self.folder = to.join(self.folder.strip_prefix(dir).unwrap_or(Path::new("")));
                    }
                    self.save(cx);
                    Ok(())
                }
                Err(e) => Err(e.to_string()),
            },
            Dialog::RenameFile { path } => match kd_store::rename_file(path, &name) {
                Ok(to) => {
                    self.state.library.rename(path, &to);
                    for t in self.tabs.iter_mut() {
                        if t == path {
                            *t = to.clone();
                        }
                    }
                    if let Some(v) = self.md.remove(path) {
                        v.update(cx, |v, _| v.path = Some(to.clone()));
                        self.md.insert(to.clone(), v);
                    }
                    self.save(cx);
                    Ok(())
                }
                Err(e) => Err(e.to_string()),
            },
            Dialog::DeleteFolder { dir } => match trash::delete(dir) {
                Ok(()) => {
                    if self.folder.starts_with(dir) {
                        self.folder = dir.parent().map(Path::to_path_buf).unwrap_or_else(|| self.state.settings.documents_dir.clone());
                    }
                    Ok(())
                }
                Err(e) => Err(e.to_string()),
            },
            Dialog::TrashFile { path } => match trash::delete(path) {
                Ok(()) => {
                    self.state.library.forget(path);
                    if let Some(i) = self.tabs.iter().position(|t| t == path) {
                        self.discard_tab(i + 1, window, cx);
                    }
                    self.save(cx);
                    self.notify(format!("Moved “{}” to the Trash", file_name(path)), cx);
                    Ok(())
                }
                Err(e) => Err(e.to_string()),
            },
            Dialog::Missing { path } => {
                self.state.library.forget(path);
                self.save(cx);
                Ok(())
            }
            Dialog::Unsaved { path } => {
                if let Some(i) = self.tabs.iter().position(|t| t == path) {
                    self.discard_tab(i + 1, window, cx);
                }
                Ok(())
            }
            Dialog::SaveTemplate { kind } => self.save_template(&name, *kind, cx),
        };
        match result {
            Ok(()) => {
                self.refresh();
                self.close_dialog(window, cx);
            }
            Err(e) => self.notify(e, cx),
        }
    }

    // ---- tabs --------------------------------------------------------------------------------

    pub fn open_path(&mut self, path: PathBuf, window: &mut Window, cx: &mut Context<Self>) {
        if !path.exists() {
            self.open_dialog(Dialog::Missing { path }, window, cx);
            return;
        }
        if DocKind::from_path(&path).is_none() {
            self.notify(format!("KadeeStudio does not open “{}” files yet", ext_of(&path)), cx);
            return;
        }
        self.state.library.touch(&path, now());
        self.save(cx);
        if is_markdown(&path) && !self.md.contains_key(&path) {
            match std::fs::read_to_string(&path) {
                Ok(text) => {
                    let view = self.markdown_view(&text, path.clone(), window, cx);
                    self.md.insert(path.clone(), view);
                }
                Err(e) => {
                    self.notify(format!("Could not read “{}”: {e}", file_name(&path)), cx);
                    return;
                }
            }
        }
        if let Some(i) = self.tabs.iter().position(|p| *p == path) {
            self.active = i + 1;
        } else {
            self.tabs.push(path.clone());
            self.active = self.tabs.len();
        }
        self.close_settings(true, cx);
        self.sync_composer(window, cx);
        if let Some(v) = self.md.get(&path) {
            v.read(cx).focus_editor(window, cx);
        }
        cx.notify();
    }

    /// A Kadee Markdown editor for `path`, its events wired to the app.
    fn markdown_view(&mut self, text: &str, path: PathBuf, window: &mut Window, cx: &mut Context<Self>) -> Entity<MarkdownView> {
        let view = cx.new(|cx| MarkdownView::new(text, Some(path), window, cx));
        let sub = cx.subscribe_in(&view, window, |this, _view, e: &MarkdownEvent, window, cx| match e {
            MarkdownEvent::Changed => cx.notify(),
            MarkdownEvent::Toast(m) => this.notify(m.clone(), cx),
            MarkdownEvent::Host(id) => this.markdown_command(id, window, cx),
        });
        self._subscriptions.push(sub);
        view
    }

    /// The editor of the tab on screen, when it is a Markdown document.
    pub fn active_markdown(&self) -> Option<Entity<MarkdownView>> {
        let path = self.tabs.get(self.active.checked_sub(1)?)?;
        self.md.get(path).cloned()
    }

    /// Whether the document in tab `path` has unsaved changes.
    pub fn is_unsaved(&self, path: &Path, cx: &App) -> bool {
        self.md.get(path).is_some_and(|v| v.read(cx).is_dirty(cx))
    }

    /// Commands of Kadee Markdown that the app runs (files, theme, KaDee AI).
    fn markdown_command(&mut self, id: &str, window: &mut Window, cx: &mut Context<Self>) {
        let cmd = match id {
            "file.new" => Command::NewDocumentPicker,
            "file.newBlank" | "file.newDeck" => Command::NewDocument(ModuleKind::Markdown),
            "file.open" => Command::OpenDialog,
            "file.save" => Command::Save,
            "file.saveAs" => Command::SaveAs,
            "file.close" => Command::CloseActiveTab,
            "file.options" => Command::OpenSettings(kd_commands::SettingsSection::General),
            "view.light" | "view.dark" | "view.system" => {
                self.state.settings.theme = match id {
                    "view.light" => ThemePref::Light,
                    "view.dark" => ThemePref::Dark,
                    _ => ThemePref::System,
                };
                Theme::apply(self.state.settings.theme, cx);
                self.save(cx);
                cx.notify();
                return;
            }
            "ai.ask" => {
                self.ai_pane = true;
                self.sync_composer(window, cx);
                self.composer.read(cx).focus(window);
                cx.notify();
                return;
            }
            "help.markdown" => {
                if let Some(t) = templates::built_in("welcome") {
                    self.new_from_template("Markdown guide", t.source, window, cx);
                }
                return;
            }
            "file.saveTemplate" => {
                if let Some(v) = self.active_markdown() {
                    let kind = templates::kind_for(&v.read(cx).text(cx));
                    self.open_dialog(Dialog::SaveTemplate { kind }, window, cx);
                }
                return;
            }
            "template.use" => {
                if let Some(v) = self.active_markdown() {
                    let (text, name) = {
                        let v = v.read(cx);
                        let name = v.path.as_ref().and_then(|p| p.file_stem()).map(|s| s.to_string_lossy().to_string()).unwrap_or_default();
                        (v.text(cx), name)
                    };
                    self.new_from_template(&name, &text, window, cx);
                }
                return;
            }
            "file.info" => {
                if let Some(v) = self.active_markdown() {
                    let text = v.read(cx).text(cx);
                    let words = kd_formats::md::word_count(&kd_formats::md::parse(&text));
                    self.notify(format!("{words} words · {} characters", text.chars().count()), cx);
                }
                return;
            }
            _ => {
                let label = kd_markdown::commands::get(id).map(|c| c.label.trim_end_matches('…')).unwrap_or(id);
                self.notify(format!("{label} comes in a later version of Kadee Markdown."), cx);
                return;
            }
        };
        self.run(cmd, window, cx);
    }

    /// Save the Markdown document on screen; with `as_new` (or never saved), ask where.
    fn save_markdown(&mut self, view: Entity<MarkdownView>, as_new: bool, window: &mut Window, cx: &mut Context<Self>) {
        let current = view.read(cx).path.clone();
        match current {
            Some(path) if !as_new => {
                let text = view.read(cx).text(cx);
                match write_atomic(&path, &text) {
                    Ok(()) => {
                        view.update(cx, |v, cx| v.mark_saved(None, cx));
                        self.state.library.touch(&path, now());
                        self.save(cx);
                        self.refresh();
                        self.notify(format!("Saved “{}”", file_name(&path)), cx);
                    }
                    Err(e) => self.notify(format!("Could not save: {e}"), cx),
                }
            }
            _ => {
                let dir = current.as_ref().and_then(|p| p.parent().map(Path::to_path_buf)).unwrap_or_else(|| self.target_dir());
                let name = current
                    .as_ref()
                    .and_then(|p| p.file_name())
                    .map(|n| n.to_string_lossy().to_string())
                    .unwrap_or_else(|| "Untitled.md".into());
                let rx = cx.prompt_for_new_path(&dir, Some(&name));
                cx.spawn_in(window, async move |this, cx| {
                    let Ok(Ok(Some(path))) = rx.await else { return };
                    let path = if path.extension().is_none() { path.with_extension("md") } else { path };
                    let _ = this.update_in(cx, |this, window, cx| {
                        let text = view.read(cx).text(cx);
                        if let Err(e) = write_atomic(&path, &text) {
                            this.notify(format!("Could not save: {e}"), cx);
                            return;
                        }
                        let old = view.read(cx).path.clone();
                        view.update(cx, |v, cx| v.mark_saved(Some(path.clone()), cx));
                        if let Some(old) = old.filter(|o| *o != path) {
                            this.md.remove(&old);
                            if let Some(t) = this.tabs.iter_mut().find(|t| **t == old) {
                                *t = path.clone();
                            }
                        }
                        this.md.insert(path.clone(), view.clone());
                        this.state.library.touch(&path, now());
                        this.save(cx);
                        this.refresh();
                        this.sync_composer(window, cx);
                        this.notify(format!("Saved “{}”", file_name(&path)), cx);
                    });
                })
                .detach();
            }
        }
    }

    // ---- templates (.mdd documents, .mds slide decks) ----------------------------------------

    /// Where the user's own templates are saved.
    pub fn my_templates_dir(&self) -> PathBuf {
        self.store.dir().join("templates")
    }

    /// The folders templates are listed from: ours, then MdCraft's (read as is).
    pub fn template_dirs(&self) -> Vec<PathBuf> {
        let mut dirs = vec![self.my_templates_dir()];
        if let Some(base) = directories::BaseDirs::new() {
            dirs.push(base.config_dir().join("mdcraft").join("templates"));
        }
        dirs
    }

    pub fn user_templates(&self) -> Vec<templates::UserTemplate> {
        templates::user_templates(&self.template_dirs())
    }

    /// A new document from a template: the placeholders filled in, saved next to the other new
    /// documents and opened.
    pub fn new_from_template(&mut self, name: &str, source: &str, window: &mut Window, cx: &mut Context<Self>) {
        let vars = templates::Vars::today(&self.state.settings.display_name);
        let text = templates::fill(source, &vars);
        let dir = self.target_dir();
        let path = kd_store::unique_path(&dir, &templates::document_name(name), "md");
        match std::fs::create_dir_all(&dir).and_then(|_| std::fs::write(&path, text)) {
            Ok(()) => {
                self.refresh();
                self.open_path(path, window, cx);
            }
            Err(e) => self.notify(format!("Could not create the document: {e}"), cx),
        }
    }

    /// A new template of `kind` in the templates folder, opened to edit.
    pub fn new_template(&mut self, kind: TemplateKind, window: &mut Window, cx: &mut Context<Self>) {
        let id = if kind == TemplateKind::Document { "blank" } else { "blank-deck" };
        let source = templates::built_in(id).map(|t| t.source).unwrap_or("# Title\n");
        let dir = self.my_templates_dir();
        let base = if kind == TemplateKind::Document { "New document template" } else { "New slide deck template" };
        let path = kd_store::unique_path(&dir, base, kind.extension());
        match std::fs::create_dir_all(&dir).and_then(|_| std::fs::write(&path, source)) {
            Ok(()) => self.open_path(path, window, cx),
            Err(e) => self.notify(format!("Could not create the template: {e}"), cx),
        }
    }

    /// File › Save as template: the document on screen into the templates folder.
    fn save_template(&mut self, name: &str, kind: TemplateKind, cx: &mut Context<Self>) -> Result<(), String> {
        let Some(view) = self.active_markdown() else { return Err("Open a Markdown document first".into()) };
        let name = templates::document_name(name);
        let dir = self.my_templates_dir();
        let path = dir.join(format!("{name}.{}", kind.extension()));
        let text = view.read(cx).text(cx);
        std::fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
        write_atomic(&path, &text).map_err(|e| e.to_string())?;
        self.notify(format!("Saved the template “{name}” ({})", kind.label().to_lowercase()), cx);
        Ok(())
    }

    /// KaDee AI's "Insert into document": the answer at the cursor of the Markdown tab on screen.
    pub fn insert_into_document(&mut self, text: &str, window: &mut Window, cx: &mut Context<Self>) {
        if let Some(v) = self.active_markdown() {
            v.update(cx, |v, cx| v.insert_text(text, window, cx));
            self.notify("Inserted into the document", cx);
        }
    }

    fn close_tab(&mut self, n: usize, window: &mut Window, cx: &mut Context<Self>) {
        if n == 0 || n > self.tabs.len() {
            return;
        }
        if self.is_unsaved(&self.tabs[n - 1], cx) {
            let path = self.tabs[n - 1].clone();
            self.active = n;
            self.open_dialog(Dialog::Unsaved { path }, window, cx);
            return;
        }
        self.discard_tab(n, window, cx);
    }

    /// Close tab `n` without asking.
    fn discard_tab(&mut self, n: usize, window: &mut Window, cx: &mut Context<Self>) {
        if n == 0 || n > self.tabs.len() {
            return;
        }
        let path = self.tabs.remove(n - 1);
        self.md.remove(&path);
        if let Some(c) = self.chats.remove(&path.display().to_string()) {
            c.stop();
        }
        self.closed.push(path);
        if self.active >= n {
            self.active = self.active.saturating_sub(1).min(self.tabs.len());
        }
        self.sync_composer(window, cx);
        cx.notify();
    }

    fn activate(&mut self, n: usize, window: &mut Window, cx: &mut Context<Self>) {
        if n <= self.tabs.len() {
            self.active = n;
            self.menu = None;
            self.sync_composer(window, cx);
            cx.notify();
        }
    }

    // ---- KaDee AI ----------------------------------------------------------------------------

    /// The chat of the tab on screen: "" on Home, the file's path in a file tab.
    pub fn chat_key(&self) -> String {
        if self.active == 0 { String::new() } else { self.tabs.get(self.active - 1).map(|p| p.display().to_string()).unwrap_or_default() }
    }

    pub fn chat(&mut self, key: &str) -> &mut ChatState {
        let file = (!key.is_empty()).then(|| PathBuf::from(key));
        self.chats.entry(key.to_string()).or_insert_with(|| {
            let target = file.as_deref().and_then(app_kind_of);
            ChatState::new(target, false)
        })
    }

    /// Put the draft of the chat on screen into the composer.
    fn sync_composer(&mut self, _window: &mut Window, cx: &mut Context<Self>) {
        let key = self.chat_key();
        if key == self.composer_key {
            return;
        }
        self.composer_key = key.clone();
        let draft = self.chat(&key).draft.clone();
        self.composer.update(cx, |i, cx| i.set_text(draft, cx));
    }

    pub fn set_draft(&mut self, text: &str, window: &mut Window, cx: &mut Context<Self>) {
        self.composer.update(cx, |i, cx| i.set_text(text, cx));
        self.composer.read(cx).focus(window);
    }

    /// The model the chat on screen uses: picked in the composer, else Settings' for its scope.
    pub fn chat_model(&mut self) -> Option<ModelRef> {
        let key = self.chat_key();
        let c = self.chat(&key);
        let (picked, scope) = (c.model.clone(), c.scope());
        picked.or_else(|| self.hub.model_for(scope))
    }

    pub fn send_chat(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let key = self.chat_key();
        let file = (!key.is_empty()).then(|| PathBuf::from(&key));
        let editor = self.active_markdown().map(|v| v.read(cx).context_text(cx));
        let context = match (&file, editor) {
            (Some(f), Some((text, selected))) => {
                let whole = self.active_markdown().map(|v| v.read(cx).text(cx)).unwrap_or_default();
                let mut c = kd_ai::text_context(&file_name(f), &whole, kd_ai::CONTEXT_BUDGET);
                if selected {
                    c.push_str(&format!("\nThe user has selected this part of it:\n=== selection ===\n{text}\n"));
                }
                Some(c)
            }
            (Some(f), None) => kd_ai::file_context(f, kd_ai::CONTEXT_BUDGET),
            (None, _) if self.chat(&key).use_starred => kd_ai::starred_context(&self.state.library.starred, kd_ai::CONTEXT_BUDGET),
            (None, _) => None,
        };
        let text = self.composer.read(cx).text().to_string();
        let hub = self.hub.clone();
        let c = self.chat(&key);
        c.draft = text;
        if c.send(&hub, context) {
            self.composer.update(cx, |i, cx| i.clear(cx));
            self.composer.read(cx).focus(window);
            self.ai_scroll.scroll_to_bottom();
            self.start_ai_pump(cx);
        }
        cx.notify();
    }

    /// Take in the running answers while any is streaming.
    fn start_ai_pump(&mut self, cx: &mut Context<Self>) {
        if self.ai_task.is_some() {
            return;
        }
        self.ai_task = Some(cx.spawn(async move |this, cx| {
            loop {
                cx.background_executor().timer(Duration::from_millis(33)).await;
                let running = this
                    .update(cx, |app, cx| {
                        let mut changed = false;
                        for c in app.chats.values_mut() {
                            changed |= c.pump();
                        }
                        if changed {
                            app.ai_scroll.scroll_to_bottom();
                            cx.notify();
                        }
                        let running = app.chats.values().any(|c| c.running());
                        if !running {
                            app.ai_task = None;
                        }
                        running
                    })
                    .unwrap_or(false);
                if !running {
                    break;
                }
            }
        }));
    }

    pub fn stop_chat(&mut self, cx: &mut Context<Self>) {
        let key = self.chat_key();
        self.chat(&key).stop();
        cx.notify();
    }

    pub fn new_chat(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let key = self.chat_key();
        self.chat(&key).clear();
        self.composer.update(cx, |i, cx| i.clear(cx));
        self.composer.read(cx).focus(window);
        cx.notify();
    }

    pub fn pick_model(&mut self, model: Option<ModelRef>, cx: &mut Context<Self>) {
        let key = self.chat_key();
        self.chat(&key).model = model;
        self.close_menu(cx);
    }

    // ---- commands ----------------------------------------------------------------------------

    pub fn on_run_command(&mut self, action: &RunCommand, window: &mut Window, cx: &mut Context<Self>) {
        if let Some(cmd) = Command::from_id(&action.id) {
            self.run(cmd, window, cx);
        }
    }

    pub fn on_dismiss(&mut self, _: &keys::Dismiss, window: &mut Window, cx: &mut Context<Self>) {
        if self.menu.is_some() {
            self.close_menu(cx);
        } else if self.dialog.is_some() {
            self.close_dialog(window, cx);
        } else if self.settings.is_some() {
            self.close_settings(false, cx);
        }
    }

    /// A key on the window: while a shortcut is recorded it goes to the recording.
    pub fn on_key_down(&mut self, e: &gpui::KeyDownEvent, _window: &mut Window, cx: &mut Context<Self>) {
        if self.is_recording() {
            self.record_key(e, cx);
        }
    }

    /// Run one command.
    pub fn run(&mut self, cmd: Command, window: &mut Window, cx: &mut Context<Self>) {
        self.menu = None;
        match cmd {
            Command::NewDocumentPicker => self.show_page(Page::New, window, cx),
            Command::NewDocument(ModuleKind::Pdf) | Command::OpenModule(ModuleKind::Pdf) => self.prompt_open(window, cx),
            Command::NewDocument(kind) => match create_new_document(kind, &self.target_dir()) {
                Ok(path) => {
                    self.refresh();
                    self.open_path(path, window, cx);
                }
                Err(e) => self.notify(format!("Could not create the document: {e}"), cx),
            },
            Command::NewOffice(kind) => match create_office_document(kind, &self.target_dir()) {
                Ok(path) => {
                    self.refresh();
                    self.open_path(path, window, cx);
                }
                Err(e) => self.notify(format!("Could not create the {}: {e}", kind.short().to_lowercase()), cx),
            },
            Command::OpenModule(kind) => self.notify(format!("{} arrives in Phase {}", kind.name(), kind.phase()), cx),
            Command::OpenDialog => self.prompt_open(window, cx),
            Command::OpenPath(path) => self.open_path(path, window, cx),
            Command::Save | Command::SaveAs => match self.active_markdown() {
                Some(v) => self.save_markdown(v, cmd == Command::SaveAs, window, cx),
                None => self.notify("Nothing to save here: open a Markdown document to edit it", cx),
            },
            Command::Print => self.notify("Printing comes in a later version", cx),
            Command::DeleteFile(path) => {
                let d = if path.exists() { Dialog::TrashFile { path } } else { Dialog::Missing { path } };
                self.open_dialog(d, window, cx)
            }
            Command::RenameFile(path) => self.open_dialog(Dialog::RenameFile { path }, window, cx),
            Command::DuplicateFile(path) => match kd_store::duplicate_file(&path) {
                Ok(copy) => {
                    self.refresh();
                    self.notify(format!("Made “{}”", file_name(&copy)), cx);
                }
                Err(e) => self.notify(format!("Could not duplicate: {e}"), cx),
            },
            Command::PinFolder => self.prompt_pin_folder(cx),
            Command::UnpinFolder(dir) => {
                self.state.settings.folders.retain(|d| *d != dir);
                if self.page == Page::Folder && self.folder.starts_with(&dir) {
                    self.page = Page::Home;
                }
                self.save(cx);
                cx.notify();
            }
            Command::ShowFolder(dir) => {
                self.folder = dir;
                self.show_page(Page::Folder, window, cx);
            }
            Command::ToggleFolder(dir) => {
                let ex = &mut self.state.settings.expanded_folders;
                if let Some(i) = ex.iter().position(|d| *d == dir) {
                    ex.remove(i);
                } else {
                    ex.push(dir);
                }
                self.save(cx);
                cx.notify();
            }
            Command::NewFolder(parent) => self.open_dialog(Dialog::NewFolder { parent }, window, cx),
            Command::RenameFolder(dir) => self.open_dialog(Dialog::RenameFolder { dir }, window, cx),
            Command::DeleteFolder(dir) => self.open_dialog(Dialog::DeleteFolder { dir }, window, cx),
            Command::ShowPage(Page::Ai) => {
                self.active = 0;
                self.close_settings(true, cx);
                self.ai_pane = true;
                self.sync_composer(window, cx);
                self.composer.read(cx).focus(window);
                cx.notify();
            }
            Command::ShowPage(page) => self.show_page(page, window, cx),
            Command::OpenSettings(section) => {
                self.open_settings(section, window, cx);
                self.sync_composer(window, cx);
            }
            Command::ToggleAiPane => {
                self.ai_pane = !self.ai_pane;
                if self.ai_pane {
                    self.sync_composer(window, cx);
                    self.composer.read(cx).focus(window);
                } else {
                    window.focus(&self.focus);
                }
                cx.notify();
            }
            Command::LaunchCraft(id) => match self.installed_app(&id).cloned() {
                Some(app) => match app.launch(None) {
                    Ok(()) => self.notify(format!("Opening {}…", app.app.name), cx),
                    Err(e) => self.notify(format!("Could not open {}: {e}", app.app.name), cx),
                },
                None => self.notify("That app is not installed on this computer", cx),
            },
            Command::ActivateTab(n) => self.activate(n, window, cx),
            Command::NextTab => {
                let n = self.tabs.len() + 1;
                self.activate((self.active + 1) % n, window, cx)
            }
            Command::PreviousTab => {
                let n = self.tabs.len() + 1;
                self.activate((self.active + n - 1) % n, window, cx)
            }
            Command::CloseTab(n) => self.close_tab(n, window, cx),
            Command::CloseActiveTab => {
                if self.active > 0 {
                    self.close_tab(self.active, window, cx)
                } else if self.settings.is_some() {
                    self.close_settings(true, cx);
                }
            }
            Command::ReopenClosedTab => {
                if let Some(p) = self.closed.pop() {
                    self.open_path(p, window, cx);
                }
            }
            Command::ToggleStar(path) => {
                let on = self.state.library.toggle_star(&path);
                self.save(cx);
                self.notify(if on { "Added to Starred" } else { "Removed from Starred" }, cx);
            }
            Command::RemoveFromRecent(path) => {
                if self.page == Page::Starred {
                    self.state.library.toggle_star(&path);
                } else {
                    self.state.library.remove_recent(&path);
                }
                self.save(cx);
                cx.notify();
            }
            Command::OpenExternally(path) => cx.open_with_system(&path),
            Command::RevealInFolder(path) => cx.reveal_path(&path),
            Command::CopyPath(path) => {
                cx.write_to_clipboard(ClipboardItem::new_string(path.display().to_string()));
                self.notify("Path copied", cx);
            }
            Command::SetTheme(t) => self.set_theme(t, cx),
            Command::CycleTheme => self.set_theme(self.state.settings.theme.cycle(), cx),
            Command::OpenUrl(url) => cx.open_url(&url),
            Command::Refresh => {
                self.refresh();
                self.detect_apps(cx);
                self.notify("Refreshed", cx);
            }
        }
    }

    fn show_page(&mut self, page: Page, window: &mut Window, cx: &mut Context<Self>) {
        self.active = 0;
        self.close_settings(true, cx);
        self.page = page;
        if page == Page::Folder {
            self.folder_docs = list_documents(&self.folder);
            self.folder_subdirs = list_subfolders(&self.folder);
        }
        self.sync_composer(window, cx);
        cx.notify();
    }

    fn set_theme(&mut self, t: ThemePref, cx: &mut Context<Self>) {
        self.state.settings.theme = t;
        Theme::apply(t, cx);
        self.save(cx);
        cx.notify();
    }

    fn prompt_open(&mut self, _window: &mut Window, cx: &mut Context<Self>) {
        let rx = cx.prompt_for_paths(PathPromptOptions { files: true, directories: false, multiple: true, prompt: Some("Open".into()) });
        cx.spawn_in(_window, async move |this, cx| {
            if let Ok(Ok(Some(paths))) = rx.await {
                let _ = this.update_in(cx, |app, window, cx| {
                    for p in paths {
                        app.open_path(p, window, cx);
                    }
                });
            }
        })
        .detach();
    }

    fn prompt_pin_folder(&mut self, cx: &mut Context<Self>) {
        let rx =
            cx.prompt_for_paths(PathPromptOptions { files: false, directories: true, multiple: false, prompt: Some("Add folder".into()) });
        cx.spawn(async move |this, cx| {
            if let Ok(Ok(Some(paths))) = rx.await {
                let _ = this.update(cx, |app, cx| {
                    for p in paths {
                        if !app.folder_roots().contains(&p) {
                            app.state.settings.folders.push(p);
                        }
                    }
                    app.save(cx);
                    cx.notify();
                });
            }
        })
        .detach();
    }

    /// Settings › General › Documents folder › Change…
    pub fn prompt_documents_dir(&mut self, cx: &mut Context<Self>) {
        let rx = cx.prompt_for_paths(PathPromptOptions {
            files: false,
            directories: true,
            multiple: false,
            prompt: Some("Use this folder".into()),
        });
        cx.spawn(async move |this, cx| {
            if let Ok(Ok(Some(mut paths))) = rx.await
                && let Some(p) = paths.pop()
            {
                let _ = this.update(cx, |app, cx| {
                    app.state.settings.documents_dir = p;
                    app.refresh();
                    app.save(cx);
                    cx.notify();
                });
            }
        })
        .detach();
    }
}

/// `--page=recent` and friends open a page at start (screenshots, docs, tests).
fn start_page() -> Option<Page> {
    let arg = std::env::args().find_map(|a| a.strip_prefix("--page=").map(str::to_string))?;
    Some(match arg.as_str() {
        "new" => Page::New,
        "recent" => Page::Recent,
        "starred" => Page::Starred,
        "craft" | "craft-apps" => Page::CraftApps,
        _ => Page::Home,
    })
}

/// The module a file belongs to for KaDee AI.
pub fn app_kind_of(path: &Path) -> Option<AppKind> {
    let ext = path.extension()?.to_str()?.to_ascii_lowercase();
    match ext.as_str() {
        "docx" | "doc" | "odt" | "rtf" => return Some(AppKind::Word),
        "xlsx" | "xls" | "ods" | "csv" => return Some(AppKind::Sheet),
        "pptx" | "ppt" | "odp" => return Some(AppKind::Slides),
        _ => {}
    }
    match DocKind::from_path(path)?.group {
        DocGroup::Markdown => Some(AppKind::Markdown),
        DocGroup::Pdf => Some(AppKind::Pdf),
        DocGroup::Dashboard => Some(AppKind::Dashboard),
        DocGroup::Report => Some(AppKind::Report),
        _ => None,
    }
}

/// Whether Kadee Markdown edits `path`.
pub fn is_markdown(path: &Path) -> bool {
    DocKind::from_path(path).is_some_and(|k| k.opener == kd_store::Opener::Module(ModuleKind::Markdown))
}

/// Write `text` to `path` through a temporary file next to it, so a failed write never leaves
/// half a document.
pub fn write_atomic(path: &Path, text: &str) -> std::io::Result<()> {
    let dir = path.parent().filter(|d| !d.as_os_str().is_empty()).unwrap_or(Path::new("."));
    let name = path.file_name().map(|n| n.to_string_lossy().to_string()).unwrap_or_else(|| "document".into());
    let tmp = dir.join(format!(".{name}.kadee-tmp"));
    std::fs::write(&tmp, text)?;
    std::fs::rename(&tmp, path).inspect_err(|_| {
        let _ = std::fs::remove_file(&tmp);
    })
}

pub fn file_name(p: &Path) -> String {
    p.file_name().map(|n| n.to_string_lossy().into_owned()).unwrap_or_else(|| p.display().to_string())
}

fn ext_of(p: &Path) -> String {
    p.extension().map(|e| format!(".{}", e.to_string_lossy())).unwrap_or_else(|| file_name(p))
}

/// What "New …" calls an Office kind.
pub fn office_label(kind: OfficeKind) -> &'static str {
    match kind {
        OfficeKind::Word => "Word document",
        OfficeKind::Spreadsheet => "spreadsheet",
        OfficeKind::SlideDeck => "slide deck",
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn files_map_to_ai_modules() {
        assert_eq!(app_kind_of(Path::new("a.md")), Some(AppKind::Markdown));
        assert_eq!(app_kind_of(Path::new("b.xlsx")), Some(AppKind::Sheet));
        assert_eq!(app_kind_of(Path::new("c.kdash")), Some(AppKind::Dashboard));
        assert_eq!(app_kind_of(Path::new("d.psd")), None);
    }
}
