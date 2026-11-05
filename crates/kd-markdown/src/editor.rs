//! The Markdown source editor, drawn with GPUI: syntax colours, line numbers, word wrap, the
//! active line, selection, IME, clipboard, undo / redo (kd-doc) and list continuation on Enter,
//! as MdCraft's editor does.

use std::ops::Range;

use gpui::{
    App, Bounds, ClipboardItem, ContentMask, Context, CursorStyle, Element, ElementId, ElementInputHandler, Entity, EntityInputHandler,
    EventEmitter, FocusHandle, Focusable, Font, FontStyle, FontWeight, GlobalElementId, Hsla, KeyBinding, LayoutId, MouseButton,
    MouseDownEvent, MouseMoveEvent, MouseUpEvent, Pixels, Point, ScrollHandle, SharedString, Size, Style, TextAlign, TextRun,
    UTF16Selection, UnderlineStyle, Window, WrappedLine, actions, div, fill, point, prelude::*, px, relative, size,
};
use kd_doc::{Change, Document, EditKind, Selection};
use kd_formats::md::{self, Edit, LineToks, Tok};
use kd_ui::{Palette, Theme, mix};
use unicode_segmentation::UnicodeSegmentation;

actions!(
    kd_editor,
    [
        Left,
        Right,
        Up,
        Down,
        SelectLeft,
        SelectRight,
        SelectUp,
        SelectDown,
        WordLeft,
        WordRight,
        SelectWordLeft,
        SelectWordRight,
        LineStart,
        LineEnd,
        SelectLineStart,
        SelectLineEnd,
        DocStart,
        DocEnd,
        SelectDocStart,
        SelectDocEnd,
        PageUp,
        PageDown,
        Backspace,
        Delete,
        BackspaceWord,
        DeleteWord,
        Enter,
        Tab,
        ShiftTab,
        SelectAll,
        Copy,
        Cut,
        Paste,
        Undo,
        Redo
    ]
);

const CONTEXT: &str = "KdEditor";

/// The editor's keys; call once at start-up (and again after the key map is rebound).
pub fn bind_keys(cx: &mut App) {
    let c = Some(CONTEXT);
    let mut keys = vec![
        KeyBinding::new("left", Left, c),
        KeyBinding::new("right", Right, c),
        KeyBinding::new("up", Up, c),
        KeyBinding::new("down", Down, c),
        KeyBinding::new("shift-left", SelectLeft, c),
        KeyBinding::new("shift-right", SelectRight, c),
        KeyBinding::new("shift-up", SelectUp, c),
        KeyBinding::new("shift-down", SelectDown, c),
        KeyBinding::new("home", LineStart, c),
        KeyBinding::new("end", LineEnd, c),
        KeyBinding::new("shift-home", SelectLineStart, c),
        KeyBinding::new("shift-end", SelectLineEnd, c),
        KeyBinding::new("pageup", PageUp, c),
        KeyBinding::new("pagedown", PageDown, c),
        KeyBinding::new("backspace", Backspace, c),
        KeyBinding::new("shift-backspace", Backspace, c),
        KeyBinding::new("delete", Delete, c),
        KeyBinding::new("enter", Enter, c),
        KeyBinding::new("shift-enter", Enter, c),
        KeyBinding::new("tab", Tab, c),
        KeyBinding::new("shift-tab", ShiftTab, c),
        KeyBinding::new("secondary-a", SelectAll, c),
        KeyBinding::new("secondary-c", Copy, c),
        KeyBinding::new("secondary-x", Cut, c),
        KeyBinding::new("secondary-v", Paste, c),
        KeyBinding::new("secondary-z", Undo, c),
        KeyBinding::new("secondary-shift-z", Redo, c),
    ];
    if cfg!(target_os = "macos") {
        keys.extend([
            KeyBinding::new("alt-left", WordLeft, c),
            KeyBinding::new("alt-right", WordRight, c),
            KeyBinding::new("alt-shift-left", SelectWordLeft, c),
            KeyBinding::new("alt-shift-right", SelectWordRight, c),
            KeyBinding::new("cmd-left", LineStart, c),
            KeyBinding::new("cmd-right", LineEnd, c),
            KeyBinding::new("cmd-shift-left", SelectLineStart, c),
            KeyBinding::new("cmd-shift-right", SelectLineEnd, c),
            KeyBinding::new("cmd-up", DocStart, c),
            KeyBinding::new("cmd-down", DocEnd, c),
            KeyBinding::new("cmd-shift-up", SelectDocStart, c),
            KeyBinding::new("cmd-shift-down", SelectDocEnd, c),
            KeyBinding::new("alt-backspace", BackspaceWord, c),
            KeyBinding::new("alt-delete", DeleteWord, c),
        ]);
    } else {
        keys.extend([
            KeyBinding::new("ctrl-left", WordLeft, c),
            KeyBinding::new("ctrl-right", WordRight, c),
            KeyBinding::new("ctrl-shift-left", SelectWordLeft, c),
            KeyBinding::new("ctrl-shift-right", SelectWordRight, c),
            KeyBinding::new("ctrl-home", DocStart, c),
            KeyBinding::new("ctrl-end", DocEnd, c),
            KeyBinding::new("ctrl-shift-home", SelectDocStart, c),
            KeyBinding::new("ctrl-shift-end", SelectDocEnd, c),
            KeyBinding::new("ctrl-backspace", BackspaceWord, c),
            KeyBinding::new("ctrl-delete", DeleteWord, c),
            KeyBinding::new("ctrl-y", Redo, c),
        ]);
    }
    cx.bind_keys(keys);
}

/// The monospace font of the editor.
pub fn mono_family() -> SharedString {
    if cfg!(target_os = "macos") {
        "Menlo".into()
    } else if cfg!(windows) {
        "Consolas".into()
    } else {
        "DejaVu Sans Mono".into()
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum EditorEvent {
    /// The text changed.
    Changed,
    /// The cursor or selection moved.
    Moved,
}

/// One logical line as laid out at the last paint.
struct LineLayout {
    start: usize,
    y: Pixels,
    rows: usize,
    line: WrappedLine,
}

pub struct SourceEditor {
    focus: FocusHandle,
    pub doc: Document,
    pub line_numbers: bool,
    pub wrap: bool,
    pub font_size: Pixels,
    pub scroll: ScrollHandle,
    toks: Vec<LineToks>,
    toks_version: u64,
    /// Find: the matches to show, and which one is current.
    pub marks: Vec<Range<usize>>,
    pub mark_current: Option<usize>,
    marked: Option<Range<usize>>,
    selecting: bool,
    /// Column kept while moving up and down.
    goal_x: Option<Pixels>,
    /// Bring the cursor into view at the next paint.
    reveal: bool,
    // from the last paint
    layout: Vec<LineLayout>,
    text_origin: Point<Pixels>,
    line_height: Pixels,
    bounds: Option<Bounds<Pixels>>,
}

impl EventEmitter<EditorEvent> for SourceEditor {}

impl SourceEditor {
    pub fn new(text: &str, cx: &mut Context<Self>) -> Self {
        SourceEditor {
            focus: cx.focus_handle(),
            doc: Document::new(text),
            line_numbers: false,
            wrap: true,
            font_size: px(15.0),
            scroll: ScrollHandle::new(),
            toks: Vec::new(),
            toks_version: u64::MAX,
            marks: Vec::new(),
            mark_current: None,
            marked: None,
            selecting: false,
            goal_x: None,
            reveal: false,
            layout: Vec::new(),
            text_origin: point(px(0.0), px(0.0)),
            line_height: px(24.0),
            bounds: None,
        }
    }

    pub fn text(&self) -> &str {
        self.doc.text()
    }

    pub fn selection(&self) -> Selection {
        self.doc.selection()
    }

    pub fn is_focused(&self, window: &Window) -> bool {
        self.focus.is_focused(window)
    }

    pub fn focus(&self, window: &mut Window) {
        window.focus(&self.focus);
    }

    // ---- editing API (commands use these) ------------------------------------------------

    /// Apply an edit from kd-formats (one undo step).
    pub fn apply_edit(&mut self, e: Edit, cx: &mut Context<Self>) {
        if self.doc.apply(e.changes, e.selection, EditKind::Other) {
            self.changed(cx);
        }
    }

    /// Replace the selection with `text` (with optional SEL marks).
    pub fn insert(&mut self, text: &str, cx: &mut Context<Self>) {
        let e = md::edit::insert(self.doc.text(), self.doc.selection(), text);
        self.apply_edit(e, cx);
    }

    pub fn set_selection(&mut self, s: Selection, cx: &mut Context<Self>) {
        self.doc.set_selection(s);
        self.goal_x = None;
        self.reveal = true;
        cx.emit(EditorEvent::Moved);
        cx.notify();
    }

    /// Put the cursor at the start of a 1-based line and show it.
    pub fn go_to_line(&mut self, line: usize, window: &mut Window, cx: &mut Context<Self>) {
        let at = self.doc.line_start(line.saturating_sub(1));
        self.set_selection(Selection::cursor(at), cx);
        window.focus(&self.focus);
    }

    pub fn undo(&mut self, cx: &mut Context<Self>) {
        if self.doc.undo() {
            self.changed(cx);
        }
    }

    pub fn redo(&mut self, cx: &mut Context<Self>) {
        if self.doc.redo() {
            self.changed(cx);
        }
    }

    pub fn selected_text(&self) -> &str {
        &self.doc.text()[self.doc.selection().range()]
    }

    /// Copy the selection to the clipboard.
    pub fn copy_selection(&mut self, cx: &mut Context<Self>) {
        let t = self.selected_text();
        if !t.is_empty() {
            cx.write_to_clipboard(ClipboardItem::new_string(t.to_string()));
        }
    }

    /// Cut the selection to the clipboard.
    pub fn cut_selection(&mut self, cx: &mut Context<Self>) {
        let s = self.doc.selection();
        if !s.is_empty() {
            self.copy_selection(cx);
            self.replace(s.range(), "", EditKind::Other, cx);
        }
    }

    /// Paste the clipboard's text over the selection.
    pub fn paste_clipboard(&mut self, cx: &mut Context<Self>) {
        if let Some(t) = cx.read_from_clipboard().and_then(|c| c.text()) {
            let t = t.replace("\r\n", "\n");
            let s = self.doc.selection();
            self.replace(s.range(), &t, EditKind::Other, cx);
        }
    }

    pub fn select_everything(&mut self, cx: &mut Context<Self>) {
        let n = self.text_len();
        self.set_selection(Selection::new(0, n), cx);
    }

    /// Replace the whole text (one undo step), keeping the cursor where it can.
    pub fn set_text(&mut self, text: &str, cx: &mut Context<Self>) {
        self.doc.set_text(text);
        self.changed(cx);
    }

    fn changed(&mut self, cx: &mut Context<Self>) {
        self.goal_x = None;
        self.reveal = true;
        cx.emit(EditorEvent::Changed);
        cx.notify();
    }

    fn replace(&mut self, range: Range<usize>, text: &str, kind: EditKind, cx: &mut Context<Self>) {
        let at = range.start + text.len();
        if self.doc.apply(vec![Change::new(range.start, range.end, text)], Selection::cursor(at), kind) {
            self.changed(cx);
        }
    }

    // ---- movement --------------------------------------------------------------------------

    fn text_len(&self) -> usize {
        self.doc.len()
    }

    fn prev_boundary(&self, at: usize) -> usize {
        self.doc.text()[..at].grapheme_indices(true).next_back().map(|(i, _)| i).unwrap_or(0)
    }

    fn next_boundary(&self, at: usize) -> usize {
        self.doc.text()[at..].graphemes(true).next().map(|g| at + g.len()).unwrap_or(self.text_len())
    }

    fn word_left(&self, at: usize) -> usize {
        let t = &self.doc.text()[..at];
        let trimmed = t.trim_end();
        trimmed
            .char_indices()
            .rev()
            .take_while(|(_, c)| c.is_alphanumeric() || *c == '_')
            .last()
            .map(|(i, _)| i)
            .unwrap_or_else(|| if trimmed.len() < t.len() { trimmed.len() } else { self.prev_boundary(at) })
    }

    fn word_right(&self, at: usize) -> usize {
        let t = &self.doc.text()[at..];
        let skip = t.len() - t.trim_start().len();
        let rest = &t[skip..];
        let n = rest.char_indices().find(|(_, c)| !(c.is_alphanumeric() || *c == '_')).map(|(i, _)| i).unwrap_or(rest.len());
        if n == 0 && skip == 0 { self.next_boundary(at) } else { at + skip + n }
    }

    fn line_bounds(&self, at: usize) -> (usize, usize) {
        md::edit::line_at(self.doc.text(), at)
    }

    fn move_to(&mut self, at: usize, select: bool, cx: &mut Context<Self>) {
        let s = self.doc.selection();
        let sel = if select { Selection::new(s.anchor, at) } else { Selection::cursor(at) };
        self.doc.set_selection(sel);
        self.reveal = true;
        cx.emit(EditorEvent::Moved);
        cx.notify();
    }

    fn horizontal(&mut self, dir: i32, select: bool, word: bool, cx: &mut Context<Self>) {
        let s = self.doc.selection();
        self.goal_x = None;
        if !select && !s.is_empty() && !word {
            let at = if dir < 0 { s.start() } else { s.end() };
            return self.move_to(at, false, cx);
        }
        let at = match (dir < 0, word) {
            (true, false) => self.prev_boundary(s.head),
            (false, false) => self.next_boundary(s.head),
            (true, true) => self.word_left(s.head),
            (false, true) => self.word_right(s.head),
        };
        self.move_to(at, select, cx);
    }

    fn vertical(&mut self, rows: i32, select: bool, cx: &mut Context<Self>) {
        let s = self.doc.selection();
        let p = self.position_for(s.head);
        let x = *self.goal_x.get_or_insert(p.x);
        let y = p.y + self.line_height * rows as f32 + self.line_height / 2.0;
        let at = if y < px(0.0) { 0 } else { self.offset_for(point(x, y)) };
        let goal = self.goal_x;
        self.move_to(at, select, cx);
        self.goal_x = goal;
    }

    // ---- geometry (last paint) ---------------------------------------------------------------

    /// Position of `offset` relative to the text's top-left.
    fn position_for(&self, offset: usize) -> Point<Pixels> {
        for (i, l) in self.layout.iter().enumerate() {
            let end = l.start + l.line.len();
            let last = i + 1 == self.layout.len();
            if offset >= l.start && (offset <= end || last) {
                let p = l.line.position_for_index((offset - l.start).min(l.line.len()), self.line_height).unwrap_or_default();
                return point(p.x, l.y + p.y);
            }
        }
        point(px(0.0), px(0.0))
    }

    /// The offset nearest to `p` (relative to the text's top-left).
    fn offset_for(&self, p: Point<Pixels>) -> usize {
        if p.y < px(0.0) {
            return 0;
        }
        for l in &self.layout {
            let h = self.line_height * l.rows as f32;
            if p.y < l.y + h {
                let local = point(p.x.max(px(0.0)), p.y - l.y);
                let ix = match l.line.closest_index_for_position(local, self.line_height) {
                    Ok(i) | Err(i) => i,
                };
                return self.doc.snap(l.start + ix);
            }
        }
        self.text_len()
    }

    fn offset_for_mouse(&self, p: Point<Pixels>) -> usize {
        self.offset_for(point(p.x - self.text_origin.x, p.y - self.text_origin.y))
    }

    // ---- actions ---------------------------------------------------------------------------

    fn left(&mut self, _: &Left, _: &mut Window, cx: &mut Context<Self>) {
        self.horizontal(-1, false, false, cx)
    }
    fn right(&mut self, _: &Right, _: &mut Window, cx: &mut Context<Self>) {
        self.horizontal(1, false, false, cx)
    }
    fn select_left(&mut self, _: &SelectLeft, _: &mut Window, cx: &mut Context<Self>) {
        self.horizontal(-1, true, false, cx)
    }
    fn select_right(&mut self, _: &SelectRight, _: &mut Window, cx: &mut Context<Self>) {
        self.horizontal(1, true, false, cx)
    }
    fn word_left_a(&mut self, _: &WordLeft, _: &mut Window, cx: &mut Context<Self>) {
        self.horizontal(-1, false, true, cx)
    }
    fn word_right_a(&mut self, _: &WordRight, _: &mut Window, cx: &mut Context<Self>) {
        self.horizontal(1, false, true, cx)
    }
    fn select_word_left(&mut self, _: &SelectWordLeft, _: &mut Window, cx: &mut Context<Self>) {
        self.horizontal(-1, true, true, cx)
    }
    fn select_word_right(&mut self, _: &SelectWordRight, _: &mut Window, cx: &mut Context<Self>) {
        self.horizontal(1, true, true, cx)
    }
    fn up(&mut self, _: &Up, _: &mut Window, cx: &mut Context<Self>) {
        self.vertical(-1, false, cx)
    }
    fn down(&mut self, _: &Down, _: &mut Window, cx: &mut Context<Self>) {
        self.vertical(1, false, cx)
    }
    fn select_up(&mut self, _: &SelectUp, _: &mut Window, cx: &mut Context<Self>) {
        self.vertical(-1, true, cx)
    }
    fn select_down(&mut self, _: &SelectDown, _: &mut Window, cx: &mut Context<Self>) {
        self.vertical(1, true, cx)
    }
    fn page_up(&mut self, _: &PageUp, _: &mut Window, cx: &mut Context<Self>) {
        let rows = (f32::from(self.scroll.bounds().size.height) / f32::from(self.line_height)).max(2.0) as i32 - 1;
        self.vertical(-rows, false, cx)
    }
    fn page_down(&mut self, _: &PageDown, _: &mut Window, cx: &mut Context<Self>) {
        let rows = (f32::from(self.scroll.bounds().size.height) / f32::from(self.line_height)).max(2.0) as i32 - 1;
        self.vertical(rows, false, cx)
    }
    fn line_start(&mut self, _: &LineStart, _: &mut Window, cx: &mut Context<Self>) {
        let at = self.smart_home();
        self.move_to(at, false, cx)
    }
    fn line_end(&mut self, _: &LineEnd, _: &mut Window, cx: &mut Context<Self>) {
        let at = self.line_bounds(self.doc.selection().head).1;
        self.move_to(at, false, cx)
    }
    fn select_line_start(&mut self, _: &SelectLineStart, _: &mut Window, cx: &mut Context<Self>) {
        let at = self.smart_home();
        self.move_to(at, true, cx)
    }
    fn select_line_end(&mut self, _: &SelectLineEnd, _: &mut Window, cx: &mut Context<Self>) {
        let at = self.line_bounds(self.doc.selection().head).1;
        self.move_to(at, true, cx)
    }
    fn doc_start(&mut self, _: &DocStart, _: &mut Window, cx: &mut Context<Self>) {
        self.move_to(0, false, cx)
    }
    fn doc_end(&mut self, _: &DocEnd, _: &mut Window, cx: &mut Context<Self>) {
        let n = self.text_len();
        self.move_to(n, false, cx)
    }
    fn select_doc_start(&mut self, _: &SelectDocStart, _: &mut Window, cx: &mut Context<Self>) {
        self.move_to(0, true, cx)
    }
    fn select_doc_end(&mut self, _: &SelectDocEnd, _: &mut Window, cx: &mut Context<Self>) {
        let n = self.text_len();
        self.move_to(n, true, cx)
    }

    /// Home goes to the first non-space character, then to the line start.
    fn smart_home(&self) -> usize {
        let head = self.doc.selection().head;
        let (s, e) = self.line_bounds(head);
        let first = s + (self.doc.text()[s..e].len() - self.doc.text()[s..e].trim_start().len());
        if head == first { s } else { first }
    }

    fn backspace(&mut self, _: &Backspace, _: &mut Window, cx: &mut Context<Self>) {
        let s = self.doc.selection();
        let range = if s.is_empty() { self.prev_boundary(s.head)..s.head } else { s.range() };
        if !range.is_empty() {
            self.replace(range, "", EditKind::Deleting, cx);
        }
    }

    fn delete(&mut self, _: &Delete, _: &mut Window, cx: &mut Context<Self>) {
        let s = self.doc.selection();
        let range = if s.is_empty() { s.head..self.next_boundary(s.head) } else { s.range() };
        if !range.is_empty() {
            self.replace(range, "", EditKind::Deleting, cx);
        }
    }

    fn backspace_word(&mut self, _: &BackspaceWord, _: &mut Window, cx: &mut Context<Self>) {
        let s = self.doc.selection();
        let range = if s.is_empty() { self.word_left(s.head)..s.head } else { s.range() };
        if !range.is_empty() {
            self.replace(range, "", EditKind::Other, cx);
        }
    }

    fn delete_word(&mut self, _: &DeleteWord, _: &mut Window, cx: &mut Context<Self>) {
        let s = self.doc.selection();
        let range = if s.is_empty() { s.head..self.word_right(s.head) } else { s.range() };
        if !range.is_empty() {
            self.replace(range, "", EditKind::Other, cx);
        }
    }

    /// Enter: a new line that continues the list or quote; on an empty item, ends the list.
    fn enter(&mut self, _: &Enter, _: &mut Window, cx: &mut Context<Self>) {
        let s = self.doc.selection();
        let (ls, _) = self.line_bounds(s.start());
        let line = &self.doc.text()[ls..s.start()];
        let (cont, empty_item) = continuation(line);
        if empty_item {
            // "- " alone: remove the marker instead of adding another
            let e = Edit { changes: vec![Change::new(ls, s.end(), "")], selection: Selection::cursor(ls) };
            return self.apply_edit(e, cx);
        }
        let ins = format!("\n{cont}");
        let at = s.start() + ins.len();
        if self.doc.apply(vec![Change::new(s.start(), s.end(), ins)], Selection::cursor(at), EditKind::Other) {
            self.changed(cx);
        }
    }

    fn tab(&mut self, _: &Tab, _: &mut Window, cx: &mut Context<Self>) {
        let s = self.doc.selection();
        let text = self.doc.text();
        let (ls, le) = self.line_bounds(s.start());
        let in_list = md::edit::active_formats(text, s.start());
        if !s.is_empty() || in_list.bullet || in_list.number || in_list.task {
            if let Some(e) = md::edit::indent(text, s, true) {
                self.apply_edit(e, cx);
            }
        } else {
            let _ = (ls, le);
            self.replace(s.range(), "  ", EditKind::Typing, cx);
        }
    }

    fn shift_tab(&mut self, _: &ShiftTab, _: &mut Window, cx: &mut Context<Self>) {
        if let Some(e) = md::edit::indent(self.doc.text(), self.doc.selection(), false) {
            self.apply_edit(e, cx);
        }
    }

    fn select_all(&mut self, _: &SelectAll, _: &mut Window, cx: &mut Context<Self>) {
        let n = self.text_len();
        self.doc.set_selection(Selection::new(0, n));
        cx.emit(EditorEvent::Moved);
        cx.notify();
    }

    fn copy(&mut self, _: &Copy, _: &mut Window, cx: &mut Context<Self>) {
        let t = self.selected_text();
        if !t.is_empty() {
            cx.write_to_clipboard(ClipboardItem::new_string(t.to_string()));
        }
    }

    fn cut(&mut self, _: &Cut, _: &mut Window, cx: &mut Context<Self>) {
        let s = self.doc.selection();
        if !s.is_empty() {
            cx.write_to_clipboard(ClipboardItem::new_string(self.selected_text().to_string()));
            self.replace(s.range(), "", EditKind::Other, cx);
        }
    }

    fn paste(&mut self, _: &Paste, _: &mut Window, cx: &mut Context<Self>) {
        if let Some(t) = cx.read_from_clipboard().and_then(|c| c.text()) {
            let t = t.replace("\r\n", "\n");
            let s = self.doc.selection();
            self.replace(s.range(), &t, EditKind::Other, cx);
        }
    }

    fn undo_a(&mut self, _: &Undo, _: &mut Window, cx: &mut Context<Self>) {
        self.undo(cx)
    }

    fn redo_a(&mut self, _: &Redo, _: &mut Window, cx: &mut Context<Self>) {
        self.redo(cx)
    }

    fn on_mouse_down(&mut self, e: &MouseDownEvent, window: &mut Window, cx: &mut Context<Self>) {
        window.focus(&self.focus);
        let at = self.offset_for_mouse(e.position);
        self.goal_x = None;
        match e.click_count {
            2 => {
                let (a, b) = md::edit::word_at(self.doc.text(), at).unwrap_or((at, at));
                self.doc.set_selection(Selection::new(a, b));
            }
            n if n >= 3 => {
                let (a, b) = self.line_bounds(at);
                let b = if b < self.text_len() { b + 1 } else { b };
                self.doc.set_selection(Selection::new(a, b));
            }
            _ => {
                let s = self.doc.selection();
                self.doc.set_selection(if e.modifiers.shift { Selection::new(s.anchor, at) } else { Selection::cursor(at) });
                self.selecting = true;
            }
        }
        cx.emit(EditorEvent::Moved);
        cx.notify();
    }

    fn on_mouse_up(&mut self, _: &MouseUpEvent, _: &mut Window, _: &mut Context<Self>) {
        self.selecting = false;
    }

    fn on_mouse_move(&mut self, e: &MouseMoveEvent, _: &mut Window, cx: &mut Context<Self>) {
        if self.selecting {
            let at = self.offset_for_mouse(e.position);
            let s = self.doc.selection();
            if s.head != at {
                self.doc.set_selection(Selection::new(s.anchor, at));
                self.reveal = true;
                cx.emit(EditorEvent::Moved);
                cx.notify();
            }
        }
    }

    // ---- UTF-16 for the platform input handler -----------------------------------------------

    fn to_utf16(&self, offset: usize) -> usize {
        self.doc.text()[..offset.min(self.text_len())].encode_utf16().count()
    }

    fn offset_from_utf16(&self, offset: usize) -> usize {
        let mut n = 0;
        for (i, c) in self.doc.text().char_indices() {
            if n >= offset {
                return i;
            }
            n += c.len_utf16();
        }
        self.text_len()
    }

    fn range_to_utf16(&self, r: &Range<usize>) -> Range<usize> {
        self.to_utf16(r.start)..self.to_utf16(r.end)
    }

    fn range_from_utf16(&self, r: &Range<usize>) -> Range<usize> {
        self.offset_from_utf16(r.start)..self.offset_from_utf16(r.end)
    }
}

/// What a new line after `line` starts with: the list or quote marker to continue, and
/// whether `line` is an empty item (Enter then ends the list).
pub fn continuation(line: &str) -> (String, bool) {
    let indent = &line[..line.len() - line.trim_start().len()];
    let t = line.trim_start();
    let mut quote = String::new();
    let mut rest = t;
    while let Some(r) = rest.strip_prefix('>') {
        quote.push_str("> ");
        rest = r.trim_start();
    }
    let (marker, body) =
        if let Some(r) = rest.strip_prefix("- [ ] ").or_else(|| rest.strip_prefix("- [x] ")).or_else(|| rest.strip_prefix("- [X] ")) {
            ("- [ ] ".to_string(), r)
        } else if let Some(r) = rest.strip_prefix("- ").or_else(|| rest.strip_prefix("* ")).or_else(|| rest.strip_prefix("+ ")) {
            (format!("{} ", &rest[..1]), r)
        } else {
            let d = rest.chars().take_while(|c| c.is_ascii_digit()).count();
            if d > 0 && (rest[d..].starts_with(". ") || rest[d..].starts_with(") ")) {
                let n: usize = rest[..d].parse().unwrap_or(1);
                (format!("{}{} ", n + 1, &rest[d..d + 1]), &rest[d + 2..])
            } else {
                (String::new(), rest)
            }
        };
    if marker.is_empty() && quote.is_empty() {
        return (indent.to_string(), false);
    }
    let empty = body.trim().is_empty();
    (format!("{indent}{quote}{marker}"), empty && !marker.is_empty())
}

impl EntityInputHandler for SourceEditor {
    fn text_for_range(
        &mut self,
        r: Range<usize>,
        actual: &mut Option<Range<usize>>,
        _: &mut Window,
        _: &mut Context<Self>,
    ) -> Option<String> {
        let range = self.range_from_utf16(&r);
        actual.replace(self.range_to_utf16(&range));
        Some(self.doc.text()[range].to_string())
    }

    fn selected_text_range(&mut self, _: bool, _: &mut Window, _: &mut Context<Self>) -> Option<UTF16Selection> {
        let s = self.doc.selection();
        Some(UTF16Selection { range: self.range_to_utf16(&s.range()), reversed: s.head < s.anchor })
    }

    fn marked_text_range(&self, _: &mut Window, _: &mut Context<Self>) -> Option<Range<usize>> {
        self.marked.as_ref().map(|r| self.range_to_utf16(r))
    }

    fn unmark_text(&mut self, _: &mut Window, _: &mut Context<Self>) {
        self.marked = None;
    }

    fn replace_text_in_range(&mut self, r: Option<Range<usize>>, text: &str, _: &mut Window, cx: &mut Context<Self>) {
        let range = r.as_ref().map(|r| self.range_from_utf16(r)).or(self.marked.clone()).unwrap_or(self.doc.selection().range());
        self.marked = None;
        let kind = if text.chars().all(|c| c.is_alphanumeric()) { EditKind::Typing } else { EditKind::Other };
        self.replace(range, text, kind, cx);
    }

    fn replace_and_mark_text_in_range(
        &mut self,
        r: Option<Range<usize>>,
        text: &str,
        new_sel: Option<Range<usize>>,
        _: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let range = r.as_ref().map(|r| self.range_from_utf16(r)).or(self.marked.clone()).unwrap_or(self.doc.selection().range());
        let start = range.start;
        let sel = new_sel.map(|s| {
            let s = self.range_from_utf16(&s);
            Selection::new(start + s.start, start + s.end)
        });
        if self.doc.apply(
            vec![Change::new(range.start, range.end, text)],
            sel.unwrap_or(Selection::cursor(start + text.len())),
            EditKind::Typing,
        ) {
            self.marked = (!text.is_empty()).then(|| start..start + text.len());
            self.changed(cx);
        }
    }

    fn bounds_for_range(
        &mut self,
        r: Range<usize>,
        _bounds: Bounds<Pixels>,
        _: &mut Window,
        _: &mut Context<Self>,
    ) -> Option<Bounds<Pixels>> {
        let range = self.range_from_utf16(&r);
        let a = self.position_for(range.start);
        let o = self.text_origin;
        Some(Bounds::new(point(o.x + a.x, o.y + a.y), size(px(2.0), self.line_height)))
    }

    fn character_index_for_point(&mut self, p: Point<Pixels>, _: &mut Window, _: &mut Context<Self>) -> Option<usize> {
        Some(self.to_utf16(self.offset_for_mouse(p)))
    }
}

/// Colours of the editor's tokens.
struct EdColors {
    text: Hsla,
    mark: Hsla,
    heading: Hsla,
    code: Hsla,
    code_bg: Hsla,
    link: Hsla,
    url: Hsla,
    quote: Hsla,
    html: Hsla,
    comment: Hsla,
    key: Hsla,
    value: Hsla,
    math: Hsla,
    highlight_bg: Hsla,
}

impl EdColors {
    fn of(pal: &Palette) -> EdColors {
        let c = |l: u32, d: u32| -> Hsla { gpui::rgb(if pal.dark { d } else { l }).into() };
        EdColors {
            text: pal.text.into(),
            mark: c(0x9aa3ad, 0x6e7681),
            heading: pal.accent.into(),
            code: c(0x0a3069, 0xa5d6ff),
            code_bg: c(0xe9ecef, 0x2b2f36),
            link: c(0x0b65c2, 0x6cb6ff),
            url: c(0x8a8f98, 0x7d8590),
            quote: c(0x6b7280, 0x9aa4ae),
            html: c(0xcf222e, 0xff7b72),
            comment: c(0x8250df, 0xc297ff),
            key: c(0x116329, 0x7ee787),
            value: c(0x0550ae, 0x79c0ff),
            math: c(0x953800, 0xffa657),
            highlight_bg: c(0xfff3a3, 0x5a4d12),
        }
    }
}

/// Runs for one line from its tokens.
fn runs_for(line: &str, toks: Option<&LineToks>, font: &Font, col: &EdColors) -> Vec<TextRun> {
    let base = TextRun { len: 0, font: font.clone(), color: col.text, background_color: None, underline: None, strikethrough: None };
    let mut runs = Vec::new();
    let mut pos = 0;
    let push = |runs: &mut Vec<TextRun>, len: usize, r: TextRun| {
        if len > 0 {
            runs.push(TextRun { len, ..r });
        }
    };
    for (range, tok) in toks.into_iter().flatten() {
        let range = range.start.min(line.len())..range.end.min(line.len());
        if range.start < pos || range.is_empty() {
            continue;
        }
        push(&mut runs, range.start - pos, base.clone());
        let mut r = base.clone();
        match tok {
            Tok::Mark | Tok::Fence => r.color = col.mark,
            Tok::Heading => {
                r.color = col.heading;
                r.font.weight = FontWeight::BOLD;
            }
            Tok::Bold => r.font.weight = FontWeight::BOLD,
            Tok::Italic => r.font.style = FontStyle::Italic,
            Tok::BoldItalic => {
                r.font.weight = FontWeight::BOLD;
                r.font.style = FontStyle::Italic;
            }
            Tok::Strike => r.strikethrough = Some(gpui::StrikethroughStyle { thickness: px(1.0), color: Some(col.text) }),
            Tok::Highlight => r.background_color = Some(col.highlight_bg),
            Tok::Code => {
                r.color = col.code;
                r.background_color = Some(col.code_bg);
            }
            Tok::CodeBlock => r.color = col.code,
            Tok::LinkText => {
                r.color = col.link;
                r.underline = Some(UnderlineStyle { thickness: px(1.0), color: Some(col.link), wavy: false });
            }
            Tok::Url => r.color = col.url,
            Tok::Quote => {
                r.color = col.quote;
                r.font.style = FontStyle::Italic;
            }
            Tok::Html => r.color = col.html,
            Tok::Comment => r.color = col.comment,
            Tok::MetaKey => r.color = col.key,
            Tok::MetaValue => r.color = col.value,
            Tok::Math => r.color = col.math,
        }
        push(&mut runs, range.len(), r);
        pos = range.end;
    }
    push(&mut runs, line.len() - pos, base);
    runs
}

struct EditorElement {
    editor: Entity<SourceEditor>,
}

struct Prepaint {
    lines: Vec<(Point<Pixels>, WrappedLine)>,
    numbers: Vec<(Point<Pixels>, gpui::ShapedLine)>,
    selections: Vec<gpui::PaintQuad>,
    cursor: Option<gpui::PaintQuad>,
    active: Option<gpui::PaintQuad>,
    gutter: Option<gpui::PaintQuad>,
}

impl IntoElement for EditorElement {
    type Element = Self;
    fn into_element(self) -> Self {
        self
    }
}

const PAD_X: f32 = 28.0;
const PAD_Y: f32 = 18.0;

/// Shape every line for `width` (the whole element's width).
fn shape_all(ed: &mut SourceEditor, width: Pixels, window: &mut Window, cx: &App) -> (Vec<LineLayout>, Pixels) {
    let pal = Theme::palette(cx);
    let col = EdColors::of(&pal);
    if ed.toks_version != ed.doc.version() || ed.toks.is_empty() {
        ed.toks = md::highlight(ed.doc.text());
        ed.toks_version = ed.doc.version();
    }
    let font = Font { family: mono_family(), ..gpui::font(mono_family()) };
    let gutter = gutter_width(ed, window);
    let wrap = ed.wrap.then(|| (width - gutter - px(PAD_X * 2.0)).max(px(80.0)));
    let lh = ed.line_height;
    let mut y = px(0.0);
    let mut out = Vec::new();
    let mut start = 0;
    let text = ed.doc.text().to_string();
    for (i, line) in text.split('\n').enumerate() {
        let runs = runs_for(line, ed.toks.get(i), &font, &col);
        let shaped = window
            .text_system()
            .shape_text(SharedString::from(line.to_string()), ed.font_size, &runs, wrap, None)
            .ok()
            .and_then(|mut v| if v.is_empty() { None } else { Some(v.remove(0)) })
            .unwrap_or_default();
        let rows = shaped.wrap_boundaries().len() + 1;
        out.push(LineLayout { start, y, rows, line: shaped });
        y += lh * rows as f32;
        start += line.len() + 1;
    }
    (out, y)
}

fn gutter_width(ed: &SourceEditor, _window: &Window) -> Pixels {
    if !ed.line_numbers {
        return px(0.0);
    }
    let digits = ed.doc.line_count().to_string().len().max(2) as f32;
    px(digits * f32::from(ed.font_size) * 0.62 + 24.0)
}

impl Element for EditorElement {
    type RequestLayoutState = ();
    type PrepaintState = Prepaint;

    fn id(&self) -> Option<ElementId> {
        None
    }

    fn source_location(&self) -> Option<&'static core::panic::Location<'static>> {
        None
    }

    fn request_layout(
        &mut self,
        _: Option<&GlobalElementId>,
        _: Option<&gpui::InspectorElementId>,
        window: &mut Window,
        _cx: &mut App,
    ) -> (LayoutId, ()) {
        let mut style = Style::default();
        style.size.width = relative(1.).into();
        let editor = self.editor.clone();
        let id = window.request_measured_layout(style, move |known, available, window, cx| {
            let width = known.width.or(match available.width {
                gpui::AvailableSpace::Definite(w) => Some(w),
                _ => None,
            });
            let width = width.unwrap_or(px(600.0));
            let height = editor.update(cx, |ed, cx| {
                let (_, h) = shape_all(ed, width, window, cx);
                h
            });
            Size { width, height: height + px(PAD_Y * 2.0) + px(200.0) }
        });
        (id, ())
    }

    fn prepaint(
        &mut self,
        _: Option<&GlobalElementId>,
        _: Option<&gpui::InspectorElementId>,
        bounds: Bounds<Pixels>,
        _: &mut (),
        window: &mut Window,
        cx: &mut App,
    ) -> Prepaint {
        let pal = Theme::palette(cx);
        let focused = self.editor.read(cx).focus.is_focused(window);
        let (layout, _) = self.editor.update(cx, |ed, cx| shape_all(ed, bounds.size.width, window, cx));
        let ed = self.editor.read(cx);
        let gutter = gutter_width(ed, window);
        let origin = point(bounds.left() + gutter + px(PAD_X), bounds.top() + px(PAD_Y));
        let lh = ed.line_height;
        let sel = ed.doc.selection();

        let mut lines = Vec::with_capacity(layout.len());
        let mut numbers = Vec::new();
        let num_color: Hsla = gpui::rgb(if pal.dark { 0x6e7681 } else { 0x9aa0a6 }).into();
        let font = gpui::font(mono_family());
        let cur_line = ed.doc.line_of(sel.head);
        // only what can be seen is drawn
        let view = ed.scroll.bounds();
        let visible = |y: Pixels, h: Pixels| {
            let top = origin.y + y;
            view.size.height == px(0.0) || (top + h >= view.top() - lh * 4.0 && top <= view.bottom() + lh * 4.0)
        };
        for (i, l) in layout.iter().enumerate() {
            if !visible(l.y, lh * l.rows as f32) {
                continue;
            }
            lines.push((point(origin.x, origin.y + l.y), l.line.clone()));
            if ed.line_numbers {
                let text: SharedString = (i + 1).to_string().into();
                let run = TextRun {
                    len: text.len(),
                    font: font.clone(),
                    color: if i == cur_line { pal.text_muted.into() } else { num_color },
                    background_color: None,
                    underline: None,
                    strikethrough: None,
                };
                let shaped = window.text_system().shape_line(text, ed.font_size * 0.86, &[run], None);
                let x = bounds.left() + gutter - px(12.0) - shaped.width;
                numbers.push((point(x, origin.y + l.y), shaped));
            }
        }

        // selection, cursor and the active line, from this layout
        let pos = |offset: usize| -> Point<Pixels> {
            for (i, l) in layout.iter().enumerate() {
                let end = l.start + l.line.len();
                if offset >= l.start && (offset <= end || i + 1 == layout.len()) {
                    let p = l.line.position_for_index((offset - l.start).min(l.line.len()), lh).unwrap_or_default();
                    return point(p.x, l.y + p.y);
                }
            }
            point(px(0.0), px(0.0))
        };
        let sel_color = Hsla::from(pal.accent).opacity(0.26);
        let mut selections = Vec::new();
        let right = bounds.right() - px(PAD_X) - origin.x;
        let mark_color: Hsla = gpui::rgba(0xf5c40066).into();
        let current_color: Hsla = gpui::rgba(0xff8c00aa).into();
        for (k, m) in ed.marks.iter().enumerate() {
            let a = pos(m.start);
            if !visible(a.y, lh) {
                continue;
            }
            let b = pos(m.end);
            let color = if ed.mark_current == Some(k) { current_color } else { mark_color };
            let mut y = a.y;
            while y <= b.y {
                let x0 = if y == a.y { a.x } else { px(0.0) };
                let x1 = if y == b.y { b.x } else { right };
                if x1 > x0 {
                    selections.push(fill(
                        Bounds::from_corners(point(origin.x + x0, origin.y + y), point(origin.x + x1, origin.y + y + lh)),
                        color,
                    ));
                }
                y += lh;
            }
        }
        if !sel.is_empty() {
            let a = pos(sel.start());
            let b = pos(sel.end());
            let mut y = a.y;
            while y <= b.y {
                let x0 = if y == a.y { a.x } else { px(0.0) };
                let x1 = if y == b.y { b.x } else { right };
                if x1 > x0 && visible(y, lh) {
                    selections.push(fill(
                        Bounds::from_corners(point(origin.x + x0, origin.y + y), point(origin.x + x1, origin.y + y + lh)),
                        sel_color,
                    ));
                }
                y += lh;
            }
        }
        let c = pos(sel.head);
        let cursor = (focused && sel.is_empty())
            .then(|| fill(Bounds::new(point(origin.x + c.x, origin.y + c.y + px(2.0)), size(px(2.0), lh - px(4.0))), pal.accent));
        let active = layout.get(cur_line).map(|l| {
            fill(
                Bounds::new(point(bounds.left() + gutter, origin.y + l.y), size(bounds.size.width - gutter, lh * l.rows as f32)),
                Hsla::from(pal.accent).opacity(if pal.dark { 0.07 } else { 0.05 }),
            )
        });
        let gutter_q =
            ed.line_numbers.then(|| fill(Bounds::new(bounds.origin, size(gutter, bounds.size.height)), mix(pal.surface, pal.canvas, 0.5)));

        // keep the cursor in view after keys and edits
        let reveal = ed.reveal;
        let cursor_top = origin.y + c.y;
        self.editor.update(cx, |ed, _| {
            ed.layout = layout;
            ed.text_origin = origin;
            ed.bounds = Some(bounds);
            ed.reveal = false;
            if reveal {
                let view = ed.scroll.bounds();
                if view.size.height > px(0.0) {
                    let margin = lh * 2.0;
                    let mut off = ed.scroll.offset();
                    if cursor_top < view.top() + margin {
                        off.y += view.top() + margin - cursor_top;
                    } else if cursor_top + lh > view.bottom() - margin {
                        off.y -= cursor_top + lh - (view.bottom() - margin);
                    }
                    off.y = off.y.min(px(0.0));
                    if off != ed.scroll.offset() {
                        ed.scroll.set_offset(off);
                    }
                }
            }
        });
        Prepaint { lines, numbers, selections, cursor, active, gutter: gutter_q }
    }

    fn paint(
        &mut self,
        _: Option<&GlobalElementId>,
        _: Option<&gpui::InspectorElementId>,
        bounds: Bounds<Pixels>,
        _: &mut (),
        p: &mut Prepaint,
        window: &mut Window,
        cx: &mut App,
    ) {
        let focus = self.editor.read(cx).focus.clone();
        window.handle_input(&focus, ElementInputHandler::new(bounds, self.editor.clone()), cx);
        let lh = self.editor.read(cx).line_height;
        window.with_content_mask(Some(ContentMask { bounds }), |window| {
            if let Some(g) = p.gutter.take() {
                window.paint_quad(g);
            }
            if let Some(a) = p.active.take() {
                window.paint_quad(a);
            }
            for q in p.selections.drain(..) {
                window.paint_quad(q);
            }
            for (o, l) in &p.lines {
                let _ = l.paint(*o, lh, TextAlign::Left, None, window, cx);
            }
            for (o, n) in &p.numbers {
                let _ = n.paint(*o + point(px(0.0), px(2.0)), lh, window, cx);
            }
            if let Some(c) = p.cursor.take() {
                window.paint_quad(c);
            }
        });
    }
}

impl Render for SourceEditor {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let pal = Theme::palette(cx);
        self.line_height = (self.font_size * 1.6).round();
        div()
            .id("kd-source-editor")
            .key_context(CONTEXT)
            .track_focus(&self.focus)
            .size_full()
            .overflow_y_scroll()
            .track_scroll(&self.scroll)
            .bg(pal.surface)
            .cursor(CursorStyle::IBeam)
            .text_color(pal.text)
            .font_family(mono_family())
            .on_action(cx.listener(Self::left))
            .on_action(cx.listener(Self::right))
            .on_action(cx.listener(Self::up))
            .on_action(cx.listener(Self::down))
            .on_action(cx.listener(Self::select_left))
            .on_action(cx.listener(Self::select_right))
            .on_action(cx.listener(Self::select_up))
            .on_action(cx.listener(Self::select_down))
            .on_action(cx.listener(Self::word_left_a))
            .on_action(cx.listener(Self::word_right_a))
            .on_action(cx.listener(Self::select_word_left))
            .on_action(cx.listener(Self::select_word_right))
            .on_action(cx.listener(Self::line_start))
            .on_action(cx.listener(Self::line_end))
            .on_action(cx.listener(Self::select_line_start))
            .on_action(cx.listener(Self::select_line_end))
            .on_action(cx.listener(Self::doc_start))
            .on_action(cx.listener(Self::doc_end))
            .on_action(cx.listener(Self::select_doc_start))
            .on_action(cx.listener(Self::select_doc_end))
            .on_action(cx.listener(Self::page_up))
            .on_action(cx.listener(Self::page_down))
            .on_action(cx.listener(Self::backspace))
            .on_action(cx.listener(Self::delete))
            .on_action(cx.listener(Self::backspace_word))
            .on_action(cx.listener(Self::delete_word))
            .on_action(cx.listener(Self::enter))
            .on_action(cx.listener(Self::tab))
            .on_action(cx.listener(Self::shift_tab))
            .on_action(cx.listener(Self::select_all))
            .on_action(cx.listener(Self::copy))
            .on_action(cx.listener(Self::cut))
            .on_action(cx.listener(Self::paste))
            .on_action(cx.listener(Self::undo_a))
            .on_action(cx.listener(Self::redo_a))
            .on_mouse_down(MouseButton::Left, cx.listener(Self::on_mouse_down))
            .on_mouse_up(MouseButton::Left, cx.listener(Self::on_mouse_up))
            .on_mouse_up_out(MouseButton::Left, cx.listener(Self::on_mouse_up))
            .on_mouse_move(cx.listener(Self::on_mouse_move))
            .child(EditorElement { editor: cx.entity() })
    }
}

impl Focusable for SourceEditor {
    fn focus_handle(&self, _: &App) -> FocusHandle {
        self.focus.clone()
    }
}

#[cfg(test)]
mod tests {
    use super::continuation;

    #[test]
    fn enter_continues_lists_and_quotes() {
        assert_eq!(continuation("- item"), ("- ".into(), false));
        assert_eq!(continuation("  * item"), ("  * ".into(), false));
        assert_eq!(continuation("3. third"), ("4. ".into(), false));
        assert_eq!(continuation("- [x] done"), ("- [ ] ".into(), false));
        assert_eq!(continuation("> quoted"), ("> ".into(), false));
        assert_eq!(continuation("- "), ("- ".into(), true));
        assert_eq!(continuation("plain"), ("".into(), false));
        assert_eq!(continuation("    code"), ("    ".into(), false));
    }
}
