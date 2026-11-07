//! Document properties in the front matter, as Pandoc names them, and what they do to the
//! document: `toc` and `toc-depth` (a table of contents at the top), `numbersections`
//! (1, 1.1, 1.2 … before the headings), `abstract` (under the title), `keywords` (the files'
//! metadata) and `dir` (right to left).
//!
//! A document with a single level-1 heading treats it as the title: numbering then starts at
//! level 2.

use std::borrow::Cow;

use super::{Block, Inline, Parsed, Style};

/// A list value: `[a, b]`, `a, b` or `- a` lines folded by the parser → the items.
pub fn list(v: &str) -> Vec<String> {
    v.trim()
        .trim_start_matches('[')
        .trim_end_matches(']')
        .split(',')
        .map(|s| s.trim().trim_matches(|c| c == '"' || c == '\'').to_string())
        .filter(|s| !s.is_empty())
        .collect()
}

/// A list as a front matter value: one item plain, more as `[a, b]`.
pub fn list_value(items: &[String]) -> Option<String> {
    match items.len() {
        0 => None,
        1 => Some(items[0].clone()),
        _ => Some(format!("[{}]", items.join(", "))),
    }
}

fn get<'a>(front: &'a [(String, String)], k: &str) -> Option<&'a str> {
    front.iter().find(|(key, _)| key == k).map(|(_, v)| v.as_str())
}

fn on(front: &[(String, String)], k: &str) -> bool {
    get(front, k).is_some_and(|v| matches!(v.trim(), "true" | "yes" | "on" | "1"))
}

/// How deep the table of contents goes (1–6, 3 by default).
pub fn toc_depth(front: &[(String, String)]) -> u8 {
    get(front, "toc-depth").and_then(|v| v.trim().parse().ok()).unwrap_or(3).clamp(1, 6)
}

/// Whether headings are numbered (`numbersections`, or MdCraft's `numbered`).
pub fn numbered(front: &[(String, String)]) -> bool {
    on(front, "numbersections") || on(front, "numbered")
}

pub fn rtl(front: &[(String, String)]) -> bool {
    get(front, "dir").is_some_and(|v| v.trim() == "rtl")
}

/// The numbers of headings at these levels (in order): "1", "1.2" …; `None` for a heading
/// that is not numbered (the title).
pub fn numbers(levels: &[u8]) -> Vec<Option<String>> {
    let h1 = levels.iter().filter(|l| **l == 1).count();
    let top = if h1 == 1 { 2 } else { 1 };
    let mut counters = [0u32; 7];
    levels
        .iter()
        .map(|&l| {
            if l < top {
                return None;
            }
            let l = l.min(6) as usize;
            counters[l] += 1;
            for c in counters.iter_mut().skip(l + 1) {
                *c = 0;
            }
            let parts: Vec<String> = (top as usize..=l).map(|i| counters[i].max(if i < l { 1 } else { 0 }).to_string()).collect();
            Some(parts.join("."))
        })
        .collect()
}

/// The abstract as blocks: "Abstract" and its text, indented.
fn abstract_block(text: &str, line: usize) -> Block {
    let style = super::para::ParaStyle { left: Some(1.0), right: Some(1.0), after: Some(0.5), ..Default::default() };
    let mut blocks =
        vec![Block::Paragraph { inlines: vec![Inline::Text("Abstract".into(), Style { bold: true, ..Style::default() })], line }];
    for p in text.split("\\n\\n").flat_map(|p| p.split("\n\n")) {
        if !p.trim().is_empty() {
            blocks.push(Block::Paragraph { inlines: vec![Inline::Text(p.trim().to_string(), Style::default())], line });
        }
    }
    Block::Styled { style, blocks, line }
}

/// Number the headings, add the table of contents and the abstract (for the preview and the
/// exports, which work on the parsed document).
pub fn apply(parsed: &mut Parsed) {
    let front = parsed.front_matter.clone();
    if numbered(&front) {
        let levels: Vec<u8> =
            parsed.blocks.iter().filter_map(|b| if let Block::Heading { level, .. } = b { Some(*level) } else { None }).collect();
        let mut nums = numbers(&levels).into_iter();
        for b in parsed.blocks.iter_mut() {
            if let Block::Heading { inlines, .. } = b
                && let Some(Some(n)) = nums.next()
            {
                inlines.insert(0, Inline::Text(format!("{n} "), Style::default()));
            }
        }
    }
    // after a leading title: the abstract, then the table of contents
    let at = match parsed.blocks.first() {
        Some(Block::Heading { level: 1, .. }) => 1,
        _ => 0,
    };
    let line = parsed.blocks.get(at).or(parsed.blocks.last()).map(|b| b.line()).unwrap_or(1);
    let mut extra = Vec::new();
    if let Some(a) = get(&front, "abstract").filter(|a| !a.trim().is_empty()) {
        extra.push(abstract_block(a.trim().trim_matches('"'), line));
    }
    let has_toc = parsed.blocks.iter().any(|b| matches!(b, Block::Toc { .. }));
    if on(&front, "toc") && !has_toc {
        extra.push(Block::Toc { line });
    }
    for (i, b) in extra.into_iter().enumerate() {
        parsed.blocks.insert(at + i, b);
    }
}

/// The same for Markdown text (the web page export): numbers in the heading lines, the
/// abstract and `[TOC]` after the title.
pub fn expand_text(text: &str) -> Cow<'_, str> {
    let front = super::edit::front_matter(text);
    let numbered = numbered(&front);
    let abs = get(&front, "abstract").filter(|a| !a.trim().is_empty()).map(|a| a.trim().trim_matches('"').replace("\\n", "\n"));
    let toc = on(&front, "toc") && !text.contains("[TOC]");
    if !numbered && abs.is_none() && !toc {
        return Cow::Borrowed(text);
    }
    let body = super::edit::body_start(text);
    // the heading lines (outside code)
    let mut heads: Vec<(usize, u8)> = Vec::new();
    let mut fence: Option<&str> = None;
    let mut pos = body;
    for line in text[body..].split_inclusive('\n') {
        let t = line.trim_start();
        if let Some(f) = fence {
            if t.starts_with(f) {
                fence = None;
            }
        } else if t.starts_with("```") || t.starts_with("~~~") {
            fence = Some(&t[..3]);
        } else if line.starts_with('#') {
            let n = line.chars().take_while(|c| *c == '#').count();
            if n <= 6 && line[n..].starts_with([' ', '\t']) {
                heads.push((pos, n as u8));
            }
        }
        pos += line.len();
    }
    let mut out = String::with_capacity(text.len() + 256);
    out.push_str(&text[..body]);
    let nums = if numbered { numbers(&heads.iter().map(|(_, l)| *l).collect::<Vec<_>>()) } else { vec![None; heads.len()] };
    // where the extra blocks go: after the first line when it is a level-1 heading
    let title_end = heads
        .first()
        .filter(|(p, l)| *l == 1 && text[body..*p].trim().is_empty())
        .map(|(p, _)| text[*p..].find('\n').map(|e| p + e + 1).unwrap_or(text.len()));
    let mut extra = String::new();
    if let Some(a) = &abs {
        extra.push_str(&format!(
            "\n<div style=\"margin-left: 1cm; margin-right: 1cm; margin-bottom: 0.5cm\">\n\n**Abstract**\n\n{a}\n\n</div>\n\n"
        ));
    }
    if toc {
        extra.push_str("\n[TOC]\n\n");
    }
    if title_end.is_none() {
        out.push_str(&extra);
    }
    let mut last = body;
    for ((p, l), n) in heads.iter().zip(nums) {
        out.push_str(&text[last..*p]);
        let hashes = *l as usize;
        out.push_str(&text[*p..*p + hashes]);
        match n {
            Some(n) => out.push_str(&format!(
                " {n} {}",
                text[*p + hashes..].trim_start_matches([' ', '\t']).split_inclusive('\n').next().unwrap_or("")
            )),
            None => out.push_str(text[*p + hashes..].split_inclusive('\n').next().unwrap_or("")),
        }
        last = text[*p..].find('\n').map(|e| p + e + 1).unwrap_or(text.len());
        if Some(last) == title_end {
            out.push_str(&extra);
        }
    }
    out.push_str(&text[last.min(text.len())..]);
    Cow::Owned(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn numbers_headings_like_pandoc_with_a_title() {
        assert_eq!(numbers(&[1, 2, 2, 3, 2]), vec![None, Some("1".into()), Some("2".into()), Some("2.1".into()), Some("3".into())]);
        assert_eq!(numbers(&[1, 2, 1, 2]), vec![Some("1".into()), Some("1.1".into()), Some("2".into()), Some("2.1".into())]);
        assert_eq!(list("[Ann, \"Bo B\"]"), vec!["Ann".to_string(), "Bo B".to_string()]);
    }

    #[test]
    fn adds_toc_abstract_and_numbers() {
        let text =
            "---\ntoc: true\nnumbersections: true\nabstract: In short.\n---\n# Title\n\n## Intro\n\n```\n# not a heading\n```\n\n## Next\n";
        let mut p = crate::md::parse(text);
        apply(&mut p);
        assert!(matches!(p.blocks[1], Block::Styled { .. }));
        assert!(matches!(p.blocks[2], Block::Toc { .. }));
        assert!(matches!(&p.blocks[3], Block::Heading { inlines, .. } if matches!(&inlines[0], Inline::Text(t, _) if t.starts_with("1"))));
        let t = expand_text(text);
        assert!(t.contains("## 1 Intro") && t.contains("## 2 Next") && t.contains("# not a heading"), "{t}");
        assert!(t.find("**Abstract**").unwrap() < t.find("[TOC]").unwrap() && t.find("[TOC]").unwrap() < t.find("## 1").unwrap());
    }
}
