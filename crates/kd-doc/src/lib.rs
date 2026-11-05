//! # kd-doc — the KaDee document model
//!
//! A text document as the editors see it: the text, a [`Selection`], [`Change`]s applied as one
//! transaction, and grouped undo / redo. Offsets are UTF-8 byte offsets into the text.
//!
//! Every edit goes through [`Document::apply`], so the CRDT that collaboration needs can later
//! sit behind the same API (each transaction becomes a CRDT update).

use std::time::{Duration, Instant};

/// A cursor (`anchor == head`) or a selection from `anchor` to `head`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct Selection {
    pub anchor: usize,
    pub head: usize,
}

impl Selection {
    pub fn cursor(at: usize) -> Selection {
        Selection { anchor: at, head: at }
    }
    pub fn new(anchor: usize, head: usize) -> Selection {
        Selection { anchor, head }
    }
    pub fn start(&self) -> usize {
        self.anchor.min(self.head)
    }
    pub fn end(&self) -> usize {
        self.anchor.max(self.head)
    }
    pub fn is_empty(&self) -> bool {
        self.anchor == self.head
    }
    pub fn range(&self) -> std::ops::Range<usize> {
        self.start()..self.end()
    }
}

/// Replace `from..to` (in the text before the transaction) with `insert`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Change {
    pub from: usize,
    pub to: usize,
    pub insert: String,
}

impl Change {
    pub fn new(from: usize, to: usize, insert: impl Into<String>) -> Change {
        Change { from, to, insert: insert.into() }
    }
    pub fn insert(at: usize, text: impl Into<String>) -> Change {
        Change { from: at, to: at, insert: text.into() }
    }
}

/// What kind of edit, for grouping undo steps: typing letters joins one step.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EditKind {
    Typing,
    Deleting,
    /// A command, paste, a dialog …: always a step of its own.
    Other,
}

/// One applied transaction: the changes and the changes that undo them.
#[derive(Debug, Clone)]
struct Tx {
    forward: Vec<Change>,
    inverse: Vec<Change>,
}

/// A step of undo: transactions applied together.
#[derive(Debug, Clone)]
struct Step {
    txs: Vec<Tx>,
    before: Selection,
    after: Selection,
    kind: EditKind,
    at: Instant,
}

/// How long typing keeps joining the same undo step.
const GROUP: Duration = Duration::from_millis(1200);

/// A text document with its selection and history.
#[derive(Debug, Clone, Default)]
pub struct Document {
    text: String,
    selection: Selection,
    undo: Vec<Step>,
    redo: Vec<Step>,
    /// Goes up with every change (views cache layouts by it).
    version: u64,
    /// The version that was last saved.
    saved_version: u64,
}

impl Document {
    pub fn new(text: impl Into<String>) -> Document {
        Document { text: text.into(), ..Document::default() }
    }

    pub fn text(&self) -> &str {
        &self.text
    }

    pub fn len(&self) -> usize {
        self.text.len()
    }

    pub fn is_empty(&self) -> bool {
        self.text.is_empty()
    }

    pub fn version(&self) -> u64 {
        self.version
    }

    pub fn selection(&self) -> Selection {
        self.selection
    }

    /// Move the cursor or selection (kept inside the text, on char boundaries).
    pub fn set_selection(&mut self, s: Selection) {
        self.selection = Selection { anchor: self.snap(s.anchor), head: self.snap(s.head) };
    }

    /// True when there are changes since [`Document::mark_saved`].
    pub fn is_dirty(&self) -> bool {
        self.version != self.saved_version
    }

    pub fn mark_saved(&mut self) {
        self.saved_version = self.version;
    }

    pub fn can_undo(&self) -> bool {
        !self.undo.is_empty()
    }

    pub fn can_redo(&self) -> bool {
        !self.redo.is_empty()
    }

    /// The nearest char boundary at or before `at`.
    pub fn snap(&self, at: usize) -> usize {
        let mut o = at.min(self.text.len());
        while !self.text.is_char_boundary(o) {
            o -= 1;
        }
        o
    }

    /// Apply `changes` (positions in the current text, not overlapping) as one transaction and
    /// put the selection at `after`. Returns false (and changes nothing) when they are invalid.
    pub fn apply(&mut self, changes: Vec<Change>, after: Selection, kind: EditKind) -> bool {
        let before = self.selection;
        let Some(tx) = self.apply_raw(changes) else { return false };
        self.set_selection(after);
        let now = Instant::now();
        let join = kind != EditKind::Other
            && self.undo.last().is_some_and(|s| s.kind == kind && s.after == before && now.duration_since(s.at) < GROUP);
        if join && let Some(last) = self.undo.last_mut() {
            last.txs.push(tx);
            last.after = self.selection;
            last.at = now;
        } else {
            self.undo.push(Step { txs: vec![tx], before, after: self.selection, kind, at: now });
        }
        self.redo.clear();
        true
    }

    /// Replace the whole text (one undo step), keeping the cursor where it can stay.
    pub fn set_text(&mut self, text: &str) {
        let old = &self.text;
        // only the part that differs, so undo and the cursor stay sensible
        let a = old.bytes().zip(text.bytes()).take_while(|(x, y)| x == y).count();
        let mut a = a.min(old.len()).min(text.len());
        while !old.is_char_boundary(a) || !text.is_char_boundary(a) {
            a -= 1;
        }
        let max_b = (old.len() - a).min(text.len() - a);
        let mut b = old.bytes().rev().zip(text.bytes().rev()).take(max_b).take_while(|(x, y)| x == y).count();
        while !old.is_char_boundary(old.len() - b) || !text.is_char_boundary(text.len() - b) {
            b -= 1;
        }
        let change = Change::new(a, old.len() - b, &text[a..text.len() - b]);
        let cursor = self.selection.head.min(text.len());
        self.apply(vec![change], Selection::cursor(cursor), EditKind::Other);
    }

    pub fn undo(&mut self) -> bool {
        let Some(step) = self.undo.pop() else { return false };
        for tx in step.txs.iter().rev() {
            self.apply_raw(tx.inverse.clone());
        }
        self.set_selection(step.before);
        self.redo.push(step);
        true
    }

    pub fn redo(&mut self) -> bool {
        let Some(step) = self.redo.pop() else { return false };
        for tx in &step.txs {
            self.apply_raw(tx.forward.clone());
        }
        self.set_selection(step.after);
        self.undo.push(step);
        true
    }

    /// Apply changes without touching history; returns the transaction.
    fn apply_raw(&mut self, mut changes: Vec<Change>) -> Option<Tx> {
        changes.sort_by_key(|c| (c.from, c.to));
        let mut last = 0;
        for c in &changes {
            if c.from < last
                || c.to < c.from
                || c.to > self.text.len()
                || !self.text.is_char_boundary(c.from)
                || !self.text.is_char_boundary(c.to)
            {
                return None;
            }
            last = c.to;
        }
        let mut out = String::with_capacity(self.text.len() + changes.iter().map(|c| c.insert.len()).sum::<usize>());
        let mut inverse = Vec::with_capacity(changes.len());
        let mut pos = 0;
        for c in &changes {
            out.push_str(&self.text[pos..c.from]);
            let start = out.len();
            out.push_str(&c.insert);
            inverse.push(Change::new(start, out.len(), &self.text[c.from..c.to]));
            pos = c.to;
        }
        out.push_str(&self.text[pos..]);
        self.text = out;
        self.version += 1;
        Some(Tx { forward: changes, inverse })
    }

    // ---- lines -----------------------------------------------------------------------------

    /// The 0-based line containing `offset`.
    pub fn line_of(&self, offset: usize) -> usize {
        self.text[..offset.min(self.text.len())].bytes().filter(|b| *b == b'\n').count()
    }

    /// Where line `line` (0-based) starts.
    pub fn line_start(&self, line: usize) -> usize {
        if line == 0 {
            return 0;
        }
        self.text.match_indices('\n').nth(line - 1).map(|(i, _)| i + 1).unwrap_or(self.text.len())
    }

    /// The number of lines (an empty text has one).
    pub fn line_count(&self) -> usize {
        self.text.bytes().filter(|b| *b == b'\n').count() + 1
    }

    /// 1-based line and column (in characters) of `offset`, for the status bar.
    pub fn line_col(&self, offset: usize) -> (usize, usize) {
        let offset = self.snap(offset);
        let line = self.line_of(offset);
        let start = self.line_start(line);
        (line + 1, self.text[start..offset].chars().count() + 1)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn apply_undo_redo() {
        let mut d = Document::new("hello world");
        assert!(d.apply(vec![Change::new(0, 5, "goodbye")], Selection::cursor(7), EditKind::Other));
        assert_eq!(d.text(), "goodbye world");
        assert!(d.is_dirty());
        assert!(d.undo());
        assert_eq!(d.text(), "hello world");
        assert!(d.redo());
        assert_eq!(d.text(), "goodbye world");
        assert_eq!(d.selection(), Selection::cursor(7));
    }

    #[test]
    fn several_changes_in_one_transaction() {
        let mut d = Document::new("abc");
        d.apply(vec![Change::insert(3, "**"), Change::insert(0, "**")], Selection::new(2, 5), EditKind::Other);
        assert_eq!(d.text(), "**abc**");
        d.undo();
        assert_eq!(d.text(), "abc");
    }

    #[test]
    fn typing_joins_one_undo_step() {
        let mut d = Document::new("");
        for (i, c) in "word".chars().enumerate() {
            d.apply(vec![Change::insert(i, c.to_string())], Selection::cursor(i + 1), EditKind::Typing);
        }
        assert_eq!(d.text(), "word");
        d.undo();
        assert_eq!(d.text(), "");
    }

    #[test]
    fn overlapping_or_bad_changes_are_refused() {
        let mut d = Document::new("héllo");
        assert!(!d.apply(vec![Change::new(0, 3, "x"), Change::new(2, 4, "y")], Selection::default(), EditKind::Other));
        assert!(!d.apply(vec![Change::new(2, 2, "x")], Selection::default(), EditKind::Other)); // inside é
        assert_eq!(d.text(), "héllo");
        assert!(!d.is_dirty());
    }

    #[test]
    fn set_text_changes_only_the_difference() {
        let mut d = Document::new("one two three");
        d.set_text("one 2 three");
        assert_eq!(d.text(), "one 2 three");
        d.undo();
        assert_eq!(d.text(), "one two three");
    }

    #[test]
    fn lines_and_columns() {
        let d = Document::new("ab\ncdé\n\nx");
        assert_eq!(d.line_count(), 4);
        assert_eq!(d.line_start(1), 3);
        assert_eq!(d.line_start(3), 9);
        assert_eq!(d.line_of(4), 1);
        assert_eq!(d.line_col(7), (2, 4));
    }

    #[test]
    fn saving_clears_dirty() {
        let mut d = Document::new("a");
        d.apply(vec![Change::insert(1, "b")], Selection::cursor(2), EditKind::Typing);
        d.mark_saved();
        assert!(!d.is_dirty());
        d.undo();
        assert!(d.is_dirty());
    }
}
