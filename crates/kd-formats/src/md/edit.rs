//! MdCraft's editing operations, ported from its `editor.js`: wrap (bold, italic …), headings,
//! lists and quotes, blocks, tables, front matter, footnotes and slides. Each takes the text and
//! the selection and returns the [`Edit`] to apply (or `None` when there is nothing to do), so
//! they are plain functions, tested without a window.
//!
//! In inserted text, [`SEL`]…[`SEL`] marks what is selected afterwards (MdCraft's convention).

use kd_doc::{Change, Selection};

/// Marks the text to select after an insert.
pub const SEL: char = '\u{1}';

/// Changes and the selection after them.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Edit {
    pub changes: Vec<Change>,
    pub selection: Selection,
}

/// Remove the [`SEL`] marks; returns the text and the marked range (relative).
pub fn take_marks(s: &str) -> (String, Option<(usize, usize)>) {
    let Some(a) = s.find(SEL) else { return (s.to_string(), None) };
    let mut out = String::with_capacity(s.len());
    out.push_str(&s[..a]);
    let rest = &s[a + SEL.len_utf8()..];
    match rest.find(SEL) {
        Some(b) => {
            out.push_str(&rest[..b]);
            out.push_str(&rest[b + SEL.len_utf8()..]);
            (out, Some((a, a + b)))
        }
        None => {
            out.push_str(rest);
            (out, Some((a, a)))
        }
    }
}

// ---------------------------------------------------------------------------------- lines

/// `(start, end)` of the line holding `pos` (end excludes the newline).
pub fn line_at(text: &str, pos: usize) -> (usize, usize) {
    let pos = pos.min(text.len());
    let start = text[..pos].rfind('\n').map(|i| i + 1).unwrap_or(0);
    let end = text[pos..].find('\n').map(|i| pos + i).unwrap_or(text.len());
    (start, end)
}

/// The lines the selection covers.
pub fn selected_lines(text: &str, sel: Selection) -> Vec<(usize, usize)> {
    let mut out = Vec::new();
    let (mut s, mut e) = line_at(text, sel.start());
    loop {
        out.push((s, e));
        if e >= sel.end() || e >= text.len() {
            break;
        }
        s = e + 1;
        e = text[s..].find('\n').map(|i| s + i).unwrap_or(text.len());
    }
    out
}

/// Every line's `(start, end)`.
fn all_lines(text: &str) -> Vec<(usize, usize)> {
    let mut out = Vec::new();
    let mut s = 0;
    loop {
        let e = text[s..].find('\n').map(|i| s + i).unwrap_or(text.len());
        out.push((s, e));
        if e >= text.len() {
            break;
        }
        s = e + 1;
    }
    out
}

// ---------------------------------------------------------------------------------- insert

/// Replace the selection with `ins` (with optional [`SEL`] marks).
pub fn insert(_text: &str, sel: Selection, ins: &str) -> Edit {
    let (ins, marks) = take_marks(ins);
    let from = sel.start();
    let selection = match marks {
        Some((a, b)) => Selection::new(from + a, from + b),
        None => Selection::cursor(from + ins.len()),
    };
    Edit { changes: vec![Change::new(from, sel.end(), ins)], selection }
}

/// Insert a block on lines of its own, with a blank line before and after; after the
/// paragraph, table or list the cursor is in, not inside it.
pub fn insert_block(text: &str, sel: Selection, block: &str) -> Edit {
    let lines = all_lines(text);
    let idx_of = |pos: usize| lines.iter().position(|(s, e)| pos >= *s && pos <= *e).unwrap_or(lines.len() - 1);
    let li = idx_of(sel.start());
    let (ls, le) = lines[li];
    let at_start_of_empty = text[ls..le].trim().is_empty();
    let mut end_li = idx_of(sel.end());
    if !at_start_of_empty {
        while end_li + 1 < lines.len() && !text[lines[end_li + 1].0..lines[end_li + 1].1].trim().is_empty() {
            end_li += 1;
        }
    }
    let pos = if at_start_of_empty { ls } else { lines[end_li].1 };
    let before = if !at_start_of_empty {
        "\n\n"
    } else if li > 0 && !text[lines[li - 1].0..lines[li - 1].1].trim().is_empty() {
        "\n"
    } else {
        ""
    };
    let next = (if at_start_of_empty { li } else { end_li }) + 1;
    // the newline after the insert point stays: end with one only when no blank line follows
    let after = match lines.get(next) {
        None => "\n",
        Some((s, e)) if text[*s..*e].trim().is_empty() => "",
        Some(_) => "\n",
    };
    let to = if at_start_of_empty { le } else { pos };
    let (body, marks) = take_marks(block);
    let ins = format!("{before}{body}{after}");
    let selection = match marks {
        Some((a, b)) => Selection::new(pos + before.len() + a, pos + before.len() + b),
        None => Selection::cursor(pos + ins.len() - after.len()),
    };
    // replacing past the end of an empty last line would eat nothing: keep the newline count right
    let to = to.min(text.len());
    Edit { changes: vec![Change::new(pos, to, ins)], selection }
}

// ---------------------------------------------------------------------------------- inline

fn is_word(c: char) -> bool {
    c.is_alphanumeric() || c == '_' || c == '\'' || c == '-'
}

/// The word around `pos` when the cursor is inside one (Word applies formatting to it).
pub fn word_at(text: &str, pos: usize) -> Option<(usize, usize)> {
    let (ls, le) = line_at(text, pos);
    let line = &text[ls..le];
    let off = pos - ls;
    let a = line[..off].char_indices().rev().take_while(|(_, c)| is_word(*c)).last().map(|(i, _)| i).unwrap_or(off);
    let b = line[off..].char_indices().find(|(_, c)| !is_word(*c)).map(|(i, _)| off + i).unwrap_or(line.len());
    (a < off && off < b).then_some((ls + a, ls + b))
}

/// Bold, italic …: wrap the selection (or the word at the cursor) in `mark` … `close`, or
/// unwrap it when it already is.
pub fn toggle_wrap(text: &str, sel: Selection, mark: &str, close: Option<&str>, placeholder: Option<&str>) -> Edit {
    let close = close.unwrap_or(mark);
    let (mut from, mut to) = (sel.start(), sel.end());
    if from == to
        && let Some((a, b)) = word_at(text, from)
    {
        from = a;
        to = b;
    }
    let before = text.get(from.saturating_sub(mark.len())..from).unwrap_or("");
    let after = text.get(to..(to + close.len()).min(text.len())).unwrap_or("");
    let inner = &text[from..to];
    if before == mark && after == close && from >= mark.len() {
        return Edit {
            changes: vec![Change::new(from - mark.len(), from, ""), Change::new(to, to + close.len(), "")],
            selection: Selection::new(from - mark.len(), to - mark.len()),
        };
    }
    if inner.len() >= mark.len() + close.len() && inner.starts_with(mark) && inner.ends_with(close) {
        let un = &inner[mark.len()..inner.len() - close.len()];
        return Edit { changes: vec![Change::new(from, to, un)], selection: Selection::new(from, from + un.len()) };
    }
    if from == to {
        let ph = placeholder.unwrap_or("text");
        return Edit {
            changes: vec![Change::insert(from, format!("{mark}{ph}{close}"))],
            selection: Selection::new(from + mark.len(), from + mark.len() + ph.len()),
        };
    }
    Edit {
        changes: vec![Change::insert(from, mark), Change::insert(to, close)],
        selection: Selection::new(from + mark.len(), to + mark.len()),
    }
}

/// Inline formats and line kinds at the cursor (approximate, as in MdCraft).
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct Formats {
    pub bold: bool,
    pub italic: bool,
    pub strike: bool,
    pub highlight: bool,
    pub code: bool,
    pub underline: bool,
    pub bullet: bool,
    pub number: bool,
    pub task: bool,
    pub quote: bool,
    /// 0 = not a heading.
    pub heading: u8,
}

pub fn active_formats(text: &str, pos: usize) -> Formats {
    let (ls, le) = line_at(text, pos);
    let line = &text[ls..le];
    let (before, after) = line.split_at(pos.min(le) - ls);
    let count = |s: &str, m: &str| s.matches(m).count();
    let mut f = Formats {
        bold: count(before, "**") % 2 == 1 && after.contains("**"),
        strike: count(before, "~~") % 2 == 1 && after.contains("~~"),
        highlight: count(before, "==") % 2 == 1 && after.contains("=="),
        code: count(before, "`") % 2 == 1 && after.contains('`'),
        underline: before.rfind("<u>").is_some_and(|i| !before[i..].contains("</u>")) && after.contains("</u>"),
        ..Formats::default()
    };
    let b1 = before.replace("**", "");
    let a1 = after.replace("**", "");
    f.italic = count(&b1, "*") % 2 == 1 && a1.contains('*');
    let t = line.trim_start();
    if let Some(rest) = t.strip_prefix(['-', '*', '+'])
        && rest.starts_with(' ')
    {
        let r = rest.trim_start();
        if r.starts_with("[ ]") || r.starts_with("[x]") || r.starts_with("[X]") {
            f.task = true;
        } else {
            f.bullet = true;
        }
    } else if t.chars().next().is_some_and(|c| c.is_ascii_digit()) {
        let digits = t.chars().take_while(|c| c.is_ascii_digit()).count();
        if t[digits..].starts_with(". ") || t[digits..].starts_with(") ") {
            f.number = true;
        }
    }
    f.quote = t.starts_with('>');
    f.heading = heading_level(line);
    f
}

/// Remove inline Markdown formatting from the selection.
pub fn clear_formatting(text: &str, sel: Selection) -> Option<Edit> {
    if sel.is_empty() {
        return None;
    }
    let mut s = text[sel.range()].to_string();
    for tag in ["u", "sub", "sup", "mark", "b", "i", "em", "strong", "span"] {
        s = strip_tag(&s, tag);
    }
    for m in ["**", "__", "~~", "==", "`", "*", "_", "^"] {
        s = strip_pairs(&s, m);
    }
    let from = sel.start();
    Some(Edit { selection: Selection::new(from, from + s.len()), changes: vec![Change::new(from, sel.end(), s)] })
}

fn strip_tag(s: &str, tag: &str) -> String {
    let mut out = String::with_capacity(s.len());
    let mut rest = s;
    let open = format!("<{tag}");
    let close = format!("</{tag}>");
    loop {
        let lower = rest.to_ascii_lowercase();
        let next_open = lower.find(&open).filter(|&i| lower[i + open.len()..].starts_with(['>', ' ']));
        let next_close = lower.find(&close);
        let next = match (next_open, next_close) {
            (Some(a), Some(b)) => a.min(b),
            (Some(a), None) => a,
            (None, Some(b)) => b,
            (None, None) => break,
        };
        out.push_str(&rest[..next]);
        let end = rest[next..].find('>').map(|i| next + i + 1).unwrap_or(rest.len());
        rest = &rest[end..];
    }
    out.push_str(rest);
    out
}

/// Remove `m` … `m` pairs (the text between them stays).
fn strip_pairs(s: &str, m: &str) -> String {
    let mut out = String::with_capacity(s.len());
    let mut rest = s;
    while let Some(a) = rest.find(m) {
        let after = &rest[a + m.len()..];
        match after.find(m) {
            Some(b) if b > 0 => {
                out.push_str(&rest[..a]);
                out.push_str(&after[..b]);
                rest = &after[b + m.len()..];
            }
            _ => {
                out.push_str(&rest[..a + m.len()]);
                rest = after;
            }
        }
    }
    out.push_str(rest);
    out
}

// ---------------------------------------------------------------------------------- lines

/// The heading level of a line (0 = none).
pub fn heading_level(line: &str) -> u8 {
    let indent = line.len() - line.trim_start_matches(' ').len();
    if indent > 3 {
        return 0;
    }
    let t = &line[indent..];
    let n = t.chars().take_while(|c| *c == '#').count();
    if (1..=6).contains(&n) && (t.len() == n || t[n..].starts_with([' ', '\t'])) { n as u8 } else { 0 }
}

/// Heading 1–6 for the selected lines; 0 = Normal text. The same level again turns it off.
pub fn set_heading(text: &str, sel: Selection, level: u8) -> Edit {
    let mut changes = Vec::new();
    for (s, e) in selected_lines(text, sel) {
        let line = &text[s..e];
        let indent = (line.len() - line.trim_start_matches(' ').len()).min(3);
        let cur = heading_level(line);
        let mark_len =
            if cur > 0 { cur as usize + line[indent + cur as usize..].len() - line[indent + cur as usize..].trim_start().len() } else { 0 };
        let from = s + indent;
        let to = from + mark_len;
        let ins = if level > 0 && cur != level { format!("{} ", "#".repeat(level as usize)) } else { String::new() };
        changes.push(Change::new(from, to, ins));
    }
    let selection = shift_selection(sel, &changes);
    Edit { changes, selection }
}

/// Bullets, numbering, checklist or quote.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LineKind {
    Bullet,
    Number,
    Task,
    Quote,
}

/// `(indent, marker)` lengths of a list line.
fn list_marker(line: &str) -> (usize, usize) {
    let indent = line.len() - line.trim_start().len();
    let t = &line[indent..];
    let mut n = 0;
    if t.starts_with(['-', '*', '+']) && t[1..].starts_with([' ', '\t']) {
        n = 1 + (t[1..].len() - t[1..].trim_start().len());
        let r = &t[n..];
        if (r.starts_with("[ ]") || r.starts_with("[x]") || r.starts_with("[X]")) && r[3..].starts_with([' ', '\t']) {
            n += 3 + (r[3..].len() - r[3..].trim_start().len());
        }
    } else {
        let digits = t.chars().take_while(|c| c.is_ascii_digit()).count();
        if digits > 0 && t[digits..].starts_with(['.', ')']) && t[digits + 1..].starts_with([' ', '\t']) {
            n = digits + 1 + (t[digits + 1..].len() - t[digits + 1..].trim_start().len());
        }
    }
    (indent, n)
}

/// Turn the kind on for the selected lines, or off when they all have it.
pub fn toggle_lines(text: &str, sel: Selection, kind: LineKind) -> Edit {
    let all = selected_lines(text, sel);
    let many = all.len() > 1;
    let lines: Vec<(usize, usize)> = all.into_iter().filter(|(s, e)| !many || !text[*s..*e].trim().is_empty()).collect();
    let has = |l: &str| {
        let (i, n) = list_marker(l);
        let mk = &l[i..i + n];
        match kind {
            LineKind::Quote => l.trim_start().starts_with('>'),
            LineKind::Task => mk.contains('['),
            LineKind::Number => mk.starts_with(|c: char| c.is_ascii_digit()),
            LineKind::Bullet => mk.starts_with(['-', '*', '+']) && !mk.contains('['),
        }
    };
    let all_have = lines.iter().all(|(s, e)| has(&text[*s..*e]));
    let mut changes = Vec::new();
    let mut n = 1;
    for (s, e) in lines {
        let l = &text[s..e];
        if kind == LineKind::Quote {
            if all_have {
                let i = l.len() - l.trim_start().len();
                let gt = if l[i + 1..].starts_with(' ') { 2 } else { 1 };
                changes.push(Change::new(s + i, s + i + gt, ""));
            } else {
                changes.push(Change::insert(s, "> "));
            }
            continue;
        }
        let (i, m) = list_marker(l);
        let from = s + i;
        let ins = if all_have {
            String::new()
        } else {
            match kind {
                LineKind::Bullet => "- ".to_string(),
                LineKind::Task => "- [ ] ".to_string(),
                _ => {
                    let x = format!("{n}. ");
                    n += 1;
                    x
                }
            }
        };
        changes.push(Change::new(from, from + m, ins));
    }
    let selection = shift_selection(sel, &changes);
    Edit { changes, selection }
}

/// Indent (`more`) or outdent the selected lines by two spaces.
pub fn indent(text: &str, sel: Selection, more: bool) -> Option<Edit> {
    let mut changes = Vec::new();
    for (s, e) in selected_lines(text, sel) {
        if more {
            changes.push(Change::insert(s, "  "));
        } else {
            let n = text[s..e].chars().take(2).take_while(|c| *c == ' ').count();
            if n > 0 {
                changes.push(Change::new(s, s + n, ""));
            }
        }
    }
    if changes.is_empty() {
        return None;
    }
    let selection = shift_selection(sel, &changes);
    Some(Edit { changes, selection })
}

/// Map a selection through changes (positions in the old text).
pub fn shift_selection(sel: Selection, changes: &[Change]) -> Selection {
    let map = |p: usize| {
        let mut d: isize = 0;
        for c in changes {
            if c.to <= p {
                d += c.insert.len() as isize - (c.to - c.from) as isize;
            } else if c.from < p {
                // inside a replaced range: to its new end
                return (c.from as isize + d + c.insert.len() as isize) as usize;
            }
        }
        (p as isize + d).max(0) as usize
    };
    Selection::new(map(sel.anchor), map(sel.head))
}

// ---------------------------------------------------------------------------------- front matter

/// The front matter block: `(start, end)` of the whole block (fences included) and its body.
pub fn front_matter_span(text: &str) -> Option<(usize, usize, &str)> {
    let rest = text.strip_prefix("---\n").or_else(|| text.strip_prefix("---\r\n"))?;
    let body_start = text.len() - rest.len();
    let mut pos = body_start;
    for line in rest.split_inclusive('\n') {
        let t = line.trim_end_matches(['\n', '\r']);
        if t == "---" || t == "..." {
            return Some((0, pos + line.len(), &text[body_start..pos.saturating_sub(1).max(body_start)]));
        }
        pos += line.len();
    }
    None
}

/// Front matter keys and values.
pub fn front_matter(text: &str) -> Vec<(String, String)> {
    front_matter_span(text).map(|(_, _, body)| super::parse::front_matter_pairs(body)).unwrap_or_default()
}

/// Where the document's body starts (after the front matter).
pub fn body_start(text: &str) -> usize {
    front_matter_span(text).map(|(_, e, _)| e).unwrap_or(0)
}

/// Set (or with `None`, remove) top-level front matter keys; creates the block when needed.
pub fn set_front_matter(text: &str, sel: Selection, pairs: &[(&str, Option<&str>)]) -> Edit {
    let span = front_matter_span(text);
    let mut lines: Vec<String> = span.map(|(_, _, b)| b.lines().map(str::to_string).collect()).unwrap_or_default();
    let quote = |v: &str| {
        if v.contains([':', '#', '[', ']', '{', '}', '&', '*', '!', '|', '>', '\'', '"', '%', '@', '`'])
            || v.starts_with(' ')
            || v.ends_with(' ')
        {
            format!("\"{}\"", v.replace('"', "\\\""))
        } else {
            v.to_string()
        }
    };
    for (k, v) in pairs {
        let i = lines.iter().position(|l| l.split_once(':').is_some_and(|(kk, _)| kk.trim() == *k) && !l.starts_with(' '));
        match (v.filter(|v| !v.is_empty()), i) {
            (None, Some(i)) => {
                lines.remove(i);
            }
            (Some(v), Some(i)) => lines[i] = format!("{k}: {}", quote(v)),
            (Some(v), None) => lines.push(format!("{k}: {}", quote(v))),
            (None, None) => {}
        }
    }
    let block = if lines.is_empty() { String::new() } else { format!("---\n{}\n---\n", lines.join("\n")) };
    let (from, to) = span.map(|(s, e, _)| (s, e)).unwrap_or((0, 0));
    let mut ins = block;
    if span.is_none() && !ins.is_empty() && !text.starts_with('\n') {
        ins.push('\n');
    }
    let change = Change::new(from, to, ins);
    let selection = shift_selection(sel, std::slice::from_ref(&change));
    Edit { changes: vec![change], selection }
}

// ---------------------------------------------------------------------------------- tables

/// A new table: a header row and `rows - 1` empty rows; "Column 1" is selected.
pub fn table_markdown(rows: usize, cols: usize) -> String {
    let cols = cols.max(1);
    let head: Vec<String> = (0..cols).map(|i| if i == 0 { format!("{SEL}Column 1{SEL}") } else { format!("Column {}", i + 1) }).collect();
    let sep = vec!["---"; cols].join(" | ");
    let body: Vec<String> = (0..rows.saturating_sub(1).max(1)).map(|_| format!("| {} |", vec!["   "; cols].join(" | "))).collect();
    format!("| {} |\n| {sep} |\n{}", head.join(" | "), body.join("\n"))
}

fn is_row(l: &str) -> bool {
    let t = l.trim();
    (t.starts_with('|') && t.ends_with('|') && t.len() > 1) || (!t.starts_with('|') && t.contains('|') && !t.is_empty())
}

fn split_row(l: &str) -> Vec<String> {
    let mut t = l.trim();
    t = t.strip_prefix('|').unwrap_or(t);
    if t.ends_with('|') && !t.ends_with("\\|") {
        t = &t[..t.len() - 1];
    }
    let mut cells = Vec::new();
    let mut cur = String::new();
    let mut prev = ' ';
    for c in t.chars() {
        if c == '|' && prev != '\\' {
            cells.push(cur.trim().to_string());
            cur.clear();
        } else {
            cur.push(c);
        }
        prev = c;
    }
    cells.push(cur.trim().to_string());
    cells
}

fn is_separator(l: &str) -> bool {
    let cells = split_row(l);
    !cells.is_empty()
        && cells.iter().all(|c| {
            let c = c.trim_matches(':');
            !c.is_empty() && c.chars().all(|x| x == '-')
        })
}

/// The table around the cursor: `(from, to, rows, cursor row)`.
pub fn table_at(text: &str, pos: usize) -> Option<(usize, usize, Vec<Vec<String>>, usize)> {
    let lines = all_lines(text);
    let cur = lines.iter().position(|(s, e)| pos >= *s && pos <= *e)?;
    if !is_row(&text[lines[cur].0..lines[cur].1]) {
        return None;
    }
    let (mut a, mut b) = (cur, cur);
    while a > 0 && is_row(&text[lines[a - 1].0..lines[a - 1].1]) {
        a -= 1;
    }
    while b + 1 < lines.len() && is_row(&text[lines[b + 1].0..lines[b + 1].1]) {
        b += 1;
    }
    if b - a < 1 || !is_separator(&text[lines[a + 1].0..lines[a + 1].1]) {
        return None;
    }
    let rows = (a..=b).map(|i| split_row(&text[lines[i].0..lines[i].1])).collect();
    Some((lines[a].0, lines[b].1, rows, cur - a))
}

/// Align the pipes of the table at the cursor, after `mutate` changes its rows.
pub fn format_table(text: &str, sel: Selection, mutate: impl FnOnce(Vec<Vec<String>>, usize) -> Vec<Vec<String>>) -> Option<Edit> {
    let (from, to, rows, cur) = table_at(text, sel.head)?;
    let mut rows = mutate(rows, cur);
    let n = rows.iter().map(Vec::len).max().unwrap_or(1);
    for r in &mut rows {
        r.resize(n, String::new());
    }
    let aligns: Vec<char> = rows[1]
        .iter()
        .map(|c| match (c.starts_with(':'), c.ends_with(':')) {
            (true, true) => 'c',
            (false, true) => 'r',
            (true, false) => 'l',
            _ => ' ',
        })
        .collect();
    let width: Vec<usize> = (0..n)
        .map(|i| rows.iter().enumerate().filter(|(j, _)| *j != 1).map(|(_, r)| r[i].chars().count()).max().unwrap_or(0).max(3))
        .collect();
    let pad = |s: &str, k: usize, a: char| {
        let sp = k.saturating_sub(s.chars().count());
        match a {
            'r' => format!("{}{s}", " ".repeat(sp)),
            'c' => format!("{}{s}{}", " ".repeat(sp / 2), " ".repeat(sp - sp / 2)),
            _ => format!("{s}{}", " ".repeat(sp)),
        }
    };
    let out: Vec<String> = rows
        .iter()
        .enumerate()
        .map(|(j, r)| {
            if j == 1 {
                let cells: Vec<String> = width
                    .iter()
                    .zip(&aligns)
                    .map(|(k, a)| {
                        let d = "-".repeat(
                            (*k).saturating_sub(match a {
                                'c' => 2,
                                'l' | 'r' => 1,
                                _ => 0,
                            })
                            .max(1),
                        );
                        match a {
                            'c' => format!(":{d}:"),
                            'r' => format!("{d}:"),
                            'l' => format!(":{d}"),
                            _ => d,
                        }
                    })
                    .collect();
                format!("| {} |", cells.join(" | "))
            } else {
                format!("| {} |", r.iter().zip(&width).zip(&aligns).map(|((c, k), a)| pad(c, *k, *a)).collect::<Vec<_>>().join(" | "))
            }
        })
        .collect();
    let out = out.join("\n");
    let head = (from + sel.head.saturating_sub(from)).min(from + out.len());
    Some(Edit { changes: vec![Change::new(from, to, out)], selection: Selection::cursor(head) })
}

pub fn table_add_row(text: &str, sel: Selection) -> Option<Edit> {
    format_table(text, sel, |mut rows, cur| {
        let empty = vec![String::new(); rows[0].len()];
        rows.insert((cur + 1).max(2), empty);
        rows
    })
}

pub fn table_add_col(text: &str, sel: Selection) -> Option<Edit> {
    format_table(text, sel, |rows, _| {
        rows.into_iter()
            .enumerate()
            .map(|(j, mut r)| {
                r.push(match j {
                    0 => "New".into(),
                    1 => "---".into(),
                    _ => String::new(),
                });
                r
            })
            .collect()
    })
}

pub fn table_del_row(text: &str, sel: Selection) -> Option<Edit> {
    format_table(text, sel, |mut rows, cur| {
        if cur >= 2 && rows.len() > 3 {
            rows.remove(cur);
        }
        rows
    })
}

// ---------------------------------------------------------------------------------- footnotes, slides

/// A footnote reference at the cursor and its text at the end of the document.
pub fn footnote(text: &str, sel: Selection) -> Edit {
    let mut n = 1;
    while text.contains(&format!("[^{n}]")) {
        n += 1;
    }
    let r = format!("[^{n}]");
    let tail = format!("{}\n[^{n}]: ", if text.ends_with('\n') { "" } else { "\n" });
    let end = text.len();
    let start = end + r.len() + tail.len();
    Edit {
        changes: vec![Change::insert(sel.end(), r), Change::insert(end, format!("{tail}Footnote text."))],
        selection: Selection::new(start, start + "Footnote text.".len()),
    }
}

/// Lines (0-based) of `<!-- slide -->` markers outside code fences.
pub fn slide_markers(text: &str) -> Vec<usize> {
    let mut out = Vec::new();
    let mut fence = false;
    for (i, l) in text.lines().enumerate() {
        let t = l.trim_start();
        if t.starts_with("```") || t.starts_with("~~~") {
            fence = !fence;
        } else if !fence && t.starts_with("<!--") && t[4..].trim_start().starts_with("slide") {
            out.push(i);
        }
    }
    out
}

/// A new slide after the current one, with a layout's text (`cls` is its class).
pub fn new_slide(text: &str, sel: Selection, cls: &str, body: &str) -> Edit {
    let lines = all_lines(text);
    let cur = lines.iter().position(|(s, e)| sel.head >= *s && sel.head <= *e).unwrap_or(0);
    let markers = slide_markers(text);
    // the slide ends before the next marker after the cursor, or at the end
    let end_line = markers.iter().find(|m| **m > cur).map(|m| m.saturating_sub(1)).unwrap_or(lines.len() - 1);
    let (_, end) = lines[end_line];
    let marker = if cls.is_empty() { "<!-- slide -->".to_string() } else { format!("<!-- slide class=\"{cls}\" -->") };
    let prefix = if text[lines[end_line].0..end].trim().is_empty() { "\n" } else { "\n\n" };
    let (body, marks) = take_marks(body);
    let mut ins = format!("{prefix}{marker}\n\n{body}\n");
    // a blank line before the next slide's marker
    if end_line + 1 < lines.len() && !text[lines[end_line].0..end].trim().is_empty() {
        ins.push('\n');
    }
    let base = end + prefix.len() + marker.len() + 2;
    let selection = match marks {
        Some((a, b)) => Selection::new(base + a, base + b),
        None => Selection::cursor(end + ins.len()),
    };
    Edit { changes: vec![Change::insert(end, ins)], selection }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn run(text: &str, e: &Edit) -> String {
        let mut d = kd_doc::Document::new(text);
        assert!(d.apply(e.changes.clone(), e.selection, kd_doc::EditKind::Other), "{e:?}");
        d.text().to_string()
    }

    #[test]
    fn wrap_and_unwrap() {
        let t = "make this bold";
        let e = toggle_wrap(t, Selection::new(5, 9), "**", None, None);
        let t2 = run(t, &e);
        assert_eq!(t2, "make **this** bold");
        assert_eq!(e.selection, Selection::new(7, 11));
        let e = toggle_wrap(&t2, e.selection, "**", None, None);
        assert_eq!(run(&t2, &e), t);
        // the word at the cursor
        let e = toggle_wrap(t, Selection::cursor(6), "*", None, None);
        assert_eq!(run(t, &e), "make *this* bold");
        // nothing selected, no word: a placeholder, selected
        let e = toggle_wrap("a ", Selection::cursor(2), "`", None, Some("code"));
        assert_eq!(run("a ", &e), "a `code`");
        assert_eq!(e.selection, Selection::new(3, 7));
    }

    #[test]
    fn headings_toggle() {
        let t = "Title\nbody";
        let e = set_heading(t, Selection::cursor(2), 2);
        let t2 = run(t, &e);
        assert_eq!(t2, "## Title\nbody");
        let e = set_heading(&t2, Selection::cursor(4), 2);
        assert_eq!(run(&t2, &e), t);
        let e = set_heading(&t2, Selection::cursor(4), 1);
        assert_eq!(run(&t2, &e), "# Title\nbody");
        assert_eq!(heading_level("### x"), 3);
        assert_eq!(heading_level("#hashtag"), 0);
    }

    #[test]
    fn lists_and_quotes() {
        let t = "one\ntwo\n\nthree";
        let all = Selection::new(0, t.len());
        let t2 = run(t, &toggle_lines(t, all, LineKind::Number));
        assert_eq!(t2, "1. one\n2. two\n\n3. three");
        let t3 = run(&t2, &toggle_lines(&t2, Selection::new(0, t2.len()), LineKind::Bullet));
        assert_eq!(t3, "- one\n- two\n\n- three");
        let t4 = run(&t3, &toggle_lines(&t3, Selection::new(0, t3.len()), LineKind::Bullet));
        assert_eq!(t4, t);
        let q = run(t, &toggle_lines(t, Selection::cursor(0), LineKind::Quote));
        assert_eq!(q, "> one\ntwo\n\nthree");
        let task = run(t, &toggle_lines(t, Selection::cursor(0), LineKind::Task));
        assert_eq!(task, "- [ ] one\ntwo\n\nthree");
        assert!(active_formats(&task, 3).task);
    }

    #[test]
    fn blocks_go_after_the_paragraph() {
        let t = "First para\nstill first\n\nSecond";
        let e = insert_block(t, Selection::cursor(3), "---");
        assert_eq!(run(t, &e), "First para\nstill first\n\n---\n\nSecond");
        let e = insert_block("", Selection::cursor(0), &format!("{SEL}x{SEL}"));
        assert_eq!(run("", &e), "x\n");
        assert_eq!(e.selection, Selection::new(0, 1));
    }

    #[test]
    fn front_matter_set_and_remove() {
        let t = "# Doc\n";
        let t2 = run(t, &set_front_matter(t, Selection::cursor(0), &[("theme", Some("report")), ("title", Some("A: B"))]));
        assert_eq!(t2, "---\ntheme: report\ntitle: \"A: B\"\n---\n\n# Doc\n");
        assert_eq!(front_matter(&t2)[0], ("theme".into(), "report".into()));
        let t3 = run(&t2, &set_front_matter(&t2, Selection::cursor(0), &[("theme", None), ("title", None)]));
        assert_eq!(t3, "\n# Doc\n");
        assert_eq!(body_start(&t2), 36);
    }

    #[test]
    fn tables() {
        let (t, _) = take_marks(&table_markdown(3, 2));
        assert_eq!(t, "| Column 1 | Column 2 |\n| --- | --- |\n|     |     |\n|     |     |");
        let t = "| a | b |\n|:-|-:|\n| long cell | 2 |";
        let e = format_table(t, Selection::cursor(0), |r, _| r).unwrap();
        assert_eq!(run(t, &e), "| a         |   b |\n| :-------- | --: |\n| long cell |   2 |");
        let e = table_add_row(t, Selection::cursor(t.len())).unwrap();
        assert_eq!(run(t, &e).lines().count(), 4);
        let e = table_add_col(t, Selection::cursor(0)).unwrap();
        assert!(run(t, &e).starts_with("| a         |   b | New |"));
    }

    #[test]
    fn clearing_and_footnotes_and_slides() {
        let t = "**bold** *it* `c` ~~s~~ <u>u</u>";
        let e = clear_formatting(t, Selection::new(0, t.len())).unwrap();
        assert_eq!(run(t, &e), "bold it c s u");
        let e = footnote("Text", Selection::cursor(4));
        assert_eq!(run("Text", &e), "Text[^1]\n\n[^1]: Footnote text.");
        let deck = "# One\n\n<!-- slide -->\n\n## Two";
        assert_eq!(slide_markers(deck), vec![2]);
        let e = new_slide(deck, Selection::cursor(0), "", &format!("## {SEL}Title{SEL}"));
        let out = run(deck, &e);
        assert_eq!(out, "# One\n\n<!-- slide -->\n\n## Title\n\n<!-- slide -->\n\n## Two");
        assert_eq!(&out[e.selection.range()], "Title");
    }

    #[test]
    fn indenting() {
        let t = "- a\n- b";
        let t2 = run(t, &indent(t, Selection::new(0, t.len()), true).unwrap());
        assert_eq!(t2, "  - a\n  - b");
        let t3 = run(&t2, &indent(&t2, Selection::new(0, t2.len()), false).unwrap());
        assert_eq!(t3, t);
    }
}
