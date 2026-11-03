//! A text input for GPUI: one line (search, names, API keys) or several wrapped lines (the KaDee
//! AI composer). Built on GPUI's own input example (Apache-2.0) and extended with wrapping,
//! up/down movement, horizontal scrolling, masking and events.
//!
//! Keys: arrows, Shift+arrows, Home/End, Cmd/Ctrl+A/C/X/V, Backspace, Delete. **Enter** emits
//! [`InputEvent::Submit`]; **Shift+Enter** adds a line in a multi-line input; **Esc** emits
//! [`InputEvent::Escape`]. IME composition works through GPUI's input handler.

use std::ops::Range;

use gpui::{
    App, Bounds, ClipboardItem, ContentMask, Context, CursorStyle, Element, ElementId, ElementInputHandler, Entity, EntityInputHandler,
    EventEmitter, FocusHandle, Focusable, GlobalElementId, KeyBinding, LayoutId, MouseButton, MouseDownEvent, MouseMoveEvent, MouseUpEvent,
    PaintQuad, Pixels, Point, SharedString, Size, Style, TextAlign, TextRun, UTF16Selection, UnderlineStyle, Window, WrappedLine, actions,
    div, fill, point, prelude::*, px, relative, size,
};
use unicode_segmentation::UnicodeSegmentation;

use crate::theme::Theme;

actions!(
    kd_input,
    [
        Backspace,
        Delete,
        Left,
        Right,
        Up,
        Down,
        SelectLeft,
        SelectRight,
        SelectUp,
        SelectDown,
        SelectAll,
        Home,
        End,
        Paste,
        Cut,
        Copy,
        Enter,
        Newline,
        Escape
    ]
);

const CONTEXT: &str = "KdInput";

/// Bind the input's keys; call once at start-up.
pub fn bind_keys(cx: &mut App) {
    let c = Some(CONTEXT);
    cx.bind_keys([
        KeyBinding::new("backspace", Backspace, c),
        KeyBinding::new("shift-backspace", Backspace, c),
        KeyBinding::new("delete", Delete, c),
        KeyBinding::new("left", Left, c),
        KeyBinding::new("right", Right, c),
        KeyBinding::new("up", Up, c),
        KeyBinding::new("down", Down, c),
        KeyBinding::new("shift-left", SelectLeft, c),
        KeyBinding::new("shift-right", SelectRight, c),
        KeyBinding::new("shift-up", SelectUp, c),
        KeyBinding::new("shift-down", SelectDown, c),
        KeyBinding::new("secondary-a", SelectAll, c),
        KeyBinding::new("secondary-v", Paste, c),
        KeyBinding::new("secondary-c", Copy, c),
        KeyBinding::new("secondary-x", Cut, c),
        KeyBinding::new("home", Home, c),
        KeyBinding::new("end", End, c),
        KeyBinding::new("enter", Enter, c),
        KeyBinding::new("shift-enter", Newline, c),
        KeyBinding::new("escape", Escape, c),
    ]);
    #[cfg(target_os = "macos")]
    cx.bind_keys([KeyBinding::new("cmd-left", Home, c), KeyBinding::new("cmd-right", End, c)]);
}

/// What an input tells its owner.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum InputEvent {
    /// The text changed.
    Changed,
    /// Enter was pressed.
    Submit,
    /// Esc was pressed.
    Escape,
}

pub struct TextInput {
    focus_handle: FocusHandle,
    content: String,
    placeholder: SharedString,
    multiline: bool,
    masked: bool,
    selected_range: Range<usize>,
    selection_reversed: bool,
    marked_range: Option<Range<usize>>,
    is_selecting: bool,
    // From the last paint:
    lines: Vec<WrappedLine>,
    line_starts: Vec<usize>,
    line_height: Pixels,
    last_bounds: Option<Bounds<Pixels>>,
    scroll_x: Pixels,
}

impl EventEmitter<InputEvent> for TextInput {}

impl TextInput {
    /// A one-line input.
    pub fn new(cx: &mut Context<Self>, placeholder: impl Into<SharedString>) -> Self {
        TextInput {
            focus_handle: cx.focus_handle(),
            content: String::new(),
            placeholder: placeholder.into(),
            multiline: false,
            masked: false,
            selected_range: 0..0,
            selection_reversed: false,
            marked_range: None,
            is_selecting: false,
            lines: Vec::new(),
            line_starts: Vec::new(),
            line_height: px(20.0),
            last_bounds: None,
            scroll_x: px(0.0),
        }
    }

    /// Wrap the text and allow Shift+Enter line breaks.
    pub fn multiline(mut self) -> Self {
        self.multiline = true;
        self
    }

    /// Show `*` instead of the characters (API keys).
    pub fn masked(mut self, masked: bool) -> Self {
        self.masked = masked;
        self
    }

    pub fn set_masked(&mut self, masked: bool, cx: &mut Context<Self>) {
        self.masked = masked;
        cx.notify();
    }

    pub fn set_placeholder(&mut self, placeholder: impl Into<SharedString>, cx: &mut Context<Self>) {
        self.placeholder = placeholder.into();
        cx.notify();
    }

    pub fn text(&self) -> &str {
        &self.content
    }

    /// Replace the text and put the cursor at the end.
    pub fn set_text(&mut self, text: impl Into<String>, cx: &mut Context<Self>) {
        let mut text = text.into();
        if !self.multiline {
            text = text.replace('\n', " ");
        }
        self.content = text;
        self.selected_range = self.content.len()..self.content.len();
        self.selection_reversed = false;
        self.marked_range = None;
        self.scroll_x = px(0.0);
        cx.emit(InputEvent::Changed);
        cx.notify();
    }

    pub fn clear(&mut self, cx: &mut Context<Self>) {
        self.set_text("", cx);
    }

    pub fn is_focused(&self, window: &Window) -> bool {
        self.focus_handle.is_focused(window)
    }

    pub fn focus(&self, window: &mut Window) {
        window.focus(&self.focus_handle);
    }

    // ---- editing --------------------------------------------------------------------------

    fn left(&mut self, _: &Left, _: &mut Window, cx: &mut Context<Self>) {
        if self.selected_range.is_empty() {
            self.move_to(self.previous_boundary(self.cursor_offset()), cx);
        } else {
            self.move_to(self.selected_range.start, cx)
        }
    }

    fn right(&mut self, _: &Right, _: &mut Window, cx: &mut Context<Self>) {
        if self.selected_range.is_empty() {
            self.move_to(self.next_boundary(self.selected_range.end), cx);
        } else {
            self.move_to(self.selected_range.end, cx)
        }
    }

    fn up(&mut self, _: &Up, _: &mut Window, cx: &mut Context<Self>) {
        let to = self.vertical_target(self.cursor_offset(), -1);
        self.move_to(to, cx);
    }

    fn down(&mut self, _: &Down, _: &mut Window, cx: &mut Context<Self>) {
        let to = self.vertical_target(self.cursor_offset(), 1);
        self.move_to(to, cx);
    }

    fn select_left(&mut self, _: &SelectLeft, _: &mut Window, cx: &mut Context<Self>) {
        self.select_to(self.previous_boundary(self.cursor_offset()), cx);
    }

    fn select_right(&mut self, _: &SelectRight, _: &mut Window, cx: &mut Context<Self>) {
        self.select_to(self.next_boundary(self.cursor_offset()), cx);
    }

    fn select_up(&mut self, _: &SelectUp, _: &mut Window, cx: &mut Context<Self>) {
        self.select_to(self.vertical_target(self.cursor_offset(), -1), cx);
    }

    fn select_down(&mut self, _: &SelectDown, _: &mut Window, cx: &mut Context<Self>) {
        self.select_to(self.vertical_target(self.cursor_offset(), 1), cx);
    }

    fn select_all(&mut self, _: &SelectAll, _: &mut Window, cx: &mut Context<Self>) {
        self.move_to(0, cx);
        self.select_to(self.content.len(), cx)
    }

    fn home(&mut self, _: &Home, _: &mut Window, cx: &mut Context<Self>) {
        let line_start = self.content[..self.cursor_offset()].rfind('\n').map(|i| i + 1).unwrap_or(0);
        self.move_to(line_start, cx);
    }

    fn end(&mut self, _: &End, _: &mut Window, cx: &mut Context<Self>) {
        let c = self.cursor_offset();
        let line_end = self.content[c..].find('\n').map(|i| c + i).unwrap_or(self.content.len());
        self.move_to(line_end, cx);
    }

    fn backspace(&mut self, _: &Backspace, window: &mut Window, cx: &mut Context<Self>) {
        if self.selected_range.is_empty() {
            self.select_to(self.previous_boundary(self.cursor_offset()), cx)
        }
        self.replace_text_in_range(None, "", window, cx)
    }

    fn delete(&mut self, _: &Delete, window: &mut Window, cx: &mut Context<Self>) {
        if self.selected_range.is_empty() {
            self.select_to(self.next_boundary(self.cursor_offset()), cx)
        }
        self.replace_text_in_range(None, "", window, cx)
    }

    fn enter(&mut self, _: &Enter, _: &mut Window, cx: &mut Context<Self>) {
        cx.emit(InputEvent::Submit);
    }

    fn newline(&mut self, _: &Newline, window: &mut Window, cx: &mut Context<Self>) {
        if self.multiline {
            self.replace_text_in_range(None, "\n", window, cx);
        } else {
            cx.emit(InputEvent::Submit);
        }
    }

    fn escape(&mut self, _: &Escape, _: &mut Window, cx: &mut Context<Self>) {
        cx.emit(InputEvent::Escape);
    }

    fn paste(&mut self, _: &Paste, window: &mut Window, cx: &mut Context<Self>) {
        if let Some(text) = cx.read_from_clipboard().and_then(|item| item.text()) {
            let text = if self.multiline { text } else { text.replace('\n', " ") };
            self.replace_text_in_range(None, &text, window, cx);
        }
    }

    fn copy(&mut self, _: &Copy, _: &mut Window, cx: &mut Context<Self>) {
        if !self.selected_range.is_empty() && !self.masked {
            cx.write_to_clipboard(ClipboardItem::new_string(self.content[self.selected_range.clone()].to_string()));
        }
    }

    fn cut(&mut self, _: &Cut, window: &mut Window, cx: &mut Context<Self>) {
        if !self.selected_range.is_empty() && !self.masked {
            cx.write_to_clipboard(ClipboardItem::new_string(self.content[self.selected_range.clone()].to_string()));
            self.replace_text_in_range(None, "", window, cx)
        }
    }

    fn on_mouse_down(&mut self, event: &MouseDownEvent, window: &mut Window, cx: &mut Context<Self>) {
        window.focus(&self.focus_handle);
        self.is_selecting = true;
        let at = self.index_for_mouse_position(event.position);
        if event.click_count >= 2 {
            // double-click: the word, triple-click: everything
            if event.click_count >= 3 {
                self.selected_range = 0..self.content.len();
            } else {
                let (s, e) = self.word_at(at);
                self.selected_range = s..e;
            }
            self.selection_reversed = false;
            cx.notify();
        } else if event.modifiers.shift {
            self.select_to(at, cx);
        } else {
            self.move_to(at, cx)
        }
    }

    fn on_mouse_up(&mut self, _: &MouseUpEvent, _window: &mut Window, _: &mut Context<Self>) {
        self.is_selecting = false;
    }

    fn on_mouse_move(&mut self, event: &MouseMoveEvent, _: &mut Window, cx: &mut Context<Self>) {
        if self.is_selecting {
            self.select_to(self.index_for_mouse_position(event.position), cx);
        }
    }

    fn move_to(&mut self, offset: usize, cx: &mut Context<Self>) {
        let offset = self.snap(offset);
        self.selected_range = offset..offset;
        self.selection_reversed = false;
        cx.notify()
    }

    fn cursor_offset(&self) -> usize {
        if self.selection_reversed { self.selected_range.start } else { self.selected_range.end }
    }

    fn select_to(&mut self, offset: usize, cx: &mut Context<Self>) {
        let offset = self.snap(offset);
        if self.selection_reversed {
            self.selected_range.start = offset
        } else {
            self.selected_range.end = offset
        };
        if self.selected_range.end < self.selected_range.start {
            self.selection_reversed = !self.selection_reversed;
            self.selected_range = self.selected_range.end..self.selected_range.start;
        }
        cx.notify()
    }

    /// The nearest char boundary at or before `offset`.
    fn snap(&self, offset: usize) -> usize {
        let mut o = offset.min(self.content.len());
        while !self.content.is_char_boundary(o) {
            o -= 1;
        }
        o
    }

    fn word_at(&self, offset: usize) -> (usize, usize) {
        for (start, word) in self.content.split_word_bound_indices() {
            if offset >= start && offset <= start + word.len() {
                return (start, start + word.len());
            }
        }
        (offset, offset)
    }

    fn previous_boundary(&self, offset: usize) -> usize {
        self.content.grapheme_indices(true).rev().find_map(|(idx, _)| (idx < offset).then_some(idx)).unwrap_or(0)
    }

    fn next_boundary(&self, offset: usize) -> usize {
        self.content.grapheme_indices(true).find_map(|(idx, _)| (idx > offset).then_some(idx)).unwrap_or(self.content.len())
    }

    // ---- geometry (from the last paint) ----------------------------------------------------

    /// Where `offset` is drawn, relative to the text's top-left corner.
    fn position_for(&self, offset: usize) -> Point<Pixels> {
        let mut y = px(0.0);
        for (i, line) in self.lines.iter().enumerate() {
            let start = self.line_starts.get(i).copied().unwrap_or(0);
            let rows = line.wrap_boundaries().len() + 1;
            if offset >= start && offset <= start + line.len() {
                let p = line.position_for_index(offset - start, self.line_height).unwrap_or_default();
                return point(p.x, y + p.y);
            }
            y += self.line_height * rows as f32;
        }
        point(px(0.0), y)
    }

    /// The offset nearest to `p` (relative to the text's top-left corner).
    fn offset_for(&self, p: Point<Pixels>) -> usize {
        if p.y < px(0.0) || self.lines.is_empty() {
            return 0;
        }
        let mut y = px(0.0);
        for (i, line) in self.lines.iter().enumerate() {
            let start = self.line_starts.get(i).copied().unwrap_or(0);
            let h = self.line_height * (line.wrap_boundaries().len() + 1) as f32;
            if p.y < y + h {
                let local = point(p.x.max(px(0.0)), p.y - y);
                let ix = match line.closest_index_for_position(local, self.line_height) {
                    Ok(ix) | Err(ix) => ix,
                };
                return (start + ix).min(self.content.len());
            }
            y += h;
        }
        self.content.len()
    }

    fn vertical_target(&self, offset: usize, dir: i32) -> usize {
        if self.lines.is_empty() {
            return offset;
        }
        let p = self.position_for(offset);
        let y = p.y + self.line_height * (dir as f32) + self.line_height / 2.0;
        if y < px(0.0) {
            return 0;
        }
        self.offset_for(point(p.x, y))
    }

    fn index_for_mouse_position(&self, position: Point<Pixels>) -> usize {
        let Some(bounds) = self.last_bounds else { return 0 };
        let local = point(position.x - bounds.left() + self.scroll_x, position.y - bounds.top());
        self.offset_for(local)
    }

    fn display_text(&self) -> String {
        if self.masked { "*".repeat(self.content.len()) } else { self.content.clone() }
    }

    // ---- UTF-16 for the platform input handler ---------------------------------------------

    fn offset_from_utf16(&self, offset: usize) -> usize {
        let mut utf8_offset = 0;
        let mut utf16_count = 0;
        for ch in self.content.chars() {
            if utf16_count >= offset {
                break;
            }
            utf16_count += ch.len_utf16();
            utf8_offset += ch.len_utf8();
        }
        utf8_offset
    }

    fn offset_to_utf16(&self, offset: usize) -> usize {
        let mut utf16_offset = 0;
        let mut utf8_count = 0;
        for ch in self.content.chars() {
            if utf8_count >= offset {
                break;
            }
            utf8_count += ch.len_utf8();
            utf16_offset += ch.len_utf16();
        }
        utf16_offset
    }

    fn range_to_utf16(&self, range: &Range<usize>) -> Range<usize> {
        self.offset_to_utf16(range.start)..self.offset_to_utf16(range.end)
    }

    fn range_from_utf16(&self, range_utf16: &Range<usize>) -> Range<usize> {
        self.offset_from_utf16(range_utf16.start)..self.offset_from_utf16(range_utf16.end)
    }
}

impl EntityInputHandler for TextInput {
    fn text_for_range(
        &mut self,
        range_utf16: Range<usize>,
        actual_range: &mut Option<Range<usize>>,
        _window: &mut Window,
        _cx: &mut Context<Self>,
    ) -> Option<String> {
        let range = self.range_from_utf16(&range_utf16);
        actual_range.replace(self.range_to_utf16(&range));
        Some(self.content[range].to_string())
    }

    fn selected_text_range(
        &mut self,
        _ignore_disabled_input: bool,
        _window: &mut Window,
        _cx: &mut Context<Self>,
    ) -> Option<UTF16Selection> {
        Some(UTF16Selection { range: self.range_to_utf16(&self.selected_range), reversed: self.selection_reversed })
    }

    fn marked_text_range(&self, _window: &mut Window, _cx: &mut Context<Self>) -> Option<Range<usize>> {
        self.marked_range.as_ref().map(|range| self.range_to_utf16(range))
    }

    fn unmark_text(&mut self, _window: &mut Window, _cx: &mut Context<Self>) {
        self.marked_range = None;
    }

    fn replace_text_in_range(&mut self, range_utf16: Option<Range<usize>>, new_text: &str, _: &mut Window, cx: &mut Context<Self>) {
        let range = range_utf16
            .as_ref()
            .map(|range_utf16| self.range_from_utf16(range_utf16))
            .or(self.marked_range.clone())
            .unwrap_or(self.selected_range.clone());
        let new_text = if self.multiline { new_text.to_string() } else { new_text.replace('\n', " ") };
        self.content = self.content[0..range.start].to_owned() + &new_text + &self.content[range.end..];
        self.selected_range = range.start + new_text.len()..range.start + new_text.len();
        self.selection_reversed = false;
        self.marked_range.take();
        cx.emit(InputEvent::Changed);
        cx.notify();
    }

    fn replace_and_mark_text_in_range(
        &mut self,
        range_utf16: Option<Range<usize>>,
        new_text: &str,
        new_selected_range_utf16: Option<Range<usize>>,
        _window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let range = range_utf16
            .as_ref()
            .map(|range_utf16| self.range_from_utf16(range_utf16))
            .or(self.marked_range.clone())
            .unwrap_or(self.selected_range.clone());
        self.content = self.content[0..range.start].to_owned() + new_text + &self.content[range.end..];
        self.marked_range = if new_text.is_empty() { None } else { Some(range.start..range.start + new_text.len()) };
        self.selected_range = new_selected_range_utf16
            .as_ref()
            .map(|range_utf16| self.range_from_utf16(range_utf16))
            .map(|new_range| new_range.start + range.start..new_range.end + range.start)
            .unwrap_or_else(|| range.start + new_text.len()..range.start + new_text.len());
        cx.emit(InputEvent::Changed);
        cx.notify();
    }

    fn bounds_for_range(
        &mut self,
        range_utf16: Range<usize>,
        bounds: Bounds<Pixels>,
        _window: &mut Window,
        _cx: &mut Context<Self>,
    ) -> Option<Bounds<Pixels>> {
        let range = self.range_from_utf16(&range_utf16);
        let a = self.position_for(range.start);
        let b = self.position_for(range.end);
        Some(Bounds::from_corners(
            point(bounds.left() + a.x - self.scroll_x, bounds.top() + a.y),
            point(bounds.left() + b.x - self.scroll_x, bounds.top() + b.y + self.line_height),
        ))
    }

    fn character_index_for_point(&mut self, p: Point<Pixels>, _window: &mut Window, _cx: &mut Context<Self>) -> Option<usize> {
        let bounds = self.last_bounds?;
        let local = point(p.x - bounds.left() + self.scroll_x, p.y - bounds.top());
        Some(self.offset_to_utf16(self.offset_for(local)))
    }
}

/// Shape `text` into lines (one per hard line, wrapped at `wrap`).
fn shape(
    text: &str,
    window: &Window,
    wrap: Option<Pixels>,
    color: gpui::Hsla,
    marked: Option<&Range<usize>>,
    accent: gpui::Hsla,
) -> Vec<WrappedLine> {
    let style = window.text_style();
    let font_size = style.font_size.to_pixels(window.rem_size());
    let run = TextRun { len: text.len(), font: style.font(), color, background_color: None, underline: None, strikethrough: None };
    let runs: Vec<TextRun> = match marked {
        Some(m) if m.end <= text.len() => vec![
            TextRun { len: m.start, ..run.clone() },
            TextRun {
                len: m.end - m.start,
                underline: Some(UnderlineStyle { color: Some(accent), thickness: px(1.0), wavy: false }),
                ..run.clone()
            },
            TextRun { len: text.len() - m.end, ..run },
        ]
        .into_iter()
        .filter(|r| r.len > 0)
        .collect(),
        _ => vec![run],
    };
    window
        .text_system()
        .shape_text(SharedString::from(text.to_string()), font_size, &runs, wrap, None)
        .map(|lines| lines.into_iter().collect())
        .unwrap_or_default()
}

fn line_starts(text: &str) -> Vec<usize> {
    std::iter::once(0).chain(text.match_indices('\n').map(|(i, _)| i + 1)).collect()
}

struct InputElement {
    input: Entity<TextInput>,
}

struct PrepaintState {
    lines: Vec<WrappedLine>,
    cursor: Option<PaintQuad>,
    selections: Vec<PaintQuad>,
}

impl IntoElement for InputElement {
    type Element = Self;
    fn into_element(self) -> Self::Element {
        self
    }
}

impl Element for InputElement {
    type RequestLayoutState = ();
    type PrepaintState = PrepaintState;

    fn id(&self) -> Option<ElementId> {
        None
    }

    fn source_location(&self) -> Option<&'static core::panic::Location<'static>> {
        None
    }

    fn request_layout(
        &mut self,
        _id: Option<&GlobalElementId>,
        _inspector_id: Option<&gpui::InspectorElementId>,
        window: &mut Window,
        cx: &mut App,
    ) -> (LayoutId, Self::RequestLayoutState) {
        let mut style = Style::default();
        style.size.width = relative(1.).into();
        let line_height = window.line_height();
        if !self.input.read(cx).multiline {
            style.size.height = line_height.into();
            return (window.request_layout(style, [], cx), ());
        }
        let input = self.input.clone();
        let layout = window.request_measured_layout(style, move |known, available, window, cx| {
            let width = known.width.or(match available.width {
                gpui::AvailableSpace::Definite(w) => Some(w),
                _ => None,
            });
            let text = input.read(cx).display_text();
            let shown = if text.is_empty() { input.read(cx).placeholder.to_string() } else { text };
            let lines = shape(&shown, window, width, gpui::black(), None, gpui::black());
            let rows: usize = lines.iter().map(|l| l.wrap_boundaries().len() + 1).sum::<usize>().max(1);
            Size { width: width.unwrap_or(px(100.0)), height: line_height * rows as f32 }
        });
        (layout, ())
    }

    fn prepaint(
        &mut self,
        _id: Option<&GlobalElementId>,
        _inspector_id: Option<&gpui::InspectorElementId>,
        bounds: Bounds<Pixels>,
        _request_layout: &mut Self::RequestLayoutState,
        window: &mut Window,
        cx: &mut App,
    ) -> Self::PrepaintState {
        let pal = Theme::palette(cx);
        let input = self.input.read(cx);
        let multiline = input.multiline;
        let content = input.display_text();
        let empty = content.is_empty();
        let (shown, color) = if empty {
            (input.placeholder.to_string(), gpui::Hsla::from(pal.text_faint))
        } else {
            (content.clone(), window.text_style().color)
        };
        let marked = input.marked_range.clone();
        let selected = input.selected_range.clone();
        let cursor_at = input.cursor_offset();
        let wrap = multiline.then_some(bounds.size.width);
        let lines = shape(&shown, window, wrap, color, marked.as_ref().filter(|_| !empty), pal.accent.into());
        let line_height = window.line_height();

        // geometry on the entity, for the cursor, selection and mouse
        let starts = if empty { vec![0] } else { line_starts(&content) };
        self.input.update(cx, |input, _| {
            input.lines = if empty { Vec::new() } else { lines.clone() };
            input.line_starts = starts;
            input.line_height = line_height;
        });
        let input = self.input.read(cx);
        let cursor_pos = if empty { point(px(0.0), px(0.0)) } else { input.position_for(cursor_at) };

        // one line: keep the cursor in view
        let mut scroll_x = input.scroll_x;
        if !multiline {
            let w = bounds.size.width - px(2.0);
            if cursor_pos.x - scroll_x > w {
                scroll_x = cursor_pos.x - w;
            } else if cursor_pos.x < scroll_x {
                scroll_x = cursor_pos.x;
            }
            let total = lines.first().map(|l| l.width()).unwrap_or_default();
            if total < bounds.size.width {
                scroll_x = px(0.0);
            }
        }
        let origin = point(bounds.left() - scroll_x, bounds.top());

        let mut selections = Vec::new();
        let sel_color = gpui::Hsla::from(pal.accent).opacity(0.28);
        if !selected.is_empty() && !empty {
            for (i, line) in input.lines.iter().enumerate() {
                let start = input.line_starts[i];
                let end = start + line.len();
                let (s, e) = (selected.start.max(start), selected.end.min(end));
                // an empty line inside the selection still shows a sliver
                if s > e || (s == e && !(selected.start <= start && selected.end > end)) {
                    continue;
                }
                let a = input.position_for(s);
                let b = input.position_for(e);
                let right = if multiline { bounds.size.width } else { line.width() };
                let extra = if selected.end > end { px(6.0) } else { px(0.0) };
                let mut row = a.y;
                while row <= b.y {
                    let x0 = if row == a.y { a.x } else { px(0.0) };
                    let x1 = if row == b.y { b.x + extra } else { right };
                    if x1 > x0 {
                        selections.push(fill(
                            Bounds::from_corners(point(origin.x + x0, origin.y + row), point(origin.x + x1, origin.y + row + line_height)),
                            sel_color,
                        ));
                    }
                    row += line_height;
                }
            }
        }
        let cursor = selected.is_empty().then(|| {
            fill(
                Bounds::new(point(origin.x + cursor_pos.x, origin.y + cursor_pos.y + px(1.0)), size(px(1.5), line_height - px(2.0))),
                pal.accent,
            )
        });
        self.input.update(cx, |input, _| input.scroll_x = scroll_x);
        PrepaintState { lines, cursor, selections }
    }

    fn paint(
        &mut self,
        _id: Option<&GlobalElementId>,
        _inspector_id: Option<&gpui::InspectorElementId>,
        bounds: Bounds<Pixels>,
        _request_layout: &mut Self::RequestLayoutState,
        prepaint: &mut Self::PrepaintState,
        window: &mut Window,
        cx: &mut App,
    ) {
        let focus_handle = self.input.read(cx).focus_handle.clone();
        let scroll_x = self.input.read(cx).scroll_x;
        window.handle_input(&focus_handle, ElementInputHandler::new(bounds, self.input.clone()), cx);
        let line_height = window.line_height();
        window.with_content_mask(Some(ContentMask { bounds }), |window| {
            for q in prepaint.selections.drain(..) {
                window.paint_quad(q);
            }
            let mut y = bounds.top();
            for line in &prepaint.lines {
                let _ = line.paint(point(bounds.left() - scroll_x, y), line_height, TextAlign::Left, None, window, cx);
                y += line_height * (line.wrap_boundaries().len() + 1) as f32;
            }
            if focus_handle.is_focused(window)
                && let Some(cursor) = prepaint.cursor.take()
            {
                window.paint_quad(cursor);
            }
        });
        self.input.update(cx, |input, _| input.last_bounds = Some(bounds));
    }
}

impl Render for TextInput {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        div()
            .flex()
            .w_full()
            .key_context(CONTEXT)
            .track_focus(&self.focus_handle(cx))
            .cursor(CursorStyle::IBeam)
            .on_action(cx.listener(Self::backspace))
            .on_action(cx.listener(Self::delete))
            .on_action(cx.listener(Self::left))
            .on_action(cx.listener(Self::right))
            .on_action(cx.listener(Self::up))
            .on_action(cx.listener(Self::down))
            .on_action(cx.listener(Self::select_left))
            .on_action(cx.listener(Self::select_right))
            .on_action(cx.listener(Self::select_up))
            .on_action(cx.listener(Self::select_down))
            .on_action(cx.listener(Self::select_all))
            .on_action(cx.listener(Self::home))
            .on_action(cx.listener(Self::end))
            .on_action(cx.listener(Self::paste))
            .on_action(cx.listener(Self::cut))
            .on_action(cx.listener(Self::copy))
            .on_action(cx.listener(Self::enter))
            .on_action(cx.listener(Self::newline))
            .on_action(cx.listener(Self::escape))
            .on_mouse_down(MouseButton::Left, cx.listener(Self::on_mouse_down))
            .on_mouse_up(MouseButton::Left, cx.listener(Self::on_mouse_up))
            .on_mouse_up_out(MouseButton::Left, cx.listener(Self::on_mouse_up))
            .on_mouse_move(cx.listener(Self::on_mouse_move))
            .child(InputElement { input: cx.entity() })
    }
}

impl Focusable for TextInput {
    fn focus_handle(&self, _: &App) -> FocusHandle {
        self.focus_handle.clone()
    }
}

#[cfg(test)]
mod tests {
    use super::line_starts;

    #[test]
    fn line_starts_follow_newlines() {
        assert_eq!(line_starts("ab\ncd\n"), vec![0, 3, 6]);
        assert_eq!(line_starts(""), vec![0]);
    }
}
