//! Highlighting of Markdown source for the editor, as MdCraft's editor colours it: headings,
//! the markup characters (dimmed), bold, italic, inline code and code blocks, links, quotes,
//! HTML, comments and front matter.

use std::ops::Range;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Tok {
    /// `#`, `**`, `` ` ``, `>`, list markers …: dimmed.
    Mark,
    Heading,
    Bold,
    Italic,
    BoldItalic,
    Strike,
    Highlight,
    Code,
    /// Inside a fenced code block.
    CodeBlock,
    /// ```` ```lang ```` lines.
    Fence,
    LinkText,
    Url,
    Quote,
    Html,
    Comment,
    /// Front matter keys.
    MetaKey,
    MetaValue,
    Math,
}

/// One line's tokens: ranges in the line (bytes), sorted, not overlapping.
pub type LineToks = Vec<(Range<usize>, Tok)>;

/// Tokens for every line of `text`.
pub fn highlight(text: &str) -> Vec<LineToks> {
    let mut out = Vec::new();
    let mut fence: Option<String> = None;
    let mut front = text.starts_with("---\n") || text.starts_with("---\r\n");
    let mut comment = false;
    let mut math = false;
    for (i, line) in text.split('\n').enumerate() {
        let line = line.strip_suffix('\r').unwrap_or(line);
        let t = line.trim_start();
        let indent = line.len() - t.len();
        let mut toks: LineToks = Vec::new();
        if front {
            if i > 0 && (t == "---" || t == "...") {
                front = false;
                toks.push((0..line.len(), Tok::Mark));
            } else if i == 0 {
                toks.push((0..line.len(), Tok::Mark));
            } else if let Some(c) = line.find(':') {
                toks.push((0..c, Tok::MetaKey));
                toks.push((c..c + 1, Tok::Mark));
                toks.push((c + 1..line.len(), Tok::MetaValue));
            }
            out.push(toks);
            continue;
        }
        if let Some(f) = &fence {
            if t.starts_with(f.as_str()) && t[f.len()..].trim().is_empty() {
                toks.push((0..line.len(), Tok::Fence));
                fence = None;
            } else {
                toks.push((0..line.len(), Tok::CodeBlock));
            }
            out.push(toks);
            continue;
        }
        if comment {
            toks.push((0..line.len(), Tok::Comment));
            if line.contains("-->") {
                comment = false;
            }
            out.push(toks);
            continue;
        }
        if math {
            toks.push((0..line.len(), Tok::Math));
            if t.starts_with("$$") {
                math = false;
            }
            out.push(toks);
            continue;
        }
        if indent < 4 && (t.starts_with("```") || t.starts_with("~~~")) {
            let ch = &t[..1];
            let n = t.chars().take_while(|c| c.to_string() == ch).count();
            fence = Some(ch.repeat(n));
            toks.push((0..line.len(), Tok::Fence));
            out.push(toks);
            continue;
        }
        if let Some(rest) = t.strip_prefix("$$") {
            toks.push((0..line.len(), Tok::Math));
            if !rest.contains("$$") {
                math = true;
            }
            out.push(toks);
            continue;
        }
        if t.starts_with("<!--") {
            toks.push((0..line.len(), Tok::Comment));
            if !line.contains("-->") {
                comment = true;
            }
            out.push(toks);
            continue;
        }
        if t.starts_with('<') && t[1..].starts_with(|c: char| c.is_ascii_alphabetic() || c == '/') {
            toks.push((0..line.len(), Tok::Html));
            out.push(toks);
            continue;
        }
        // heading
        let level = super::edit::heading_level(line);
        if level > 0 {
            let mark_end = indent + level as usize;
            toks.push((0..mark_end, Tok::Mark));
            toks.push((mark_end..line.len(), Tok::Heading));
            out.push(toks);
            continue;
        }
        // thematic break
        let bare: String = t.chars().filter(|c| !c.is_whitespace()).collect();
        if bare.len() >= 3 && (bare.chars().all(|c| c == '-') || bare.chars().all(|c| c == '*') || bare.chars().all(|c| c == '_')) {
            toks.push((0..line.len(), Tok::Mark));
            out.push(toks);
            continue;
        }
        // quote / list prefixes
        let mut pos = 0;
        let mut quote = false;
        while line[pos..].trim_start().starts_with('>') {
            let gt = pos + (line[pos..].len() - line[pos..].trim_start().len());
            toks.push((pos..gt + 1, Tok::Mark));
            pos = gt + 1;
            quote = true;
        }
        let rest = &line[pos..];
        let lead = rest.len() - rest.trim_start().len();
        let r = rest.trim_start();
        let marker = if r.starts_with(['-', '*', '+']) && r[1..].starts_with(' ') {
            let after = &r[2..];
            Some(if after.starts_with("[ ]") || after.starts_with("[x]") || after.starts_with("[X]") { 5 } else { 2 })
        } else {
            let d = r.chars().take_while(|c| c.is_ascii_digit()).count();
            (d > 0 && r[d..].starts_with(['.', ')']) && r[d + 1..].starts_with(' ')).then_some(d + 2)
        };
        if let Some(n) = marker {
            toks.push((pos..pos + lead + n, Tok::Mark));
            pos += lead + n;
        }
        inline(line, pos, quote, &mut toks);
        out.push(toks);
    }
    out
}

/// Inline tokens from `start` to the end of the line.
fn inline(line: &str, start: usize, quote: bool, toks: &mut LineToks) {
    let b = line.as_bytes();
    let base = if quote { Some(Tok::Quote) } else { None };
    let mut i = start;
    let mut plain_from = start;
    let flush = |from: usize, to: usize, toks: &mut LineToks| {
        if let Some(t) = base
            && to > from
        {
            toks.push((from..to, t));
        }
    };
    while i < b.len() {
        let at = i;
        let c = b[i];
        // an HTML comment in the line (comments on the text)
        if c == b'<'
            && line[i..].starts_with("<!--")
            && let Some(end) = line[i + 4..].find("-->")
        {
            flush(plain_from, i, toks);
            toks.push((i..i + 4 + end + 3, Tok::Comment));
            i += 4 + end + 3;
            plain_from = i;
            continue;
        }
        // inline code
        if c == b'`' {
            let n = b[i..].iter().take_while(|x| **x == b'`').count();
            let fence = &line[i..i + n];
            if let Some(end) = line[i + n..].find(fence) {
                flush(plain_from, i, toks);
                let close = i + n + end;
                toks.push((i..i + n, Tok::Mark));
                toks.push((i + n..close, Tok::Code));
                toks.push((close..close + n, Tok::Mark));
                i = close + n;
                plain_from = i;
                continue;
            }
        }
        // inline math $…$
        if c == b'$'
            && let Some(end) = line[i + 1..].find('$')
            && end > 0
            && !line[i + 1..].starts_with(' ')
        {
            flush(plain_from, i, toks);
            toks.push((i..i + end + 2, Tok::Math));
            i += end + 2;
            plain_from = i;
            continue;
        }
        // link / image: [text](url)
        if c == b'[' || (c == b'!' && b.get(i + 1) == Some(&b'[')) {
            let open = if c == b'!' { i + 1 } else { i };
            if let Some(close) = find_close(line, open)
                && line[close + 1..].starts_with('(')
                && let Some(end) = line[close + 2..].find(')')
            {
                flush(plain_from, i, toks);
                let url_end = close + 2 + end;
                toks.push((i..open + 1, Tok::Mark));
                toks.push((open + 1..close, Tok::LinkText));
                toks.push((close..close + 2, Tok::Mark));
                toks.push((close + 2..url_end, Tok::Url));
                toks.push((url_end..url_end + 1, Tok::Mark));
                i = url_end + 1;
                plain_from = i;
                continue;
            }
        }
        // emphasis and friends
        for (mark, tok) in [
            ("***", Tok::BoldItalic),
            ("**", Tok::Bold),
            ("__", Tok::Bold),
            ("~~", Tok::Strike),
            ("==", Tok::Highlight),
            ("*", Tok::Italic),
            ("_", Tok::Italic),
        ] {
            if line[i..].starts_with(mark) {
                let inner_start = i + mark.len();
                // `_` inside words is not emphasis
                if mark == "_" && i > 0 && line[..i].chars().last().is_some_and(char::is_alphanumeric) {
                    break;
                }
                if line[inner_start..].starts_with(' ') || inner_start >= line.len() {
                    break;
                }
                if let Some(end) = line[inner_start..].find(mark)
                    && end > 0
                {
                    flush(plain_from, i, toks);
                    let close = inner_start + end;
                    toks.push((i..inner_start, Tok::Mark));
                    toks.push((inner_start..close, tok));
                    toks.push((close..close + mark.len(), Tok::Mark));
                    i = close + mark.len();
                    plain_from = i;
                }
                break;
            }
        }
        if i == at {
            // nothing matched here: the next character
            i += line[i..].chars().next().map(char::len_utf8).unwrap_or(1);
        }
    }
    flush(plain_from, line.len(), toks);
}

/// The `]` that closes the `[` at `open`.
fn find_close(line: &str, open: usize) -> Option<usize> {
    let mut depth = 0;
    for (k, c) in line[open..].char_indices() {
        match c {
            '[' => depth += 1,
            ']' => {
                depth -= 1;
                if depth == 0 {
                    return Some(open + k);
                }
            }
            _ => {}
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    fn kinds(line: &str) -> Vec<(String, Tok)> {
        highlight(line)[0].iter().map(|(r, t)| (line[r.clone()].to_string(), *t)).collect()
    }

    #[test]
    fn headings_and_inline() {
        assert_eq!(kinds("## Title"), vec![("##".into(), Tok::Mark), (" Title".into(), Tok::Heading)]);
        let k = kinds("a **b** and `c` [d](e)");
        assert!(k.contains(&("b".into(), Tok::Bold)));
        assert!(k.contains(&("c".into(), Tok::Code)));
        assert!(k.contains(&("d".into(), Tok::LinkText)));
        assert!(k.contains(&("e".into(), Tok::Url)));
        assert!(kinds("snake_case_word").is_empty());
    }

    #[test]
    fn blocks_carry_state() {
        let h = highlight("---\ntitle: X\n---\n```rust\nfn main() {}\n```\n- item\n> quote *it*");
        assert_eq!(h[1][0].1, Tok::MetaKey);
        assert_eq!(h[4][0].1, Tok::CodeBlock);
        assert_eq!(h[6][0], (0..2, Tok::Mark));
        assert!(h[7].iter().any(|(_, t)| *t == Tok::Italic));
    }

    #[test]
    fn never_panics_on_odd_text() {
        for s in ["*", "**", "`", "[", "![", "$", "é*é*", "> > >", "1.", "- [", "__init__", "a ~~ b"] {
            let _ = highlight(s);
        }
    }
}
