//! Kadee Markdown's window content, laid out as MdCraft's: the tab strip with command search
//! and quick-access buttons, the ribbon, the navigation pane, the editor and the preview side
//! by side, and the status bar.
//!
//! Every button runs a command by id ([`MarkdownView::run`]). File commands, the theme and
//! KaDee AI belong to the host app, which gets them as [`MarkdownEvent::Host`].

use std::path::PathBuf;

use gpui::{
    AnyElement, App, ClickEvent, Context, Entity, EventEmitter, FocusHandle, Focusable, FontWeight, Hsla, IntoElement, MouseButton,
    PathPromptOptions, Pixels, Point, SharedString, Subscription, Window, anchored, deferred, div, point, prelude::*, px, relative, svg,
};
use kd_doc::Selection;
use kd_formats::md::{self, Edit, LineKind, library};
use kd_store::ThemePref;
use kd_ui::{Icon, InputEvent, Palette, TextInput, Theme, mix, tooltip};

use crate::commands::{self, CONTEXT, RunMd};
use crate::editor::{EditorEvent, SourceEditor};
use crate::preview::{Preview, PreviewEvent};
use crate::ribbon::{self, Btn, Entry, FontTarget, Gallery, Item, Menu, RowItem, Size, TabId};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ViewMode {
    Write,
    Split,
    Read,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum MarkdownEvent {
    /// The text changed (the tab shows it is unsaved).
    Changed,
    /// A command for the host: `file.*`, `view.light` / `view.dark` / `view.system`, `ai.ask`.
    Host(&'static str),
    /// A short message for the host's toast.
    Toast(String),
    /// File › Open… or Open Recent: open this file.
    OpenPath(PathBuf),
    /// File › New…: a new document from this template.
    NewFromTemplate { name: String, source: String },
}

/// The colours MdCraft's two-tone icons use.
fn icon_colors(pal: &Palette) -> (Hsla, Hsla) {
    (pal.text_muted.into(), pal.accent.into())
}

/// An MdCraft icon: its stroke part and its accent part on top.
pub fn md_icon(name: &str, size: f32, color: Hsla, accent: Hsla) -> AnyElement {
    let mut d = div()
        .relative()
        .flex_none()
        .size(px(size))
        .child(svg().absolute().top_0().left_0().path(commands::icon_path(name)).size(px(size)).text_color(color));
    if let Some(a) = commands::accent_path(name) {
        d = d.child(svg().absolute().top_0().left_0().path(a).size(px(size)).text_color(accent));
    }
    d.into_any_element()
}

/// Commands that MdCraft has and Kadee Markdown draws later.
fn later(id: &str) -> bool {
    matches!(id, "review.spelling" | "view.mathEngine" | "view.runCode")
}

/// An open menu and where it opens (window coordinates).
struct OpenMenu {
    menu: Menu,
    at: Point<Pixels>,
}

pub struct MarkdownView {
    focus: FocusHandle,
    pub editor: Entity<SourceEditor>,
    pub preview: Entity<Preview>,
    search: Entity<TextInput>,
    nav_filter: Entity<TextInput>,
    /// The file, when the document has been saved.
    pub path: Option<PathBuf>,
    tab: TabId,
    pub mode: ViewMode,
    /// The panel open on the left: Find and replace, Comments, Navigation, Help.
    side: Option<side::Side>,
    comment_input: Entity<TextInput>,
    adding_comment: bool,
    /// The writer's name, for comments (set by the host).
    pub author: String,
    /// The panel open on the right (Paragraph settings).
    right: Option<para::RightPanel>,
    para_ui: para::ParaUi,
    /// File › Properties.
    props_ui: props::PropsUi,
    /// Colours picked lately (the colour pickers' Recent colors).
    recent_colors: Vec<u32>,
    /// More colors…: the colour being made.
    more_colors: Option<color::ColorDialog>,
    nav_slides: bool,
    sync: bool,
    collapsed: bool,
    peek: bool,
    focus_mode: bool,
    zoom: f32,
    menu: Option<OpenMenu>,
    table_hover: (usize, usize),
    // Find and Replace
    find: Option<panels::FindState>,
    find_input: Entity<TextInput>,
    replace_input: Entity<TextInput>,
    // a question (Go to line, Link, Watermark …)
    prompt: Option<panels::PromptKind>,
    prompt_input: Entity<TextInput>,
    /// Accessibility results, the shortcuts …
    info: Option<panels::Info>,
    /// The slide show: the slide on screen.
    present: Option<usize>,
    present_focus: FocusHandle,
    present_fullscreen: bool,
    font_filter: Entity<TextInput>,
    /// The editor's share of the split view, and the main area (for dragging the divider).
    split: f32,
    dragging: bool,
    main_bounds: std::rc::Rc<std::cell::Cell<gpui::Bounds<Pixels>>>,
    /// Set by the host: the last opened Markdown files, the template folders and the
    /// documents folder (for Open Recent, New… and Open…).
    pub recent: Vec<PathBuf>,
    pub template_dirs: Vec<PathBuf>,
    pub documents_dir: Option<PathBuf>,
    /// File › New…: the template chooser is open.
    chooser: bool,
    /// File › Open…: the file browser.
    browser: Option<files::Browser>,
    _subs: Vec<Subscription>,
}

mod files;

mod color;
mod panels;
mod para;
mod props;
mod side;
pub use panels::bind_present_keys;

impl EventEmitter<MarkdownEvent> for MarkdownView {}

impl Focusable for MarkdownView {
    fn focus_handle(&self, _: &App) -> FocusHandle {
        self.focus.clone()
    }
}

impl MarkdownView {
    pub fn new(text: &str, path: Option<PathBuf>, window: &mut Window, cx: &mut Context<Self>) -> Self {
        let editor = cx.new(|cx| SourceEditor::new(text, cx));
        let preview = cx.new(Preview::new);
        let search = cx.new(|cx| TextInput::new(cx, "Search commands (Alt+Q)"));
        let nav_filter = cx.new(|cx| TextInput::new(cx, "Search the document"));
        let find_input = cx.new(|cx| TextInput::new(cx, "Find"));
        let replace_input = cx.new(|cx| TextInput::new(cx, "Replace with"));
        let prompt_input = cx.new(|cx| TextInput::new(cx, ""));
        let comment_input = cx.new(|cx| TextInput::new(cx, "Write a comment"));
        let font_filter = cx.new(|cx| TextInput::new(cx, "Type to find a font"));
        let base = path.as_ref().and_then(|p| p.parent().map(|d| d.to_path_buf()));
        preview.update(cx, |p, cx| {
            p.base_dir = base;
            p.update_text(text, 0, cx);
        });
        let subs = vec![
            cx.subscribe_in(&editor, window, |this, _, e: &EditorEvent, _window, cx| this.on_editor(e, cx)),
            cx.subscribe_in(&preview, window, |this, _, e: &PreviewEvent, window, cx| match e {
                PreviewEvent::GoToLine(line) => {
                    let line = *line;
                    this.editor.update(cx, |ed, cx| ed.go_to_line(line, window, cx));
                }
                PreviewEvent::OpenFile(path) => {
                    // Markdown opens in a tab here, anything else in its own app
                    let ext = path.extension().and_then(|e| e.to_str()).unwrap_or("").to_ascii_lowercase();
                    if matches!(ext.as_str(), "md" | "markdown" | "mdd" | "mds" | "mdp") {
                        cx.emit(MarkdownEvent::OpenPath(path.clone()));
                    } else {
                        cx.open_with_system(path);
                    }
                }
            }),
            cx.subscribe_in(&search, window, |this, input, e: &InputEvent, window, cx| match e {
                InputEvent::Changed => cx.notify(),
                InputEvent::Submit => {
                    let q = input.read(cx).text().to_string();
                    if let Some(c) = search_commands(&q).first() {
                        input.update(cx, |i, cx| i.clear(cx));
                        this.run(c.id, None, window, cx);
                    }
                }
                InputEvent::Escape => {
                    input.update(cx, |i, cx| i.clear(cx));
                    this.focus_editor(window, cx);
                }
            }),
            cx.subscribe(&nav_filter, |_, _, _: &InputEvent, cx| cx.notify()),
            cx.subscribe_in(&find_input, window, |this, _, e: &InputEvent, window, cx| this.on_find_input(e, window, cx)),
            cx.subscribe_in(&replace_input, window, |this, _, e: &InputEvent, window, cx| match e {
                InputEvent::Submit => this.replace_one(window, cx),
                InputEvent::Escape => this.close_find(window, cx),
                InputEvent::Changed => {}
            }),
            cx.subscribe_in(&comment_input, window, |this, _, e: &InputEvent, window, cx| match e {
                InputEvent::Submit => this.add_comment(window, cx),
                InputEvent::Escape => {
                    this.adding_comment = false;
                    this.focus_editor(window, cx);
                    cx.notify();
                }
                InputEvent::Changed => {}
            }),
            cx.subscribe_in(&prompt_input, window, |this, _, e: &InputEvent, window, cx| match e {
                InputEvent::Submit => this.submit_prompt(window, cx),
                InputEvent::Escape => this.close_prompt(window, cx),
                InputEvent::Changed => {}
            }),
            cx.subscribe_in(&font_filter, window, |this, input, e: &InputEvent, window, cx| match e {
                InputEvent::Changed => cx.notify(),
                InputEvent::Submit => this.pick_first_font(input.read(cx).text().to_string(), window, cx),
                InputEvent::Escape => {
                    this.menu = None;
                    this.focus_editor(window, cx);
                    cx.notify();
                }
            }),
        ];
        let (para_ui, para_subs) = para::ParaUi::new(window, cx);
        let (props_ui, props_subs) = props::PropsUi::new(window, cx);
        let mut subs = subs;
        subs.extend(para_subs);
        subs.extend(props_subs);
        let view = MarkdownView {
            focus: cx.focus_handle(),
            editor,
            preview,
            search,
            nav_filter,
            path,
            tab: TabId::Home,
            mode: ViewMode::Split,
            side: None,
            comment_input,
            adding_comment: false,
            author: String::new(),
            right: None,
            para_ui,
            props_ui,
            recent_colors: Vec::new(),
            more_colors: None,
            nav_slides: false,
            sync: true,
            collapsed: false,
            peek: false,
            focus_mode: false,
            zoom: 1.0,
            menu: None,
            table_hover: (2, 2),
            find: None,
            find_input,
            replace_input,
            prompt: None,
            prompt_input,
            info: None,
            present: None,
            present_focus: cx.focus_handle(),
            present_fullscreen: false,
            font_filter,
            split: 0.5,
            dragging: false,
            main_bounds: std::rc::Rc::default(),
            recent: Vec::new(),
            template_dirs: Vec::new(),
            documents_dir: None,
            chooser: false,
            browser: None,
            _subs: subs,
        };
        view.focus_editor(window, cx);
        view
    }

    // ---- for the host ----------------------------------------------------------------------

    pub fn text(&self, cx: &App) -> String {
        self.editor.read(cx).text().to_string()
    }

    /// File › Document info: the file, its dates and what the document holds.
    pub fn document_info(&self, cx: &App) -> Vec<(&'static str, String)> {
        let text = self.text(cx);
        let parsed = md::parse(&text);
        let mut rows: Vec<(&'static str, String)> = Vec::new();
        let ext =
            self.path.as_ref().and_then(|p| p.extension()).map(|e| e.to_string_lossy().to_ascii_lowercase()).unwrap_or_else(|| "md".into());
        let kind = match ext.as_str() {
            "mdd" => "Markdown document template (.mdd)",
            "mds" | "mdp" => "Markdown slide deck template",
            "markdown" => "Markdown document (.markdown)",
            _ => "Markdown document (.md)",
        };
        match &self.path {
            Some(p) => {
                rows.push(("Name", p.file_name().map(|n| n.to_string_lossy().to_string()).unwrap_or_default()));
                rows.push(("Type", kind.to_string()));
                rows.push(("Location", p.parent().map(tilde).unwrap_or_default()));
                if let Ok(meta) = std::fs::metadata(p) {
                    let when = |t: std::io::Result<std::time::SystemTime>| {
                        t.ok().map(|t| chrono::DateTime::<chrono::Local>::from(t).format("%-d %B %Y, %H:%M").to_string())
                    };
                    rows.push(("Size", human_size(meta.len())));
                    if let Some(c) = when(meta.created()) {
                        rows.push(("Created", c));
                    }
                    if let Some(m) = when(meta.modified()) {
                        rows.push(("Modified", m));
                    }
                }
            }
            None => {
                rows.push(("Name", "Untitled".into()));
                rows.push(("Type", kind.to_string()));
            }
        }
        rows.push((
            "Saved",
            if self.path.is_none() {
                "Not saved yet".into()
            } else if self.is_dirty(cx) {
                "Unsaved changes".into()
            } else {
                "Saved".into()
            },
        ));
        for (key, label) in [("title", "Title"), ("author", "Author"), ("date", "Date"), ("theme", "Theme")] {
            if let Some((_, v)) = parsed.front_matter.iter().find(|(k, _)| k == key)
                && !v.trim().is_empty()
            {
                rows.push((label, v.trim().trim_matches('"').to_string()));
            }
        }
        let words = md::word_count(&parsed);
        rows.push(("Words", format!("{words}  ·  about {} min to read", words.div_ceil(230).max(1))));
        rows.push((
            "Characters",
            format!("{} ({} without spaces)", text.chars().count(), text.chars().filter(|c| !c.is_whitespace()).count()),
        ));
        rows.push(("Lines", text.lines().count().to_string()));
        let mut c = Counts::default();
        c.walk(&parsed.blocks);
        rows.push(("Headings", md::outline(&parsed).len().to_string()));
        let slides = md::slides::slides(&text).len();
        if slides > 0 {
            rows.push(("Slides", slides.to_string()));
        }
        let mut contents = Vec::new();
        for (n, one, many) in [
            (c.tables, "table", "tables"),
            (c.pictures, "picture", "pictures"),
            (c.media, "video or audio", "videos or audio"),
            (c.diagrams, "chart or diagram", "charts and diagrams"),
            (c.links, "link", "links"),
            (c.includes, "included file", "included files"),
        ] {
            if n > 0 {
                contents.push(format!("{n} {}", if n == 1 { one } else { many }));
            }
        }
        rows.push(("Contains", if contents.is_empty() { "Text only".into() } else { contents.join(", ") }));
        rows
    }

    pub fn is_dirty(&self, cx: &App) -> bool {
        self.editor.read(cx).doc.is_dirty()
    }

    /// After saving: the file and a clean state.
    pub fn mark_saved(&mut self, path: Option<PathBuf>, cx: &mut Context<Self>) {
        if let Some(p) = path {
            let base = p.parent().map(|d| d.to_path_buf());
            self.preview.update(cx, |pv, _| pv.base_dir = base);
            self.path = Some(p);
        }
        self.editor.update(cx, |ed, _| ed.doc.mark_saved());
        cx.notify();
    }

    /// Insert text at the cursor (KaDee AI's "Insert into document").
    pub fn insert_text(&mut self, text: &str, window: &mut Window, cx: &mut Context<Self>) {
        let text = text.replace('\u{1}', "");
        self.editor.update(cx, |ed, cx| {
            let e = md::edit::insert(ed.text(), ed.selection(), &text);
            ed.apply_edit(e, cx);
        });
        self.focus_editor(window, cx);
    }

    pub fn focus_editor(&self, window: &mut Window, cx: &App) {
        self.editor.read(cx).focus(window);
    }

    /// The selected text, or the whole document when nothing is selected (KaDee AI's context).
    pub fn context_text(&self, cx: &App) -> (String, bool) {
        let ed = self.editor.read(cx);
        let sel = ed.selected_text();
        if sel.trim().is_empty() { (ed.text().to_string(), false) } else { (sel.to_string(), true) }
    }

    fn on_editor(&mut self, e: &EditorEvent, cx: &mut Context<Self>) {
        let (text, version, line) = {
            let ed = self.editor.read(cx);
            (ed.text().to_string(), ed.doc.version(), ed.doc.line_of(ed.selection().head) + 1)
        };
        if *e == EditorEvent::Changed {
            self.preview.update(cx, |p, cx| p.update_text(&text, version, cx));
            cx.emit(MarkdownEvent::Changed);
            if self.find.is_some() {
                self.refresh_matches(cx);
            }
        }
        if self.sync && self.mode == ViewMode::Split {
            self.preview.read(cx).reveal_line(line, false);
        }
        if self.tab == TabId::Table && !self.in_table(cx) {
            self.tab = TabId::Home;
        }
        cx.notify();
    }

    fn in_table(&self, cx: &App) -> bool {
        let ed = self.editor.read(cx);
        md::edit::table_at(ed.text(), ed.selection().head).is_some()
    }

    fn fm(&self, key: &str, cx: &App) -> Option<String> {
        md::edit::front_matter(self.editor.read(cx).text()).into_iter().find(|(k, _)| k == key).map(|(_, v)| v)
    }

    fn toast(&self, msg: impl Into<String>, cx: &mut Context<Self>) {
        cx.emit(MarkdownEvent::Toast(msg.into()));
    }

    /// Apply an edit computed from the text and the selection.
    fn edit(&mut self, cx: &mut Context<Self>, f: impl FnOnce(&str, Selection) -> Option<Edit>) {
        self.editor.update(cx, |ed, cx| {
            if let Some(e) = f(ed.text(), ed.selection()) {
                ed.apply_edit(e, cx);
            }
        });
    }

    fn block(&mut self, block: String, cx: &mut Context<Self>) {
        self.edit(cx, |t, s| Some(md::edit::insert_block(t, s, &block)));
    }

    fn wrap(&mut self, mark: &str, close: Option<&str>, placeholder: Option<&str>, cx: &mut Context<Self>) {
        self.edit(cx, |t, s| Some(md::edit::toggle_wrap(t, s, mark, close, placeholder)));
    }

    fn set_fm(&mut self, key: &str, value: Option<&str>, cx: &mut Context<Self>) {
        self.edit(cx, |t, s| Some(md::edit::set_front_matter(t, s, &[(key, value)])));
    }

    fn set_zoom(&mut self, z: f32, cx: &mut Context<Self>) {
        self.zoom = (z * 10.0).round().clamp(5.0, 20.0) / 10.0;
        let z = self.zoom;
        self.preview.update(cx, |p, cx| {
            p.zoom = z;
            cx.notify();
        });
        self.editor.update(cx, |ed, cx| {
            ed.font_size = px(15.0 * z);
            cx.notify();
        });
    }

    // ---- commands --------------------------------------------------------------------------

    /// Run a command by its MdCraft id, with an optional argument (a menu choice).
    pub fn run(&mut self, id: &str, arg: Option<String>, window: &mut Window, cx: &mut Context<Self>) {
        self.menu = None;
        self.peek = false;
        let Some(cmd) = commands::get(id) else { return };
        let id = cmd.id;
        if later(id) {
            self.toast(format!("{} comes in a later version of Kadee Markdown.", cmd.label.trim_end_matches('…')), cx);
            cx.notify();
            return;
        }
        let ed = self.editor.clone();
        match id {
            // the host's
            "file.newBlank" | "file.newDeck" | "file.save" | "file.saveAs" | "file.saveTemplate" | "template.use" | "file.close"
            | "file.clearRecent" | "file.options" | "view.light" | "view.dark" | "view.system" | "ai.ask" | "help.markdown" => {
                cx.emit(MarkdownEvent::Host(id))
            }
            id if id.starts_with("para.") => self.para_command(id, arg, window, cx),
            "file.properties" => self.open_props(arg.as_deref(), window, cx),
            "file.print" | "file.exportPdf" => self.print(window, cx),
            "file.exportHtml" => self.export_html(window, cx),
            "file.copyRich" => {
                let html = self.html(false, cx);
                cx.write_to_clipboard(gpui::ClipboardItem::new_string(html));
                self.toast("Copied as HTML: paste it into an e-mail or web editor's source view", cx);
            }
            "file.backstage" => {
                self.tab = TabId::File;
                if self.collapsed {
                    self.peek = true;
                }
            }
            "file.new" => self.chooser = true,
            "file.open" => self.open_browser(cx),
            "file.recent" => self.open_menu_at(Menu::Recent, window.mouse_position(), window, cx),
            "file.export" => self.open_menu_at(Menu::Export, window.mouse_position(), window, cx),
            "file.saveAsFormat" => match arg.as_deref() {
                Some("md") | None => cx.emit(MarkdownEvent::Host("file.saveAs")),
                Some(ext) => {
                    let ext = ext.to_string();
                    self.save_copy_as(&ext, window, cx);
                }
            },

            "edit.undo" => ed.update(cx, |e, cx| e.undo(cx)),
            "edit.redo" => ed.update(cx, |e, cx| e.redo(cx)),
            "edit.cut" => ed.update(cx, |e, cx| e.cut_selection(cx)),
            "edit.copy" => ed.update(cx, |e, cx| e.copy_selection(cx)),
            "edit.paste" => ed.update(cx, |e, cx| e.paste_clipboard(cx)),
            "edit.selectAll" => ed.update(cx, |e, cx| e.select_everything(cx)),

            "format.bold" => self.wrap("**", None, Some("bold text"), cx),
            "format.italic" => self.wrap("*", None, Some("italic text"), cx),
            "format.underline" => self.wrap("<u>", Some("</u>"), Some("underlined text"), cx),
            "format.strike" => self.wrap("~~", None, None, cx),
            "format.code" => self.wrap("`", None, Some("code"), cx),
            "format.highlight" => self.wrap("==", None, Some("highlighted text"), cx),
            "format.sub" => self.wrap("~", None, None, cx),
            "format.sup" => self.wrap("^", None, None, cx),
            "format.color" => {
                let c = arg.unwrap_or_else(|| "#C00000".into());
                self.edit(cx, |t, s| md::style::apply_style(t, s, "color", (!c.is_empty() && c != "auto").then_some(c.as_str())));
            }
            "format.clear" => self.edit(cx, md::edit::clear_formatting),
            "format.fontFamily" | "format.fontSize" => {
                let family = id == "format.fontFamily";
                match arg {
                    Some(v) => {
                        let v = v.trim().to_string();
                        let v = if !family && v.parse::<f32>().is_ok() { format!("{v}pt") } else { v };
                        let prop = if family { "font-family" } else { "font-size" };
                        self.edit(cx, |t, s| md::style::apply_style(t, s, prop, (!v.is_empty()).then_some(v.as_str())));
                    }
                    None => {
                        let menu = if family { Menu::Fonts(FontTarget::Selection) } else { Menu::Sizes(FontTarget::Selection) };
                        self.open_menu_at(menu, window.mouse_position(), window, cx);
                    }
                }
            }
            "format.grow" | "format.shrink" => {
                let cur = {
                    let ed = ed.read(cx);
                    md::style::style_value(ed.text(), ed.selection().head, "font-size")
                        .as_deref()
                        .and_then(md::style::points)
                        .or_else(|| self.fm("fontsize", cx).as_deref().and_then(md::style::points))
                        .unwrap_or(12.0)
                };
                let next = format!("{}pt", md::style::step_size(cur, id == "format.grow"));
                self.edit(cx, |t, s| md::style::apply_style(t, s, "font-size", Some(&next)));
            }
            "format.highlightColor" => {
                let c = arg.unwrap_or_default();
                self.edit(cx, |t, s| md::style::apply_style(t, s, "background-color", (!c.is_empty()).then_some(c.as_str())));
            }
            "format.normal" => self.edit(cx, |t, s| Some(md::edit::set_heading(t, s, 0))),
            "format.h1" | "format.h2" | "format.h3" | "format.h4" | "format.h5" | "format.h6" => {
                let level = id.as_bytes()[8] - b'0';
                self.edit(cx, |t, s| Some(md::edit::set_heading(t, s, level)));
            }
            "format.bullets" => self.edit(cx, |t, s| Some(md::edit::toggle_lines(t, s, LineKind::Bullet))),
            "format.numbering" => self.edit(cx, |t, s| Some(md::edit::toggle_lines(t, s, LineKind::Number))),
            "format.checklist" => self.edit(cx, |t, s| Some(md::edit::toggle_lines(t, s, LineKind::Task))),
            "format.quote" => self.edit(cx, |t, s| Some(md::edit::toggle_lines(t, s, LineKind::Quote))),
            "format.indent" => self.edit(cx, |t, s| md::edit::indent(t, s, true)),
            "format.outdent" => self.edit(cx, |t, s| md::edit::indent(t, s, false)),

            "insert.hr" => self.block("---".into(), cx),
            "insert.pagebreak" => self.block("<!-- pagebreak -->".into(), cx),
            "insert.toc" => self.block("[TOC]".into(), cx),
            "insert.callout" => {
                let id = arg.unwrap_or_else(|| "NOTE".into());
                let text = library::CALLOUTS.iter().find(|c| c.0 == id).map(|c| c.2).unwrap_or("");
                self.block(format!("> [!{id}]\n> \u{1}{text}\u{1}"), cx);
            }
            "insert.code" => {
                let lang = arg.unwrap_or_default();
                let sel = ed.read(cx).selected_text().to_string();
                let body = if lang == "python {run}" {
                    "import math\nprint(f\"π ≈ {math.pi:.5f}\")".to_string()
                } else if sel.is_empty() {
                    "\u{1}\u{1}".to_string()
                } else {
                    sel
                };
                self.block(format!("```{lang}\n{body}\n```"), cx);
            }
            "insert.math" => {
                let tex = arg.unwrap_or_else(|| "\u{1}E = mc^2\u{1}".into());
                self.block(format!("$$\n{tex}\n$$"), cx);
            }
            "insert.mathInline" => self.wrap("$", None, Some("x^2"), cx),
            "insert.diagram" | "insert.chart" => {
                let list = if id == "insert.diagram" { library::DIAGRAMS } else { library::CHARTS };
                let d = arg.as_deref().and_then(|a| list.iter().find(|d| d.id == a)).unwrap_or(&list[0]);
                self.block(format!("```{}\n{}\n```", d.lang, d.code), cx);
            }
            "insert.tableDialog" if arg.is_none() => self.open_prompt(panels::PromptKind::TableSize, window, cx),
            "insert.table" | "insert.tableDialog" => {
                let (r, c) = arg
                    .as_deref()
                    .and_then(|a| a.split_once('x'))
                    .and_then(|(r, c)| Some((r.parse().ok()?, c.parse().ok()?)))
                    .unwrap_or((3, 3));
                self.block(md::edit::table_markdown(r, c), cx);
            }
            "table.addRow" => self.edit(cx, md::edit::table_add_row),
            "table.addCol" => self.edit(cx, md::edit::table_add_col),
            "table.delRow" => self.edit(cx, md::edit::table_del_row),
            "table.format" => {
                if self.in_table(cx) {
                    self.edit(cx, |t, s| md::edit::format_table(t, s, |rows, _| rows));
                } else {
                    self.toast("Put the cursor in a table first.", cx);
                }
            }
            "insert.footnote" => self.edit(cx, |t, s| Some(md::edit::footnote(t, s))),
            "insert.link" => self.open_prompt(panels::PromptKind::Link, window, cx),
            "edit.find" => self.open_find(false, window, cx),
            "edit.replace" => self.open_find(true, window, cx),
            "edit.goto" => self.open_prompt(panels::PromptKind::GoToLine, window, cx),
            "insert.online" => self.open_prompt(panels::PromptKind::OnlineVideo, window, cx),
            "insert.csv" => self.pick_file("Insert a CSV file as a table", panels::Picked::Csv, window, cx),
            "insert.video" => self.pick_file("Insert a video or audio file", panels::Picked::Video, window, cx),
            "insert.import" => self.pick_file("Include a file", panels::Picked::Include, window, cx),
            "insert.importPreview" => self.pick_file("Include a file as a preview", panels::Picked::IncludePreview, window, cx),
            "insert.pictureUrl" => self.open_prompt(panels::PromptKind::PictureUrl, window, cx),
            "insert.pictureClipboard" => self.paste_picture(window, cx),
            "insert.molecule" => self.block("```molstar {pdb=1CRN height=360 title=\"Crambin\" spin=true}\n```".into(), cx),
            "insert.emoji" => match arg {
                Some(t) => ed.update(cx, |e, cx| e.insert(&t, cx)),
                None => self.open_menu_at(Menu::Emoji, window.mouse_position(), window, cx),
            },
            "insert.cover" => {
                let fm = |k: &str| self.fm(k, cx);
                let title = fm("title").unwrap_or_else(|| "Document title".into());
                let subtitle = fm("subtitle").unwrap_or_else(|| "Subtitle".into());
                let author = fm("author").unwrap_or_else(|| "Author".into());
                let date = fm("date").unwrap_or_else(|| chrono::Local::now().format("%-d %B %Y").to_string());
                let start = md::edit::body_start(ed.read(cx).text());
                ed.update(cx, |e, cx| e.set_selection(Selection::cursor(start), cx));
                self.block(format!("<div class=\"cover\">\n\n# \u{1}{title}\u{1}\n\n{subtitle}\n\n**{author}**  \n{date}\n\n</div>\n\n<!-- pagebreak -->"), cx);
            }
            "insert.details" => self.block("<details>\n<summary>Title</summary>\n\n\u{1}Content\u{1}\n\n</details>".into(), cx),
            "insert.date" | "insert.symbol" => match arg {
                Some(t) => ed.update(cx, |e, cx| e.insert(&t, cx)),
                None => {
                    let menu = if id == "insert.date" { Menu::Date } else { Menu::Symbol };
                    self.menu = Some(OpenMenu { menu, at: window.mouse_position() });
                }
            },
            "insert.smiles" => self.block("```smiles\nCC(=O)OC1=CC=CC=C1C(=O)O\n```".into(), cx),
            "insert.picture" => self.pick_picture(window, cx),
            "insert.properties" => {
                let text = ed.read(cx).text().to_string();
                let title = md::outline(&md::parse(&text)).first().map(|h| h.text.clone()).unwrap_or_else(|| "Untitled".into());
                let date = chrono::Local::now().format("%Y-%m-%d").to_string();
                let have = md::edit::front_matter(&text);
                let has = |k: &str| have.iter().any(|(key, _)| key == k);
                let mut pairs: Vec<(&str, Option<&str>)> = Vec::new();
                if !has("title") {
                    pairs.push(("title", Some(&title)));
                }
                if !has("date") {
                    pairs.push(("date", Some(&date)));
                }
                if pairs.is_empty() {
                    self.toast("The document already has its properties at the top.", cx);
                } else {
                    self.edit(cx, |t, s| Some(md::edit::set_front_matter(t, s, &pairs)));
                }
                ed.update(cx, |e, cx| e.set_selection(Selection::cursor(0), cx));
            }

            "design.theme" => {
                if let Some(t) = arg {
                    self.set_fm("theme", Some(&t), cx);
                }
            }
            "design.paper" | "design.orientation" | "design.margins" => {
                if let Some(v) = arg {
                    let text = ed.read(cx).text().to_string();
                    let front = md::edit::front_matter(&text);
                    let mut page = md::page::PageSetup::from_front(&|k| front.iter().find(|(key, _)| key == k).map(|(_, v)| v.clone()));
                    match id {
                        "design.paper" => {
                            // the older names still work
                            page.paper = match v.as_str() {
                                "a4" => "a4paper".into(),
                                "letter" => "letterpaper".into(),
                                "legal" => "legalpaper".into(),
                                "a5" => "a5paper".into(),
                                other => other.to_string(),
                            }
                        }
                        "design.orientation" => page.landscape = v == "landscape",
                        _ => {
                            page.margins = md::page::Margins::all(match v.as_str() {
                                "narrow" => 36.0,
                                "moderate" => 56.7,
                                "wide" => 99.2,
                                _ => 70.9,
                            })
                        }
                    }
                    let geometry = page.to_geometry();
                    self.edit(cx, |t, s| {
                        Some(md::edit::set_front_matter(
                            t,
                            s,
                            &[("geometry", Some(&geometry)), ("paper", None), ("orientation", None), ("margins", None)],
                        ))
                    });
                }
            }
            "design.watermark" => match arg {
                Some(t) if t.is_empty() => self.set_fm("watermark", None, cx),
                Some(t) => self.set_fm("watermark", Some(&t), cx),
                None => self.open_prompt(panels::PromptKind::Watermark, window, cx),
            },
            "design.mainfont" | "design.monofont" | "design.fontsize" | "design.sansfont" | "design.mathfont" => match arg {
                Some(v) => {
                    let v = v.trim().to_string();
                    self.set_fm(&id["design.".len()..], (!v.is_empty()).then_some(v.as_str()), cx);
                }
                None => {
                    let menu = match id {
                        "design.mainfont" => Menu::Fonts(FontTarget::DocMain),
                        "design.monofont" => Menu::Fonts(FontTarget::DocMono),
                        "design.sansfont" => Menu::Fonts(FontTarget::DocSans),
                        "design.mathfont" => Menu::Fonts(FontTarget::DocMath),
                        _ => Menu::Sizes(FontTarget::DocSize),
                    };
                    self.open_menu_at(menu, window.mouse_position(), window, cx);
                }
            },
            "design.numbered" | "design.justify" => {
                let key = &id["design.".len()..];
                let on = self.fm(key, cx).is_some_and(|v| v == "true");
                self.set_fm(key, if on { None } else { Some("true") }, cx);
            }

            "slides.new" => {
                let l = arg.as_deref().and_then(|a| library::LAYOUTS.iter().find(|l| l.id == a)).unwrap_or(&library::LAYOUTS[1]);
                self.edit(cx, |t, s| Some(md::edit::new_slide(t, s, l.cls, l.body)));
            }
            "slides.layout" | "slides.duplicate" | "slides.delete" | "slides.up" | "slides.down" | "slides.notes" | "slides.background"
                if md::slides::slides(ed.read(cx).text()).is_empty() =>
            {
                self.toast("This document has no slides yet: add one with Slides › New slide.", cx)
            }
            "slides.layout" => {
                let cls = arg.as_deref().and_then(|a| library::LAYOUTS.iter().find(|l| l.id == a)).map(|l| l.cls).unwrap_or("");
                self.edit(cx, |t, s| md::slides::set_attr(t, s, "class", (!cls.is_empty()).then_some(cls)));
            }
            "slides.duplicate" => self.edit(cx, md::slides::duplicate),
            "slides.delete" => self.edit(cx, md::slides::delete),
            "slides.up" => self.edit(cx, |t, s| md::slides::move_slide(t, s, -1)),
            "slides.down" => self.edit(cx, |t, s| md::slides::move_slide(t, s, 1)),
            "slides.notes" => self.edit(cx, md::slides::add_notes),
            "slides.background" => match arg {
                Some(c) => self.edit(cx, |t, s| md::slides::set_attr(t, s, "data-background-color", (!c.is_empty()).then_some(c.as_str()))),
                None => self.open_menu_at(Menu::SlideBackground, window.mouse_position(), window, cx),
            },
            "slides.present" | "slides.presentCurrent" => {
                let current = {
                    let ed = ed.read(cx);
                    let line = ed.doc.line_of(ed.selection().head) + 1;
                    self.preview.read(cx).slide_at_line(line)
                };
                match current {
                    None => self.toast("Nothing to present: add slides with Slides › New slide or Convert to slides.", cx),
                    Some(c) => self.start_present(if id == "slides.present" { 0 } else { c }, window, cx),
                }
                cx.notify();
                return;
            }
            "slides.fromHeadings" => match md::slides::from_headings(ed.read(cx).text()) {
                Some(t) => {
                    ed.update(cx, |e, cx| e.set_text(&t, cx));
                    self.toast("Converted: a slide at every heading. Undo puts the document back.", cx);
                }
                None => self.toast("This document already has slides (or no headings to start them at).", cx),
            },
            "review.accessibility" => {
                let issues = md::review::accessibility(&self.preview.read(cx).parsed);
                self.info = Some(panels::Info::Accessibility(issues));
            }
            "help.shortcuts" => self.info = Some(panels::Info::Shortcuts),
            "file.info" => self.info = Some(panels::Info::Document(self.document_info(cx))),
            "view.pageWidth" => self.preview.update(cx, |p, cx| {
                p.page_width = !p.page_width;
                cx.notify();
            }),

            "review.wordcount" => {
                let text = ed.read(cx).text().to_string();
                let words = md::word_count(&self.preview.read(cx).parsed);
                let chars = text.chars().count();
                let lines = ed.read(cx).doc.line_count();
                self.toast(format!("{words} words · {chars} characters · {lines} lines"), cx);
            }
            "review.outline" => {
                self.side = Some(side::Side::Nav);
                self.nav_slides = false;
            }
            "review.comment" => self.start_comment(window, cx),
            "view.comments" => self.show_side(side::Side::Comments, true, window, cx),
            "help.feedback" => self.show_side(side::Side::Help, true, window, cx),

            "view.write" => self.mode = ViewMode::Write,
            "view.split" => self.mode = ViewMode::Split,
            "view.read" => self.mode = ViewMode::Read,
            "view.nav" => self.show_side(side::Side::Nav, true, window, cx),
            "view.lineNumbers" => ed.update(cx, |e, cx| {
                e.line_numbers = !e.line_numbers;
                cx.notify();
            }),
            "view.wrap" => ed.update(cx, |e, cx| {
                e.wrap = !e.wrap;
                cx.notify();
            }),
            "view.sync" => self.sync = !self.sync,
            "view.zoomIn" => self.set_zoom(self.zoom + 0.1, cx),
            "view.zoomOut" => self.set_zoom(self.zoom - 0.1, cx),
            "view.zoomReset" => self.set_zoom(1.0, cx),
            "view.darkPage" => self.preview.update(cx, |p, cx| {
                p.dark_page = !p.dark_page;
                cx.notify();
            }),
            "view.ribbon" => self.collapsed = !self.collapsed,
            "view.focus" => self.focus_mode = !self.focus_mode,
            "view.refresh" => {
                let text = ed.read(cx).text().to_string();
                self.preview.update(cx, |p, cx| p.update_text(&text, u64::MAX - 1, cx));
            }
            "help.search" => {
                let s = self.search.clone();
                s.read(cx).focus(window);
            }
            _ => self.toast(format!("{} comes in a later version of Kadee Markdown.", cmd.label), cx),
        }
        if !matches!(
            id,
            "help.search"
                | "file.backstage"
                | "insert.date"
                | "insert.symbol"
                | "edit.find"
                | "edit.replace"
                | "file.properties"
                | "review.comment"
                | "view.comments"
                | "help.feedback"
                | "view.nav"
        ) && self.menu.is_none()
            && self.prompt.is_none()
        {
            self.focus_editor(window, cx);
        }
        cx.notify();
    }

    fn pick_picture(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let rx = cx.prompt_for_paths(PathPromptOptions { files: true, directories: false, multiple: false, prompt: Some("Insert".into()) });
        let base = self.path.as_ref().and_then(|p| p.parent().map(|d| d.to_path_buf()));
        cx.spawn_in(window, async move |this, cx| {
            let Ok(Ok(Some(paths))) = rx.await else { return };
            let Some(p) = paths.into_iter().next() else { return };
            let shown = base.as_ref().and_then(|b| p.strip_prefix(b).ok()).unwrap_or(&p).to_string_lossy().replace('\\', "/");
            let alt = p.file_stem().map(|s| s.to_string_lossy().to_string()).unwrap_or_default();
            let link = if shown.contains(' ') { format!("<{shown}>") } else { shown };
            let _ = this.update(cx, |this, cx| this.block(format!("![\u{1}{alt}\u{1}]({link})"), cx));
        })
        .detach();
    }

    /// Whether a ribbon button shows as on.
    fn checked(&self, id: &str, cx: &App) -> bool {
        let ed = self.editor.read(cx);
        let f = || md::edit::active_formats(ed.text(), ed.selection().head);
        match id {
            "format.bold" => f().bold,
            "format.italic" => f().italic,
            "format.strike" => f().strike,
            "format.highlight" => f().highlight,
            "format.code" => f().code,
            "format.underline" => f().underline,
            "format.bullets" => f().bullet,
            "format.numbering" => f().number,
            "format.checklist" => f().task,
            "format.quote" => f().quote,
            "view.write" => self.mode == ViewMode::Write,
            "view.split" => self.mode == ViewMode::Split,
            "view.read" => self.mode == ViewMode::Read,
            "view.nav" => self.side == Some(side::Side::Nav),
            "view.lineNumbers" => ed.line_numbers,
            "view.wrap" => ed.wrap,
            "view.sync" => self.sync,
            "view.focus" => self.focus_mode,
            "view.darkPage" => self.preview.read(cx).dark_page,
            "view.light" => cx.global::<Theme>().pref == ThemePref::Light,
            "view.dark" => cx.global::<Theme>().pref == ThemePref::Dark,
            "view.system" => cx.global::<Theme>().pref == ThemePref::System,
            "design.numbered" | "design.justify" => self.fm(&id["design.".len()..], cx).is_some_and(|v| v == "true"),
            id if id.starts_with("para.") => self.para_checked(id, cx).unwrap_or(false),
            _ => false,
        }
    }

    fn enabled(&self, id: &str, cx: &App) -> bool {
        let ed = self.editor.read(cx);
        match id {
            "edit.undo" => ed.doc.can_undo(),
            "edit.redo" => ed.doc.can_redo(),
            "edit.cut" | "edit.copy" => !ed.selection().is_empty(),
            "table.addRow" | "table.addCol" | "table.delRow" => self.in_table(cx),
            _ => true,
        }
    }

    fn open_menu(&mut self, menu: Menu, at: Point<Pixels>, cx: &mut Context<Self>) {
        self.menu = if self.menu.as_ref().is_some_and(|m| m.menu == menu) { None } else { Some(OpenMenu { menu, at }) };
        cx.notify();
    }
}

/// Commands whose label matches `q`, best first (MdCraft's command search).
pub fn search_commands(q: &str) -> Vec<&'static commands::Cmd> {
    let q = q.trim().to_lowercase();
    if q.is_empty() {
        return Vec::new();
    }
    let mut hits: Vec<(f32, &commands::Cmd)> = commands::COMMANDS
        .iter()
        .filter(|c| c.id != "file.backstage")
        .filter_map(|c| {
            let label = c.label.to_lowercase();
            let id = c.id.to_lowercase();
            let s = if label.starts_with(&q) {
                3.0
            } else if label.split_whitespace().any(|w| w.starts_with(&q)) {
                2.5
            } else if label.contains(&q) {
                2.0
            } else if q.split_whitespace().all(|w| label.contains(w) || id.contains(w)) {
                1.0
            } else {
                0.0
            };
            (s > 0.0).then_some((s, c))
        })
        .collect();
    hits.sort_by(|a, b| b.0.total_cmp(&a.0));
    hits.into_iter().take(12).map(|(_, c)| c).collect()
}

// ---- drawing -----------------------------------------------------------------------------------

impl MarkdownView {
    fn tabstrip(&mut self, pal: &Palette, cx: &mut Context<Self>) -> impl IntoElement {
        let in_table = self.in_table(cx);
        let tabs: Vec<AnyElement> = ribbon::tabs()
            .into_iter()
            .filter(|t| t.id != TabId::File && (!t.contextual || in_table))
            .map(|t| {
                let on = t.id == self.tab;
                let id = t.id;
                div()
                    .id(SharedString::from(format!("md-tab-{}", t.label)))
                    .relative()
                    .flex()
                    .items_center()
                    .px(px(13.0))
                    .mt(px(4.0))
                    .rounded_t(px(6.0))
                    .text_size(px(13.0))
                    .cursor_pointer()
                    .text_color(if on || t.contextual { pal.accent } else { pal.text })
                    .when(on, |d| d.bg(pal.surface).font_weight(FontWeight::SEMIBOLD).border_1().border_b_0().border_color(pal.border))
                    .when(!on, |d| d.hover(|d| d.bg(pal.surface_hover)))
                    .when(t.contextual, |d| {
                        d.child(
                            div().absolute().top_0().left(px(4.0)).right(px(4.0)).h(px(3.0)).rounded_b(px(3.0)).bg(pal.accent).opacity(0.6),
                        )
                    })
                    .when(on, |d| {
                        d.child(div().absolute().bottom(px(3.0)).left(px(12.0)).right(px(12.0)).h(px(2.0)).rounded(px(2.0)).bg(pal.accent))
                    })
                    .child(t.label)
                    .on_click(cx.listener(move |this, _: &ClickEvent, _, cx| {
                        this.tab = id;
                        this.menu = None;
                        if this.collapsed {
                            this.peek = true;
                        }
                        cx.notify();
                    }))
                    .on_mouse_down(
                        MouseButton::Left,
                        cx.listener(|this, e: &gpui::MouseDownEvent, _, cx| {
                            if e.click_count == 2 {
                                this.collapsed = !this.collapsed;
                                this.peek = false;
                                cx.notify();
                            }
                        }),
                    )
                    .into_any_element()
            })
            .collect();
        let file_tab = div()
            .id("md-file-tab")
            .flex()
            .items_center()
            .px(px(18.0))
            .mr(px(4.0))
            .bg(pal.accent)
            .text_color(pal.on_accent)
            .font_weight(FontWeight::SEMIBOLD)
            .text_size(px(13.0))
            .cursor_pointer()
            .hover(|d| d.opacity(0.92))
            .child("File")
            .when(self.tab == TabId::File, |d| d.border_b(px(3.0)).border_color(pal.on_accent))
            .on_click(cx.listener(|this, _: &ClickEvent, window, cx| this.run("file.backstage", None, window, cx)));
        let focused = !self.search.read(cx).text().is_empty();
        let search = div()
            .relative()
            .flex()
            .items_center()
            .gap(px(6.0))
            .my(px(5.0))
            .mx(px(10.0))
            .w(px(300.0))
            .h(px(26.0))
            .px(px(8.0))
            .rounded(px(6.0))
            .border_1()
            .border_color(if focused { pal.accent } else { pal.border_strong })
            .bg(pal.surface)
            .text_size(px(13.0))
            .child(md_icon("search", 15.0, pal.text_muted.into(), pal.text_muted.into()))
            .child(div().flex_1().min_w_0().child(self.search.clone()));
        let name =
            self.path.as_ref().and_then(|p| p.file_name()).map(|n| n.to_string_lossy().to_string()).unwrap_or_else(|| "Untitled".into());
        let dirty = self.is_dirty(cx);
        let (ic, ac) = icon_colors(pal);
        let qat = |id: &'static str, icon: &'static str, tip: String, cx: &mut Context<Self>| {
            div()
                .id(SharedString::from(format!("md-qat-{id}")))
                .w(px(30.0))
                .my(px(4.0))
                .flex()
                .items_center()
                .justify_center()
                .rounded(px(6.0))
                .cursor_pointer()
                .hover(|d| d.bg(pal.surface_hover))
                .child(if icon.contains('/') {
                    svg().path(icon).size(px(17.0)).text_color(pal.accent).into_any_element()
                } else {
                    md_icon(icon, 18.0, ic, ac)
                })
                .tooltip(tooltip(tip))
                .on_click(cx.listener(move |this, _: &ClickEvent, window, cx| this.run(id, None, window, cx)))
        };
        let collapse_tip = if self.collapsed { "Pin the ribbon" } else { "Collapse the ribbon" };
        div()
            .flex()
            .h(px(36.0))
            .pr(px(8.0))
            .gap(px(2.0))
            .bg(pal.canvas)
            .child(file_tab)
            .child(div().flex().gap(px(2.0)).children(tabs))
            .child(div().flex_1())
            .child(search)
            .child(div().flex().items_center().mx(px(8.0)).text_size(px(12.5)).text_color(pal.text_muted).child(format!(
                "{}{}",
                kd_ui::truncate(&name, 26),
                if dirty { " •" } else { "" }
            )))
            .child(qat("file.save", "save", commands::tip(commands::get("file.save").expect("save")), cx))
            .child(qat("edit.undo", "undo", "Undo".into(), cx))
            .child(qat("edit.redo", "redo", "Redo".into(), cx))
            .child(qat("ai.ask", Icon::Spark.path(), "KaDee AI".into(), cx))
            .child(qat("view.ribbon", if self.collapsed { "chevron-down" } else { "chevron-up" }, collapse_tip.into(), cx))
    }

    fn ribbon(&mut self, pal: &Palette, cx: &mut Context<Self>) -> impl IntoElement {
        let tab = ribbon::tabs().into_iter().find(|t| t.id == self.tab).unwrap_or_else(|| ribbon::tabs().remove(0));
        let mut n = 0usize;
        let groups: Vec<AnyElement> = tab
            .groups
            .iter()
            .enumerate()
            .map(|(gi, grp)| {
                let content: Vec<AnyElement> = grp.items.iter().map(|it| self.item(it, &mut n, pal, cx)).collect();
                div()
                    .flex()
                    .flex_col()
                    .px(px(6.0))
                    .relative()
                    .when(gi > 0, |d| d.child(div().absolute().left_0().top(px(6.0)).bottom(px(8.0)).w(px(1.0)).bg(pal.border)))
                    .child(div().flex_1().flex().items_start().gap(px(2.0)).children(content))
                    .child(
                        div()
                            .flex()
                            .justify_center()
                            .pt(px(1.0))
                            .pb(px(4.0))
                            .text_size(px(11.0))
                            .text_color(pal.text_faint)
                            .child(grp.label),
                    )
                    .into_any_element()
            })
            .collect();
        div()
            .id("md-ribbon")
            .h(px(98.0))
            .w_full()
            .overflow_x_scroll()
            .bg(pal.surface)
            .border_b_1()
            .border_color(pal.border)
            .child(div().flex().h_full().pt(px(4.0)).px(px(4.0)).children(groups))
    }

    fn item(&mut self, it: &Item, n: &mut usize, pal: &Palette, cx: &mut Context<Self>) -> AnyElement {
        match it {
            Item::Btn(b) => self.button(b, n, pal, cx),
            Item::Col(v) => div()
                .flex()
                .flex_col()
                .gap(px(1.0))
                .pt(px(1.0))
                .children(v.iter().map(|b| self.button(b, n, pal, cx)).collect::<Vec<_>>())
                .into_any_element(),
            Item::Rows(rows) => div()
                .flex()
                .flex_col()
                .gap(px(4.0))
                .pt(px(4.0))
                .children(
                    rows.iter()
                        .map(|r| {
                            let cells: Vec<AnyElement> = r
                                .iter()
                                .map(|x| match x {
                                    RowItem::Btn(b) => self.button(b, n, pal, cx),
                                    RowItem::FontFamily => self.font_box("format.fontFamily", "Theme font", 132.0, n, pal, cx),
                                    RowItem::FontSize => self.font_box("format.fontSize", "—", 50.0, n, pal, cx),
                                })
                                .collect();
                            div()
                                .flex()
                                .gap(px(1.0))
                                .p(px(1.0))
                                .bg(pal.surface)
                                .border_1()
                                .border_color(pal.border)
                                .rounded(px(6.0))
                                .children(cells)
                        })
                        .collect::<Vec<_>>(),
                )
                .into_any_element(),
            Item::Gallery(g) => self.gallery(*g, n, pal, cx),
            Item::DocFonts => {
                let main = self.fm("mainfont", cx).unwrap_or_else(|| "Theme font".into());
                let mono = self.fm("monofont", cx).unwrap_or_else(|| "Theme code font".into());
                let size = self.fm("fontsize", cx).unwrap_or_else(|| "Theme".into());
                let line = |label: &'static str,
                            cmd: &'static str,
                            value: String,
                            w: f32,
                            n: &mut usize,
                            this: &mut Self,
                            cx: &mut Context<Self>| {
                    div()
                        .flex()
                        .items_center()
                        .gap(px(6.0))
                        .text_size(px(12.0))
                        .child(div().w(px(34.0)).text_color(pal.text_muted).child(label))
                        .child(this.font_box(cmd, Box::leak(value.into_boxed_str()), w, n, pal, cx))
                };
                div()
                    .flex()
                    .flex_col()
                    .gap(px(2.0))
                    .pt(px(2.0))
                    .child(line("Body", "design.mainfont", main, 150.0, n, self, cx))
                    .child(line("Code", "design.monofont", mono, 150.0, n, self, cx))
                    .child(line("Size", "design.fontsize", size, 70.0, n, self, cx))
                    .into_any_element()
            }
        }
    }

    pub(super) fn font_box(
        &mut self,
        cmd: &'static str,
        placeholder: &'static str,
        w: f32,
        n: &mut usize,
        pal: &Palette,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        *n += 1;
        let c = commands::get(cmd).expect("font command");
        div()
            .id(("md-fontbox", *n))
            .flex()
            .items_center()
            .w(px(w))
            .h(px(24.0))
            .pl(px(6.0))
            .rounded(px(4.0))
            .border_1()
            .border_color(pal.border_strong)
            .bg(pal.surface)
            .text_size(px(12.5))
            .text_color(pal.text_muted)
            .cursor_pointer()
            .hover(|d| d.border_color(pal.accent))
            .child(div().flex_1().min_w_0().overflow_hidden().child(placeholder))
            .child(div().w(px(16.0)).flex().justify_center().child(md_icon("caret", 12.0, pal.text_muted.into(), pal.text_muted.into())))
            .tooltip(tooltip(c.label))
            .on_click(cx.listener(move |this, _: &ClickEvent, window, cx| this.run(cmd, None, window, cx)))
            .into_any_element()
    }

    fn button(&mut self, b: &Btn, n: &mut usize, pal: &Palette, cx: &mut Context<Self>) -> AnyElement {
        *n += 1;
        let Some(c) = commands::get(b.cmd) else { return div().into_any_element() };
        let label = b.label.unwrap_or(c.label).trim_end_matches('…').to_string();
        let on = self.checked(b.cmd, cx);
        let enabled = self.enabled(b.cmd, cx);
        let (ic, ac) = icon_colors(pal);
        let cmd = b.cmd;
        let menu = b.menu;
        let icon_el = |size: f32| -> AnyElement {
            if c.icon.contains('/') {
                svg().path(c.icon).size(px(size)).text_color(pal.accent).into_any_element()
            } else {
                md_icon(c.icon, size, ic, ac)
            }
        };
        let caret = || md_icon("caret", 12.0, pal.text_muted.into(), pal.text_muted.into());
        let base = |id: (&'static str, usize)| {
            div()
                .id(id)
                .flex()
                .items_center()
                .rounded(px(5.0))
                .border_1()
                .border_color(gpui::transparent_black())
                .text_color(pal.text)
                .when(on, |d| d.bg(pal.accent_soft).border_color(Hsla::from(pal.accent).opacity(0.35)))
                .when(!enabled, |d| d.opacity(0.42))
                .when(enabled, |d| d.cursor_pointer().hover(|d| d.bg(pal.surface_hover)))
        };
        let tip = commands::tip(c);
        let drop_only = b.drop_only;
        let open_menu = move |this: &mut Self, e: &ClickEvent, cx: &mut Context<Self>| {
            if let Some(m) = menu {
                let at = point(e.position().x - px(8.0), e.position().y + px(12.0));
                this.open_menu(m, at, cx);
            }
        };
        // the button: runs the command, or opens the menu of a drop-down button
        let main_click = cx.listener(move |this, e: &ClickEvent, window, cx| {
            cx.stop_propagation();
            if drop_only { open_menu(this, e, cx) } else { this.run(cmd, None, window, cx) }
        });
        // the arrow of a split button
        let open_click = cx.listener(move |this, e: &ClickEvent, _, cx| {
            cx.stop_propagation();
            open_menu(this, e, cx)
        });
        match b.size {
            Size::Large => {
                let body = |d: gpui::Stateful<gpui::Div>, with_caret: bool| {
                    d.flex_col()
                        .justify_start()
                        .min_w(px(52.0))
                        .max_w(px(92.0))
                        .px(px(4.0))
                        .pt(px(4.0))
                        .pb(px(2.0))
                        .gap(px(3.0))
                        .child(icon_el(30.0))
                        .child(
                            div()
                                .flex()
                                .flex_col()
                                .items_center()
                                .text_size(px(12.0))
                                .line_height(px(14.0))
                                .text_center()
                                .when(!label.contains(" "), |d| d.whitespace_nowrap())
                                .child(label.clone())
                                .when(with_caret, |d| d.child(caret())),
                        )
                };
                if menu.is_some() && !drop_only {
                    // split: the top runs the command, the arrow opens the menu
                    div()
                        .flex()
                        .flex_col()
                        .rounded(px(5.0))
                        .child(body(base(("md-rb", *n)), false).h(px(54.0)).rounded_b_none().tooltip(tooltip(tip)).on_click(main_click))
                        .child(
                            div()
                                .id(("md-rb-arrow", *n))
                                .h(px(18.0))
                                .flex()
                                .justify_center()
                                .items_center()
                                .rounded_b(px(5.0))
                                .cursor_pointer()
                                .hover(|d| d.bg(pal.surface_hover))
                                .child(caret())
                                .tooltip(tooltip(format!("{label} options")))
                                .on_click(open_click),
                        )
                        .into_any_element()
                } else {
                    let _ = &open_click;
                    let click = main_click;
                    body(base(("md-rb", *n)), menu.is_some()).h(px(72.0)).tooltip(tooltip(tip)).on_click(click).into_any_element()
                }
            }
            Size::Small | Size::Icon => {
                let small = b.size == Size::Small;
                let inner = |d: gpui::Stateful<gpui::Div>| {
                    if small {
                        d.h(px(23.0)).pl(px(3.0)).pr(px(6.0)).gap(px(5.0)).text_size(px(12.5)).child(icon_el(18.0)).child(label.clone())
                    } else {
                        d.size(px(28.0)).justify_center().child(icon_el(18.0))
                    }
                };
                if menu.is_some() && !drop_only {
                    div()
                        .flex()
                        .child(inner(base(("md-rb", *n))).rounded_r_none().tooltip(tooltip(tip)).on_click(main_click))
                        .child(
                            div()
                                .id(("md-rb-arrow", *n))
                                .w(px(14.0))
                                .h(px(if small { 23.0 } else { 28.0 }))
                                .flex()
                                .items_center()
                                .justify_center()
                                .rounded_r(px(5.0))
                                .cursor_pointer()
                                .hover(|d| d.bg(pal.surface_hover))
                                .child(caret())
                                .tooltip(tooltip(format!("{label} options")))
                                .on_click(open_click),
                        )
                        .into_any_element()
                } else {
                    let _ = &open_click;
                    let click = main_click;
                    inner(base(("md-rb", *n)))
                        .when(menu.is_some(), |d| d.child(caret()))
                        .tooltip(tooltip(tip))
                        .on_click(click)
                        .into_any_element()
                }
            }
        }
    }

    fn gallery(&mut self, g: Gallery, n: &mut usize, pal: &Palette, cx: &mut Context<Self>) -> AnyElement {
        *n += 1;
        let tiles: Vec<AnyElement> = match g {
            Gallery::Styles => {
                let heading = {
                    let ed = self.editor.read(cx);
                    md::edit::active_formats(ed.text(), ed.selection().head)
                };
                let styles: [(&'static str, &str, &str); 7] = [
                    ("format.normal", "Normal", "p"),
                    ("format.h1", "Heading 1", "h1"),
                    ("format.h2", "Heading 2", "h2"),
                    ("format.h3", "Heading 3", "h3"),
                    ("format.h4", "Heading 4", "h4"),
                    ("format.quote", "Quote", "q"),
                    ("format.code", "Code", "c"),
                ];
                styles
                    .into_iter()
                    .enumerate()
                    .map(|(i, (cmd, name, tag))| {
                        let on = match tag {
                            "p" => heading.heading == 0 && !heading.quote,
                            "q" => heading.quote,
                            "c" => heading.code,
                            h => heading.heading == h.as_bytes()[1] - b'0',
                        };
                        let sample = div().whitespace_nowrap().child(if tag == "c" { "AaBb" } else { "AaBbCc" });
                        let sample = match tag {
                            "h1" => sample.text_size(px(19.0)).font_weight(FontWeight::BOLD).text_color(pal.accent),
                            "h2" => sample.text_size(px(17.0)).font_weight(FontWeight::BOLD).text_color(pal.accent),
                            "h3" => sample.text_size(px(15.0)).font_weight(FontWeight::BOLD),
                            "h4" => sample.text_size(px(14.0)).font_weight(FontWeight::SEMIBOLD).italic(),
                            "q" => sample
                                .text_size(px(15.0))
                                .italic()
                                .text_color(pal.text_muted)
                                .border_l(px(3.0))
                                .border_color(pal.border_strong)
                                .pl(px(5.0)),
                            "c" => sample
                                .text_size(px(13.0))
                                .font_family(crate::editor::mono_family())
                                .bg(mix(pal.surface_hover, pal.border, 0.5))
                                .px(px(4.0))
                                .rounded(px(3.0)),
                            _ => sample.text_size(px(15.0)),
                        };
                        let key = commands::get(cmd).map(commands::tip).unwrap_or_default();
                        div()
                            .id(("md-style", i))
                            .w(px(76.0))
                            .h_full()
                            .flex()
                            .flex_col()
                            .justify_between()
                            .px(px(7.0))
                            .pt(px(6.0))
                            .pb(px(4.0))
                            .rounded(px(4.0))
                            .border_1()
                            .overflow_hidden()
                            .cursor_pointer()
                            .text_color(pal.text)
                            .border_color(if on { pal.accent } else { gpui::transparent_black().into() })
                            .when(on, |d| d.bg(pal.accent_soft))
                            .when(!on, |d| d.hover(|d| d.bg(pal.surface_hover).border_color(pal.border_strong)))
                            .child(sample)
                            .child(div().text_size(px(11.0)).text_color(pal.text_muted).whitespace_nowrap().child(name.to_string()))
                            .tooltip(tooltip(key))
                            .on_click(cx.listener(move |this, _: &ClickEvent, window, cx| this.run(cmd, None, window, cx)))
                            .into_any_element()
                    })
                    .collect()
            }
            Gallery::Themes | Gallery::DeckThemes => {
                let deck = g == Gallery::DeckThemes;
                let cur = self.fm("theme", cx).unwrap_or_default();
                library::THEMES
                    .iter()
                    .filter(|t| t.deck == deck)
                    .enumerate()
                    .map(|(i, t)| {
                        let on = cur == t.id;
                        let [a, b2, c2] = t.colors.map(|c| Hsla::from(gpui::rgb(c)));
                        let (bg, aa) = if deck { (a, c2) } else { (gpui::white(), a) };
                        let id = t.id;
                        div()
                            .id(("md-theme", i + if deck { 100 } else { 0 }))
                            .w(px(70.0))
                            .flex()
                            .flex_col()
                            .gap(px(2.0))
                            .p(px(3.0))
                            .rounded(px(5.0))
                            .border_1()
                            .cursor_pointer()
                            .border_color(if on { pal.accent } else { gpui::transparent_black().into() })
                            .when(on, |d| d.bg(pal.accent_soft))
                            .when(!on, |d| d.hover(|d| d.border_color(pal.border_strong)))
                            .child(
                                div()
                                    .h(px(44.0))
                                    .rounded(px(3.0))
                                    .border_1()
                                    .border_color(pal.border)
                                    .bg(bg)
                                    .flex()
                                    .flex_col()
                                    .justify_between()
                                    .px(px(5.0))
                                    .py(px(3.0))
                                    .child(
                                        div()
                                            .text_size(px(17.0))
                                            .font_weight(FontWeight::BOLD)
                                            .line_height(px(19.0))
                                            .text_color(aa)
                                            .child("Aa"),
                                    )
                                    .child(
                                        div()
                                            .flex()
                                            .gap(px(2.0))
                                            .children([a, b2, c2].map(|c| div().flex_1().h(px(5.0)).rounded(px(1.0)).bg(c))),
                                    ),
                            )
                            .child(div().text_size(px(10.5)).text_color(pal.text).flex().justify_center().overflow_hidden().child(t.name))
                            .tooltip(tooltip(format!("{} theme", t.name)))
                            .on_click(
                                cx.listener(move |this, _: &ClickEvent, window, cx| this.run("design.theme", Some(id.into()), window, cx)),
                            )
                            .into_any_element()
                    })
                    .collect()
            }
            Gallery::Layouts => library::LAYOUTS
                .iter()
                .filter(|l| l.id != "blank" && l.id != "image")
                .enumerate()
                .map(|(i, l)| {
                    let short = l
                        .name
                        .replace(" slide", "")
                        .replace("Title and content", "Content")
                        .replace("Section header", "Section")
                        .replace("Two content", "Two columns");
                    let id = l.id;
                    let (ic, ac) = icon_colors(pal);
                    div()
                        .id(("md-layout", i))
                        .w(px(74.0))
                        .h(px(32.0))
                        .flex()
                        .items_center()
                        .gap(px(4.0))
                        .px(px(4.0))
                        .rounded(px(4.0))
                        .border_1()
                        .border_color(gpui::transparent_black())
                        .text_size(px(11.0))
                        .text_color(pal.text)
                        .overflow_hidden()
                        .cursor_pointer()
                        .hover(|d| d.bg(pal.surface_hover).border_color(pal.border_strong))
                        .child(md_icon("slide-layout", 20.0, ic, ac))
                        .child(div().whitespace_nowrap().child(short))
                        .tooltip(tooltip(format!("Layout: {}", l.name)))
                        .on_click(
                            cx.listener(move |this, _: &ClickEvent, window, cx| this.run("slides.layout", Some(id.into()), window, cx)),
                        )
                        .into_any_element()
                })
                .collect(),
        };
        let grid = if g == Gallery::Layouts {
            div().flex().flex_col().flex_wrap().h(px(66.0)).gap(px(2.0))
        } else {
            div().flex().gap(px(4.0)).h_full()
        };
        div()
            .id(("md-gallery", *n))
            .h(px(72.0))
            .max_w(px(500.0))
            .p(px(2.0))
            .border_1()
            .border_color(pal.border)
            .rounded(px(6.0))
            .bg(pal.surface)
            .overflow_x_scroll()
            .child(grid.children(tiles))
            .into_any_element()
    }

    fn nav_pane(&mut self, pal: &Palette, cx: &mut Context<Self>) -> impl IntoElement {
        let (text, cur_line) = {
            let ed = self.editor.read(cx);
            (ed.text().to_string(), ed.doc.line_of(ed.selection().head) + 1)
        };
        let outline = md::outline(&self.preview.read(cx).parsed);
        let filter = self.nav_filter.read(cx).text().to_lowercase();
        let seg = |label: &'static str, on: bool, slides: bool, cx: &mut Context<Self>| {
            div()
                .id(label)
                .px(px(10.0))
                .py(px(3.0))
                .text_size(px(12.0))
                .cursor_pointer()
                .when(on, |d| d.bg(pal.accent).text_color(pal.on_accent))
                .when(!on, |d| d.bg(pal.surface).hover(|d| d.bg(pal.surface_hover)))
                .child(label)
                .on_click(cx.listener(move |this, _: &ClickEvent, _, cx| {
                    this.nav_slides = slides;
                    cx.notify();
                }))
        };
        let current = outline.iter().rposition(|h| h.line <= cur_line);
        let rows: Vec<AnyElement> = if !self.nav_slides {
            outline
                .iter()
                .enumerate()
                .filter(|(_, h)| filter.is_empty() || h.text.to_lowercase().contains(&filter))
                .map(|(i, h)| {
                    let line = h.line;
                    let on = Some(i) == current;
                    let pad = match h.level {
                        1 => 8.0,
                        2 => 18.0,
                        3 => 30.0,
                        _ => 42.0,
                    };
                    div()
                        .id(("md-nav-h", i))
                        .w_full()
                        .pl(px(pad))
                        .pr(px(8.0))
                        .py(px(4.0))
                        .rounded(px(5.0))
                        .border_l(px(3.0))
                        .border_color(if on { pal.accent } else { gpui::transparent_black().into() })
                        .text_size(px(match h.level {
                            1 | 2 => 13.0,
                            3 => 12.5,
                            _ => 12.0,
                        }))
                        .text_color(if h.level >= 4 { pal.text_muted } else { pal.text })
                        .when(h.level == 1 || on, |d| d.font_weight(FontWeight::SEMIBOLD))
                        .when(on, |d| d.bg(pal.accent_soft))
                        .when(!on, |d| d.hover(|d| d.bg(pal.surface_hover)))
                        .cursor_pointer()
                        .overflow_hidden()
                        .whitespace_nowrap()
                        .child(kd_ui::truncate(&h.text, 40))
                        .on_click(cx.listener(move |this, _: &ClickEvent, window, cx| {
                            this.editor.update(cx, |ed, cx| ed.go_to_line(line, window, cx));
                            this.preview.read(cx).reveal_line(line, true);
                        }))
                        .into_any_element()
                })
                .collect()
        } else {
            let mut starts = vec![0usize];
            starts.extend(md::edit::slide_markers(&text));
            starts
                .iter()
                .enumerate()
                .map(|(i, start)| {
                    let line = start + 1;
                    let next = starts.get(i + 1).map(|s| s + 1).unwrap_or(usize::MAX);
                    let title = outline
                        .iter()
                        .find(|h| h.line >= line && h.line < next)
                        .map(|h| h.text.clone())
                        .unwrap_or_else(|| format!("Slide {}", i + 1));
                    div()
                        .id(("md-nav-slide", i))
                        .flex()
                        .gap(px(6.0))
                        .p(px(5.0))
                        .rounded(px(6.0))
                        .cursor_pointer()
                        .hover(|d| d.bg(pal.surface_hover))
                        .child(
                            div()
                                .w(px(18.0))
                                .flex()
                                .justify_end()
                                .text_size(px(11.5))
                                .text_color(pal.text_muted)
                                .child((i + 1).to_string()),
                        )
                        .child(
                            div()
                                .flex_1()
                                .h(px(68.0))
                                .rounded(px(4.0))
                                .border(px(1.5))
                                .border_color(pal.border_strong)
                                .bg(gpui::white())
                                .flex()
                                .items_center()
                                .justify_center()
                                .px(px(8.0))
                                .text_size(px(12.0))
                                .font_weight(FontWeight::SEMIBOLD)
                                .text_color(gpui::black())
                                .child(kd_ui::truncate(&title, 34)),
                        )
                        .on_click(cx.listener(move |this, _: &ClickEvent, window, cx| {
                            this.editor.update(cx, |ed, cx| ed.go_to_line(line, window, cx));
                            this.preview.read(cx).reveal_line(line, true);
                        }))
                        .into_any_element()
                })
                .collect()
        };
        let empty = rows.is_empty();
        let (ic, ac) = icon_colors(pal);
        div()
            .w(px(side::PANEL_W))
            .flex_none()
            .flex()
            .flex_col()
            .bg(mix(pal.canvas, pal.surface, 0.5))
            .border_r_1()
            .border_color(pal.border)
            .child(
                div()
                    .flex()
                    .items_center()
                    .justify_between()
                    .pl(px(10.0))
                    .pr(px(8.0))
                    .pt(px(8.0))
                    .pb(px(6.0))
                    .child(
                        div()
                            .flex()
                            .rounded(px(6.0))
                            .border_1()
                            .border_color(pal.border_strong)
                            .overflow_hidden()
                            .child(seg("Headings", !self.nav_slides, false, cx))
                            .child(div().w(px(1.0)).bg(pal.border_strong))
                            .child(seg("Slides", self.nav_slides, true, cx)),
                    )
                    .child(
                        div()
                            .id("md-nav-close")
                            .size(px(26.0))
                            .flex()
                            .items_center()
                            .justify_center()
                            .rounded(px(5.0))
                            .cursor_pointer()
                            .hover(|d| d.bg(pal.surface_hover))
                            .child(md_icon("x", 16.0, ic, ac))
                            .tooltip(tooltip("Close"))
                            .on_click(cx.listener(|this, _: &ClickEvent, window, cx| this.close_side(window, cx))),
                    ),
            )
            .child(
                div()
                    .mx(px(10.0))
                    .mb(px(8.0))
                    .h(px(26.0))
                    .px(px(8.0))
                    .flex()
                    .items_center()
                    .rounded(px(5.0))
                    .border_1()
                    .border_color(pal.border_strong)
                    .bg(pal.surface)
                    .text_size(px(12.5))
                    .child(self.nav_filter.clone()),
            )
            .child(
                div()
                    .id("md-nav-body")
                    .flex_1()
                    .min_h_0()
                    .overflow_y_scroll()
                    .px(px(6.0))
                    .pb(px(16.0))
                    .flex()
                    .flex_col()
                    .children(rows)
                    .when(empty, |d| {
                        d.child(div().p(px(10.0)).text_size(px(12.5)).text_color(pal.text_muted).child(if self.nav_slides {
                            "No slides yet."
                        } else {
                            "Headings in the document show here."
                        }))
                    }),
            )
    }

    fn status_bar(&mut self, pal: &Palette, cx: &mut Context<Self>) -> impl IntoElement {
        let (line, col, dirty, slides) = {
            let ed = self.editor.read(cx);
            let (l, c) = ed.doc.line_col(ed.selection().head);
            (l, c, ed.doc.is_dirty(), md::edit::slide_markers(ed.text()).len())
        };
        let _ = slides;
        let slide_pos = {
            let pv = self.preview.read(cx);
            pv.slide_at_line(line).map(|i| (i, pv.deck.len()))
        };
        let words = md::word_count(&self.preview.read(cx).parsed);
        let save = if dirty {
            "Unsaved changes"
        } else if self.path.is_some() {
            "Saved"
        } else {
            "Not saved yet"
        };
        let item = |id: &'static str, text: String, cmd: Option<&'static str>, cx: &mut Context<Self>| {
            div().id(id).px(px(8.0)).rounded(px(4.0)).line_height(px(22.0)).child(text).when_some(cmd, |d, cmd| {
                d.cursor_pointer()
                    .hover(|d| d.bg(pal.surface_hover))
                    .on_click(cx.listener(move |this, _: &ClickEvent, window, cx| this.run(cmd, None, window, cx)))
            })
        };
        let (ic, ac) = icon_colors(pal);
        let btn = |id: &'static str, icon: &'static str, on: bool, cx: &mut Context<Self>| {
            let tip = commands::get(id).map(commands::tip).unwrap_or_default();
            div()
                .id(SharedString::from(format!("md-st-{id}")))
                .h(px(22.0))
                .min_w(px(24.0))
                .px(px(3.0))
                .flex()
                .items_center()
                .justify_center()
                .rounded(px(4.0))
                .cursor_pointer()
                .when(on, |d| d.bg(pal.accent_soft))
                .hover(|d| d.bg(pal.surface_hover))
                .child(md_icon(icon, 16.0, ic, ac))
                .tooltip(tooltip(tip))
                .on_click(cx.listener(move |this, _: &ClickEvent, window, cx| this.run(id, None, window, cx)))
        };
        let zoom = format!("{}%", (self.zoom * 100.0).round() as i32);
        div()
            .flex_none()
            .h(px(26.0))
            .flex()
            .items_center()
            .gap(px(2.0))
            .px(px(6.0))
            .bg(pal.canvas)
            .border_t_1()
            .border_color(pal.border)
            .text_size(px(12.0))
            .text_color(pal.text_muted)
            .whitespace_nowrap()
            .overflow_hidden()
            .child(item("md-st-words", format!("{words} words"), Some("review.wordcount"), cx))
            .child(item("md-st-pos", format!("Ln {line}, Col {col}"), None, cx))
            .when_some(slide_pos, |d, (i, n)| d.child(item("md-st-slides", format!("Slide {} of {n}", i + 1), None, cx)))
            .child(item("md-st-save", save.into(), None, cx))
            .child(div().flex_1())
            .child(btn("view.write", "view-write", self.mode == ViewMode::Write, cx))
            .child(btn("view.split", "view-split", self.mode == ViewMode::Split, cx))
            .child(btn("view.read", "view-read", self.mode == ViewMode::Read, cx))
            .child(btn("slides.present", "present", false, cx))
            .child(div().w(px(10.0)))
            .child(btn("view.zoomOut", "minus", false, cx))
            .child(
                div()
                    .w(px(90.0))
                    .h(px(4.0))
                    .rounded(px(2.0))
                    .bg(pal.border_strong)
                    .child(div().h_full().rounded(px(2.0)).bg(pal.accent).w(relative(((self.zoom - 0.5) / 1.5).clamp(0.0, 1.0)))),
            )
            .child(btn("view.zoomIn", "plus", false, cx))
            .child(item("md-st-zoom", zoom, Some("view.zoomReset"), cx))
    }

    fn menu_panel(&mut self, m: Menu, pal: &Palette, cx: &mut Context<Self>) -> AnyElement {
        if let Some(panel) = self.special_menu(m, pal, cx) {
            return self.menu_card(vec![panel], 0.0, pal);
        }
        let (text, head) = {
            let ed = self.editor.read(cx);
            (ed.text().to_string(), ed.selection().head)
        };
        let fm = move |k: &str| {
            if k == "\u{0}size" {
                return md::style::style_value(&text, head, "font-size");
            }
            if k == "\u{0}para" {
                return Some(md::para::style_at(&text, head).to_css());
            }
            md::edit::front_matter(&text).into_iter().find(|(key, _)| key == k).map(|(_, v)| v)
        };
        let (ic, ac) = icon_colors(pal);
        let mut children: Vec<AnyElement> = Vec::new();
        let header = |t: String| {
            div()
                .px(px(10.0))
                .pt(px(6.0))
                .pb(px(4.0))
                .text_size(px(11.5))
                .font_weight(FontWeight::SEMIBOLD)
                .text_color(pal.text_muted)
                .child(t)
        };
        match m {
            Menu::Table => {
                let (hr, hc) = self.table_hover;
                children.push(header(format!("{} × {} table", hc + 1, hr + 1)).into_any_element());
                let mut grid = Vec::new();
                for r in 0..10usize {
                    for c in 0..8usize {
                        let on = r <= hr && c <= hc;
                        grid.push(
                            div()
                                .id(("md-tcell", r * 8 + c))
                                .size(px(17.0))
                                .border_1()
                                .border_color(if on { pal.accent } else { pal.border_strong })
                                .when(on, |d| d.bg(pal.accent_soft))
                                .on_mouse_move(cx.listener(move |this, _: &gpui::MouseMoveEvent, _, cx| {
                                    if this.table_hover != (r, c) {
                                        this.table_hover = (r, c);
                                        cx.notify();
                                    }
                                }))
                                .on_click(cx.listener(move |this, _: &ClickEvent, window, cx| {
                                    this.run("insert.table", Some(format!("{}x{}", r + 1, c + 1)), window, cx)
                                }))
                                .into_any_element(),
                        );
                    }
                }
                children.push(div().px(px(10.0)).pb(px(6.0)).grid().grid_cols(8).gap(px(2.0)).children(grid).into_any_element());
            }
            Menu::Symbol => {
                children.push(header("Symbol".into()).into_any_element());
                let syms: Vec<AnyElement> = library::SYMBOLS
                    .split_whitespace()
                    .enumerate()
                    .map(|(i, s)| {
                        let s = s.to_string();
                        div()
                            .id(("md-sym", i))
                            .size(px(28.0))
                            .flex()
                            .items_center()
                            .justify_center()
                            .rounded(px(4.0))
                            .text_size(px(15.0))
                            .cursor_pointer()
                            .hover(|d| d.bg(pal.surface_hover))
                            .child(s.clone())
                            .on_click(
                                cx.listener(move |this, _: &ClickEvent, window, cx| this.run("insert.symbol", Some(s.clone()), window, cx)),
                            )
                            .into_any_element()
                    })
                    .collect();
                children.push(div().px(px(6.0)).pb(px(6.0)).grid().grid_cols(10).children(syms).into_any_element());
                return self.menu_card(children, 320.0, pal);
            }
            Menu::FontColor => {
                children.push(header("Font color".into()).into_any_element());
                const STANDARD: [u32; 10] =
                    [0xC00000, 0xFF0000, 0xFFC000, 0xFFFF00, 0x92D050, 0x00B050, 0x00B0F0, 0x0070C0, 0x002060, 0x7030A0];
                let sw: Vec<AnyElement> =
                    STANDARD
                        .iter()
                        .enumerate()
                        .map(|(i, c)| {
                            let hex = format!("#{c:06X}");
                            div()
                                .id(("md-swatch", i))
                                .size(px(20.0))
                                .rounded(px(3.0))
                                .border_1()
                                .border_color(pal.border_strong)
                                .bg(gpui::rgb(*c))
                                .cursor_pointer()
                                .tooltip(tooltip(hex.clone()))
                                .on_click(cx.listener(move |this, _: &ClickEvent, window, cx| {
                                    this.run("format.color", Some(hex.clone()), window, cx)
                                }))
                                .into_any_element()
                        })
                        .collect();
                children.push(div().px(px(10.0)).pb(px(10.0)).flex().gap(px(4.0)).children(sw).into_any_element());
                return self.menu_card(children, 260.0, pal);
            }
            _ => {}
        }
        let entries = ribbon::entries(m, &fm);
        let entries = if m == Menu::Table { entries.into_iter().skip(1).collect() } else { entries };
        for (i, e) in entries.into_iter().enumerate() {
            match e {
                Entry::Sep => children.push(div().my(px(4.0)).h(px(1.0)).bg(pal.border).into_any_element()),
                Entry::Header(t) => children.push(header(t).into_any_element()),
                Entry::Item { label, icon, sub, cmd, arg, checked } => {
                    let key = if arg.is_none() { commands::get(cmd).and_then(|c| c.key).map(commands::key_label) } else { None };
                    children.push(
                        div()
                            .id(("md-menu-item", i))
                            .flex()
                            .items_center()
                            .gap(px(8.0))
                            .mx(px(4.0))
                            .px(px(6.0))
                            .py(px(4.0))
                            .rounded(px(5.0))
                            .cursor_pointer()
                            .hover(|d| d.bg(pal.surface_hover))
                            .child(
                                div()
                                    .w(px(14.0))
                                    .flex()
                                    .justify_center()
                                    .when(checked, |d| d.child(kd_ui::icon(Icon::Check, 13.0, pal.accent))),
                            )
                            .when_some(icon, |d, icon| d.child(md_icon(icon, 18.0, ic, ac)))
                            .child(
                                div()
                                    .flex_1()
                                    .min_w_0()
                                    .flex()
                                    .flex_col()
                                    .child(div().text_size(px(13.0)).text_color(pal.text).child(label))
                                    .when_some(sub, |d, s| d.child(div().text_size(px(11.0)).text_color(pal.text_muted).child(s))),
                            )
                            .when_some(key, |d, k| d.child(div().text_size(px(11.5)).text_color(pal.text_faint).child(k)))
                            .on_click(cx.listener(move |this, _: &ClickEvent, window, cx| this.run(cmd, arg.clone(), window, cx)))
                            .into_any_element(),
                    );
                }
            }
        }
        self.menu_card(children, if m == Menu::Table { 200.0 } else { 280.0 }, pal)
    }

    fn menu_card(&self, children: Vec<AnyElement>, w: f32, pal: &Palette) -> AnyElement {
        div()
            .id("md-menu")
            .occlude()
            .min_w(px(w))
            .max_h(px(460.0))
            .overflow_y_scroll()
            .py(px(4.0))
            .rounded(px(8.0))
            .border_1()
            .border_color(pal.border)
            .bg(pal.surface)
            .shadow_lg()
            .flex()
            .flex_col()
            .children(children)
            .into_any_element()
    }

    fn search_results(&mut self, pal: &Palette, cx: &mut Context<Self>) -> Option<AnyElement> {
        let q = self.search.read(cx).text().to_string();
        if q.trim().is_empty() {
            return None;
        }
        let hits = search_commands(&q);
        let (ic, ac) = icon_colors(pal);
        let rows: Vec<AnyElement> = if hits.is_empty() {
            vec![
                div()
                    .px(px(10.0))
                    .py(px(6.0))
                    .text_size(px(12.5))
                    .text_color(pal.text_muted)
                    .child(format!("No commands match “{q}”"))
                    .into_any_element(),
            ]
        } else {
            hits.iter()
                .enumerate()
                .map(|(i, c)| {
                    let id = c.id;
                    let search = self.search.clone();
                    div()
                        .id(("md-search-hit", i))
                        .flex()
                        .items_center()
                        .gap(px(8.0))
                        .mx(px(4.0))
                        .px(px(6.0))
                        .py(px(4.0))
                        .rounded(px(5.0))
                        .cursor_pointer()
                        .when(i == 0, |d| d.bg(pal.accent_soft))
                        .hover(|d| d.bg(pal.surface_hover))
                        .child(if c.icon.contains('/') {
                            svg().path(c.icon).size(px(17.0)).text_color(pal.accent).into_any_element()
                        } else {
                            md_icon(c.icon, 18.0, ic, ac)
                        })
                        .child(div().flex_1().text_size(px(13.0)).text_color(pal.text).child(c.label))
                        .when_some(c.key, |d, k| {
                            d.child(div().text_size(px(11.5)).text_color(pal.text_faint).child(commands::key_label(k)))
                        })
                        .on_click(cx.listener(move |this, _: &ClickEvent, window, cx| {
                            search.update(cx, |s, cx| s.clear(cx));
                            this.run(id, None, window, cx)
                        }))
                        .into_any_element()
                })
                .collect()
        };
        Some(
            deferred(
                anchored()
                    .position(point(px(0.0), px(0.0)))
                    .snap_to_window_with_margin(px(8.0))
                    .child(div().mt(px(34.0)).w(px(300.0)).child(self.menu_card(rows, 300.0, pal))),
            )
            .with_priority(3)
            .into_any_element(),
        )
    }
}

impl Render for MarkdownView {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let pal = Theme::palette(cx);
        let show_top = !self.focus_mode;
        let show_ribbon = show_top && (!self.collapsed || self.peek);
        let tabstrip = show_top.then(|| self.tabstrip(&pal, cx).into_any_element());
        let ribbon = show_ribbon.then(|| self.ribbon(&pal, cx).into_any_element());
        let side_bar = (!self.focus_mode).then(|| self.side_bar(&pal, cx));
        let nav = self.side_panel(&pal, cx);
        let status = (!self.focus_mode).then(|| self.status_bar(&pal, cx).into_any_element());
        let results = self.search_results(&pal, cx);
        let menu = self.menu.as_ref().map(|m| (m.menu, m.at));
        let menu = menu.map(|(m, at)| {
            let card = self.menu_panel(m, &pal, cx);
            deferred(
                anchored().position(at).snap_to_window_with_margin(px(8.0)).child(
                    div()
                        .on_mouse_down_out(cx.listener(|this, _, _, cx| {
                            this.menu = None;
                            cx.notify();
                        }))
                        .child(card),
                ),
            )
            .with_priority(3)
        });
        let template = self.template_bar(&pal, cx);
        let prompt = self.prompt_card(&pal, cx);
        let info = self.info_card(&pal, cx);
        let present = self.present_overlay(window, cx);
        let chooser = self.chooser_card(&pal, cx);
        let right_bar = (!self.focus_mode).then(|| self.right_bar(&pal, cx));
        let right_panel = self.right_panel(&pal, window, cx);
        let para_dialog = self.para_dialog(&pal, window, cx);
        let props_dialog = self.props_dialog(&pal, cx);
        let more_colors = self.more_colors_card(&pal, cx);
        let browser = self.browser_card(&pal, cx);
        let split = self.split;
        let editor_pane = (self.mode != ViewMode::Read).then(|| {
            div()
                .min_w_0()
                .h_full()
                .flex()
                .flex_col()
                .when(self.mode == ViewMode::Split, |d| d.w(relative(split)).flex_none())
                .when(self.mode == ViewMode::Write, |d| d.flex_1())
                .child(div().flex_1().min_h_0().child(self.editor.clone()))
        });
        let dragging = self.dragging;
        let divider = (self.mode == ViewMode::Split).then(|| {
            div()
                .id("md-split")
                .w(px(5.0))
                .h_full()
                .flex_none()
                .bg(if dragging { pal.accent } else { pal.border })
                .hover(|d| d.bg(pal.accent))
                .cursor(gpui::CursorStyle::ResizeLeftRight)
                .tooltip(tooltip("Drag to resize; double-click to reset"))
                .on_mouse_down(
                    MouseButton::Left,
                    cx.listener(|this, e: &gpui::MouseDownEvent, _, cx| {
                        if e.click_count == 2 {
                            this.split = 0.5;
                        } else {
                            this.dragging = true;
                        }
                        cx.stop_propagation();
                        cx.notify();
                    }),
                )
        });
        let preview_pane = (self.mode != ViewMode::Write).then(|| div().flex_1().min_w_0().h_full().child(self.preview.clone()));
        let bounds_cell = self.main_bounds.clone();
        let measure = gpui::canvas(move |b, _, _| bounds_cell.set(b), |_, _, _, _| {}).absolute().size_full();
        div()
            .key_context(CONTEXT)
            .track_focus(&self.focus)
            .relative()
            .size_full()
            .flex()
            .flex_col()
            .bg(pal.canvas)
            .text_color(pal.text)
            .on_action(cx.listener(|this, a: &RunMd, window, cx| this.run(a.id, None, window, cx)))
            .on_key_down(cx.listener(|this, e: &gpui::KeyDownEvent, window, cx| {
                // Esc closes the File tab's windows and the cards
                if e.keystroke.key == "escape" && this.more_colors.is_some() {
                    this.more_colors = None;
                    cx.stop_propagation();
                    cx.notify();
                    return;
                }
                if e.keystroke.key == "escape" && this.props_ui.tab.is_some() && this.menu.is_none() {
                    this.close_props(window, cx);
                    cx.stop_propagation();
                    return;
                }
                if e.keystroke.key == "escape" && this.para_ui.advanced.is_some() {
                    this.para_ui.advanced = None;
                    cx.stop_propagation();
                    cx.notify();
                    return;
                }
                if e.keystroke.key == "escape" && (this.chooser || this.browser.is_some() || this.info.is_some()) {
                    this.chooser = false;
                    this.browser = None;
                    this.info = None;
                    this.focus_editor(window, cx);
                    cx.stop_propagation();
                    cx.notify();
                }
            }))
            .children(tabstrip)
            .children(ribbon)
            .children(template)
            .child(
                div()
                    .relative()
                    .flex_1()
                    .min_h_0()
                    .flex()
                    .child(measure)
                    .children(side_bar)
                    .children(nav)
                    .children(editor_pane)
                    .children(divider)
                    .children(preview_pane)
                    .children(right_panel)
                    .children(right_bar)
                    .when(dragging, |d| {
                        d.on_mouse_move(cx.listener(|this, e: &gpui::MouseMoveEvent, _, cx| {
                            let b = this.main_bounds.get();
                            let nav = this.side_width();
                            let right = this.right_width();
                            let w = f32::from(b.size.width) - nav - right;
                            if w > 100.0 {
                                let x = f32::from(e.position.x - b.left()) - nav;
                                this.split = (x / w).clamp(0.15, 0.85);
                                cx.notify();
                            }
                        }))
                        .on_mouse_up(
                            MouseButton::Left,
                            cx.listener(|this, _, _, cx| {
                                this.dragging = false;
                                cx.notify();
                            }),
                        )
                    }),
            )
            .children(status)
            .children(results)
            .children(menu)
            .children(prompt)
            .children(info)
            .children(chooser)
            .children(para_dialog)
            .children(props_dialog)
            .children(more_colors)
            .children(browser)
            .children(present)
    }
}

#[cfg(test)]
mod tests {
    use super::search_commands;

    #[test]
    fn command_search_ranks_label_starts_first() {
        let hits = search_commands("tab");
        assert_eq!(hits[0].id, "insert.table");
        assert!(search_commands("zzzz").is_empty());
        assert!(search_commands("  ").is_empty());
    }
}

/// What a document holds, for Document info.
#[derive(Default)]
struct Counts {
    tables: usize,
    pictures: usize,
    media: usize,
    diagrams: usize,
    links: usize,
    includes: usize,
}

impl Counts {
    fn inlines(&mut self, inl: &[md::Inline]) {
        let mut last_link: Option<&str> = None;
        for i in inl {
            match i {
                md::Inline::Image { url, .. } if md::media::kind(url).is_some() => self.media += 1,
                md::Inline::Image { .. } => self.pictures += 1,
                md::Inline::Text(_, s) => {
                    if let Some(l) = s.link.as_deref()
                        && last_link != Some(l)
                    {
                        self.links += 1;
                    }
                    last_link = s.link.as_deref();
                }
                _ => last_link = None,
            }
        }
    }

    fn walk(&mut self, blocks: &[md::Block]) {
        for b in blocks {
            match b {
                md::Block::Paragraph { inlines, .. } => {
                    if md::media::import_of(inlines).is_some() {
                        self.includes += 1;
                    } else {
                        self.inlines(inlines);
                    }
                }
                md::Block::Heading { inlines, .. } => self.inlines(inlines),
                md::Block::List { items, .. } => {
                    for it in items {
                        self.walk(&it.blocks);
                    }
                }
                md::Block::Quote { blocks, .. } | md::Block::Callout { blocks, .. } | md::Block::Styled { blocks, .. } => self.walk(blocks),
                md::Block::Table { header, rows, .. } => {
                    self.tables += 1;
                    for c in header.iter().chain(rows.iter().flatten()) {
                        self.inlines(c);
                    }
                }
                md::Block::Code { info, text, .. } if md::diagram::supported(info, text) => self.diagrams += 1,
                md::Block::Html { text, .. } if md::media::html_media(text).is_some() => self.media += 1,
                _ => {}
            }
        }
    }
}

/// 1 234 bytes → "1.2 KB".
fn human_size(n: u64) -> String {
    match n {
        n if n < 1024 => format!("{n} bytes"),
        n if n < 1024 * 1024 => format!("{:.1} KB", n as f64 / 1024.0),
        n => format!("{:.1} MB", n as f64 / (1024.0 * 1024.0)),
    }
}

/// A folder with the home folder as `~`.
fn tilde(p: &std::path::Path) -> String {
    let home = std::env::var_os("HOME").or_else(|| std::env::var_os("USERPROFILE")).map(std::path::PathBuf::from);
    match home.and_then(|h| p.strip_prefix(&h).ok().map(|r| r.to_path_buf())) {
        Some(r) if r.as_os_str().is_empty() => "~".into(),
        Some(r) => format!("~{}{}", std::path::MAIN_SEPARATOR, r.display()),
        None => p.display().to_string(),
    }
}
