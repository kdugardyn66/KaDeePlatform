//! Markdown → blocks for the native preview, with MdCraft's dialect: GitHub Flavored Markdown
//! (tables, task lists, strikethrough, autolinks, footnotes), front matter, alerts
//! (`> [!NOTE]`), `$math$`, `==highlight==`, `^sup^`, `~sub~`, `:emoji:` shortcodes and
//! MdCraft's markers (`<!-- pagebreak -->`, `<!-- slide -->`).

use comrak::nodes::{AlertType, AstNode, ListType, NodeValue, TableAlignment};
use comrak::{Arena, Options, parse_document};

/// Inline style flags.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Style {
    pub bold: bool,
    pub italic: bool,
    pub strike: bool,
    pub underline: bool,
    pub highlight: bool,
    pub code: bool,
    pub sup: bool,
    pub sub: bool,
    pub link: Option<String>,
    /// From `<span style="…">`: `color`, `background-color`, `font-size` and `font-family`.
    pub color: Option<String>,
    pub background: Option<String>,
    pub size: Option<String>,
    pub font: Option<String>,
}

#[derive(Debug, Clone, PartialEq)]
pub enum Inline {
    Text(String, Style),
    Image {
        url: String,
        alt: String,
        title: String,
    },
    /// `$…$`, or `$$…$$` when `display`.
    Math {
        text: String,
        display: bool,
    },
    FootnoteRef(String),
    Break,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Align {
    None,
    Left,
    Center,
    Right,
}

#[derive(Debug, Clone, PartialEq)]
pub struct ListItem {
    /// `Some(checked)` for a task.
    pub task: Option<bool>,
    pub blocks: Vec<Block>,
}

/// One block of the document. `line` is the 1-based source line it starts at.
#[derive(Debug, Clone, PartialEq)]
pub enum Block {
    Heading {
        level: u8,
        inlines: Vec<Inline>,
        line: usize,
    },
    Paragraph {
        inlines: Vec<Inline>,
        line: usize,
    },
    List {
        ordered: bool,
        start: usize,
        tight: bool,
        items: Vec<ListItem>,
        line: usize,
    },
    Quote {
        blocks: Vec<Block>,
        line: usize,
    },
    /// `> [!NOTE]` …: kind is `NOTE`, `TIP`, `IMPORTANT`, `WARNING` or `CAUTION`.
    Callout {
        kind: String,
        title: Option<String>,
        blocks: Vec<Block>,
        line: usize,
    },
    Code {
        lang: String,
        info: String,
        text: String,
        line: usize,
    },
    Math {
        text: String,
        line: usize,
    },
    Table {
        aligns: Vec<Align>,
        header: Vec<Vec<Inline>>,
        rows: Vec<Vec<Vec<Inline>>>,
        line: usize,
    },
    Rule {
        line: usize,
    },
    PageBreak {
        line: usize,
    },
    SlideBreak {
        line: usize,
    },
    /// `[TOC]`: the table of contents.
    Toc {
        line: usize,
    },
    Html {
        text: String,
        line: usize,
    },
    Footnote {
        label: String,
        blocks: Vec<Block>,
        line: usize,
    },
    /// Paragraphs with their own formatting (`<div style="…">` … `</div>`).
    Styled {
        style: super::para::ParaStyle,
        blocks: Vec<Block>,
        line: usize,
    },
}

impl Block {
    pub fn line(&self) -> usize {
        match self {
            Block::Heading { line, .. }
            | Block::Paragraph { line, .. }
            | Block::List { line, .. }
            | Block::Quote { line, .. }
            | Block::Callout { line, .. }
            | Block::Code { line, .. }
            | Block::Math { line, .. }
            | Block::Table { line, .. }
            | Block::Rule { line }
            | Block::PageBreak { line }
            | Block::SlideBreak { line }
            | Block::Toc { line }
            | Block::Html { line, .. }
            | Block::Footnote { line, .. }
            | Block::Styled { line, .. } => *line,
        }
    }
}

/// A parsed document.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct Parsed {
    /// Front matter keys (`title`, `theme` …), in order.
    pub front_matter: Vec<(String, String)>,
    pub blocks: Vec<Block>,
}

impl Parsed {
    pub fn meta(&self, key: &str) -> Option<&str> {
        self.front_matter.iter().find(|(k, _)| k == key).map(|(_, v)| v.as_str())
    }
}

pub fn options() -> Options<'static> {
    let mut o = Options::default();
    let e = &mut o.extension;
    e.strikethrough = true;
    e.table = true;
    e.autolink = true;
    e.tasklist = true;
    e.footnotes = true;
    e.front_matter_delimiter = Some("---".into());
    e.alerts = true;
    e.math_dollars = true;
    e.shortcodes = true;
    e.highlight = true;
    e.superscript = true;
    e.subscript = true;
    o.parse.relaxed_tasklist_matching = true;
    o
}

/// Parse Markdown into blocks.
pub fn parse(text: &str) -> Parsed {
    let arena = Arena::new();
    let root = parse_document(&arena, text, &options());
    let mut out = Parsed::default();
    for child in root.children() {
        if let NodeValue::FrontMatter(fm) = &child.data.borrow().value {
            out.front_matter = front_matter_pairs(fm);
            continue;
        }
        if let Some(b) = block(child) {
            out.blocks.push(b);
        }
    }
    out.blocks = group_styled(std::mem::take(&mut out.blocks));
    out
}

/// `<div style="…">`, the blocks, `</div>` → one [`Block::Styled`] (nested ones too).
fn group_styled(blocks: Vec<Block>) -> Vec<Block> {
    // each open div: its style and line (none for the document), and its blocks so far
    type Open = (Option<(super::para::ParaStyle, usize)>, Vec<Block>);
    let mut stack: Vec<Open> = vec![(None, Vec::new())];
    for b in blocks {
        if let Block::Html { text, line } = &b {
            let t = text.trim();
            if let Some(style) = super::para::div_style(t) {
                stack.push((Some((style, *line)), Vec::new()));
                continue;
            }
            if t == "</div>" && stack.len() > 1 {
                let (head, inner) = stack.pop().expect("open div");
                let (style, line) = head.expect("a styled div");
                stack.last_mut().expect("root").1.push(Block::Styled { style, blocks: inner, line });
                continue;
            }
        }
        stack.last_mut().expect("root").1.push(b);
    }
    // a div left open: keep its blocks
    while stack.len() > 1 {
        let (head, inner) = stack.pop().expect("open div");
        let (style, line) = head.expect("a styled div");
        stack.last_mut().expect("root").1.push(Block::Styled { style, blocks: inner, line });
    }
    stack.pop().map(|(_, b)| b).unwrap_or_default()
}

/// `key: value` lines of a front matter block (with or without the `---` fences).
pub fn front_matter_pairs(fm: &str) -> Vec<(String, String)> {
    fm.lines()
        .filter(|l| !l.trim_start().starts_with("---") && !l.starts_with(' ') && !l.starts_with('\t'))
        .filter_map(|l| {
            let (k, v) = l.split_once(':')?;
            let k = k.trim();
            if k.is_empty() || !k.chars().all(|c| c.is_alphanumeric() || c == '_' || c == '-') {
                return None;
            }
            Some((k.to_string(), v.trim().trim_matches(|c| c == '"' || c == '\'').to_string()))
        })
        .collect()
}

fn blocks_of<'a>(node: &'a AstNode<'a>) -> Vec<Block> {
    node.children().filter_map(block).collect()
}

fn block<'a>(node: &'a AstNode<'a>) -> Option<Block> {
    let data = node.data.borrow();
    let line = data.sourcepos.start.line;
    Some(match &data.value {
        NodeValue::Heading(h) => Block::Heading { level: h.level, inlines: inlines(node), line },
        NodeValue::Paragraph => {
            let mut ins = inlines(node);
            // `![…](…){width=640}`: the attributes after a picture or video are not text
            if let [Inline::Image { .. }, Inline::Text(t, _)] = ins.as_slice()
                && t.trim().starts_with('{')
                && t.trim().ends_with('}')
            {
                ins.truncate(1);
            }
            if let [Inline::Text(t, s)] = ins.as_slice()
                && t.trim() == "[TOC]"
                && *s == Style::default()
            {
                return Some(Block::Toc { line });
            }
            Block::Paragraph { inlines: ins, line }
        }
        NodeValue::List(l) => Block::List {
            ordered: l.list_type == ListType::Ordered,
            start: l.start,
            tight: l.tight,
            items: node
                .children()
                .map(|item| {
                    let task = match &item.data.borrow().value {
                        NodeValue::TaskItem(t) => Some(t.symbol.is_some()),
                        _ => None,
                    };
                    ListItem { task, blocks: blocks_of(item) }
                })
                .collect(),
            line,
        },
        NodeValue::BlockQuote | NodeValue::MultilineBlockQuote(_) => Block::Quote { blocks: blocks_of(node), line },
        NodeValue::Alert(a) => Block::Callout {
            kind: match a.alert_type {
                AlertType::Note => "NOTE",
                AlertType::Tip => "TIP",
                AlertType::Important => "IMPORTANT",
                AlertType::Warning => "WARNING",
                AlertType::Caution => "CAUTION",
            }
            .to_string(),
            title: a.title.clone(),
            blocks: blocks_of(node),
            line,
        },
        NodeValue::CodeBlock(c) => {
            let lang = c.info.split(|ch: char| ch.is_whitespace() || ch == '{').next().unwrap_or("").to_string();
            if lang == "math" || lang == "latex" && c.info.contains("math") {
                Block::Math { text: c.literal.trim_end().to_string(), line }
            } else {
                Block::Code { lang, info: c.info.clone(), text: c.literal.trim_end_matches('\n').to_string(), line }
            }
        }
        NodeValue::HtmlBlock(h) => {
            let t = h.literal.trim();
            if is_marker(t, "pagebreak") {
                Block::PageBreak { line }
            } else if is_marker(t, "slide") {
                Block::SlideBreak { line }
            } else {
                Block::Html { text: h.literal.clone(), line }
            }
        }
        NodeValue::ThematicBreak => Block::Rule { line },
        NodeValue::Table(t) => {
            let aligns = t
                .alignments
                .iter()
                .map(|a| match a {
                    TableAlignment::Left => Align::Left,
                    TableAlignment::Center => Align::Center,
                    TableAlignment::Right => Align::Right,
                    TableAlignment::None => Align::None,
                })
                .collect();
            let mut header = Vec::new();
            let mut rows = Vec::new();
            for row in node.children() {
                let is_header = matches!(row.data.borrow().value, NodeValue::TableRow(true));
                let cells: Vec<Vec<Inline>> = row.children().map(inlines).collect();
                if is_header {
                    header = cells;
                } else {
                    rows.push(cells);
                }
            }
            Block::Table { aligns, header, rows, line }
        }
        NodeValue::FootnoteDefinition(f) => Block::Footnote { label: f.name.clone(), blocks: blocks_of(node), line },
        // a paragraph made only of $$…$$
        _ => return None,
    })
}

fn is_marker(html: &str, name: &str) -> bool {
    html.strip_prefix("<!--").is_some_and(|rest| rest.trim_start().starts_with(name))
}

/// The inline content of a block node.
pub fn inlines<'a>(node: &'a AstNode<'a>) -> Vec<Inline> {
    let mut out = Vec::new();
    collect_children(node, &Style::default(), &mut out);
    // display math alone in a paragraph stays inline here; the preview shows it on its own line
    merge_text(out)
}

fn collect<'a>(node: &'a AstNode<'a>, style: &Style, out: &mut Vec<Inline>) {
    let data = node.data.borrow();
    let mut s = style.clone();
    match &data.value {
        NodeValue::Text(t) => out.push(Inline::Text(t.to_string(), s)),
        NodeValue::SoftBreak => out.push(Inline::Text(" ".into(), s)),
        NodeValue::LineBreak => out.push(Inline::Break),
        NodeValue::Code(c) => {
            s.code = true;
            out.push(Inline::Text(c.literal.clone(), s));
        }
        NodeValue::Math(m) => out.push(Inline::Math { text: m.literal.clone(), display: m.display_math }),
        NodeValue::ShortCode(sc) => out.push(Inline::Text(sc.emoji.clone(), s)),
        NodeValue::FootnoteReference(f) => out.push(Inline::FootnoteRef(f.name.clone())),
        NodeValue::Image(l) => {
            let alt = plain_text(node);
            out.push(Inline::Image { url: l.url.clone(), alt, title: l.title.clone() });
        }
        NodeValue::HtmlInline(h) => {
            let tag = h.trim().to_ascii_lowercase();
            if tag.starts_with("<br") {
                out.push(Inline::Break);
            } else if let Some((src, audio)) = super::media::html_media(h) {
                // <video>, <audio> and video <iframe>s are media like `![](clip.mp4)`
                if audio || tag.starts_with("<video") || super::media::kind(&src).is_some() {
                    out.push(Inline::Image { url: src, alt: String::new(), title: String::new() });
                }
            }
            // <span style>, <u> … change the style of their siblings: see `collect_children`
        }
        other => {
            match other {
                NodeValue::Strong => s.bold = true,
                NodeValue::Emph => s.italic = true,
                NodeValue::Strikethrough => s.strike = true,
                NodeValue::Underline => s.underline = true,
                NodeValue::Highlight => s.highlight = true,
                NodeValue::Superscript => s.sup = true,
                NodeValue::Subscript => s.sub = true,
                NodeValue::Link(l) => s.link = Some(l.url.clone()),
                _ => {}
            }
            drop(data);
            collect_children(node, &s, out);
        }
    }
}

/// The children of `node`, with inline HTML tags (`<span style>`, `<u>`, `<b>`, `<i>`, `<mark>`,
/// `<sup>`, `<sub>`, `<s>`) styling the siblings between an opening and its closing tag.
fn collect_children<'a>(node: &'a AstNode<'a>, style: &Style, out: &mut Vec<Inline>) {
    let mut stack: Vec<Style> = vec![style.clone()];
    for c in node.children() {
        let html = match &c.data.borrow().value {
            NodeValue::HtmlInline(h) => Some(h.trim().to_string()),
            _ => None,
        };
        let cur = stack.last().cloned().unwrap_or_default();
        match html {
            Some(tag) if tag.starts_with("</") => {
                if stack.len() > 1 {
                    stack.pop();
                }
            }
            Some(tag) if super::media::html_media(&tag).is_some() => {
                // media tags are not styles: their closing tag must not pop one
                collect(c, &cur, out);
                if !tag.ends_with("/>") {
                    stack.push(cur);
                }
            }
            Some(tag) if tag.starts_with('<') && !tag.ends_with("/>") && !tag.starts_with("<br") && !tag.starts_with("<!--") => {
                stack.push(tag_style(&tag, cur));
            }
            Some(_) => collect(c, &cur, out),
            None => collect(c, &cur, out),
        }
    }
}

/// The style inside an opening tag.
fn tag_style(tag: &str, mut s: Style) -> Style {
    let name: String = tag[1..].chars().take_while(|c| c.is_alphanumeric()).collect::<String>().to_ascii_lowercase();
    match name.as_str() {
        "u" | "ins" => s.underline = true,
        "b" | "strong" => s.bold = true,
        "i" | "em" => s.italic = true,
        "s" | "del" | "strike" => s.strike = true,
        "mark" => s.highlight = true,
        "sup" => s.sup = true,
        "sub" => s.sub = true,
        "code" | "kbd" => s.code = true,
        _ => {}
    }
    if let Some(i) = tag.find("style=\"") {
        let rest = &tag[i + 7..];
        let style = &rest[..rest.find('"').unwrap_or(rest.len())];
        for d in style.split(';') {
            let Some((k, v)) = d.split_once(':') else { continue };
            let v = v.trim().trim_matches(['"', '\'']).to_string();
            match k.trim().to_ascii_lowercase().as_str() {
                "color" => s.color = Some(v),
                "background-color" | "background" => s.background = Some(v),
                "font-size" => s.size = Some(v),
                "font-family" => s.font = Some(v.split(',').next().unwrap_or("").trim().trim_matches(['"', '\'']).to_string()),
                _ => {}
            }
        }
    }
    s
}

/// Join neighbouring text with the same style.
fn merge_text(v: Vec<Inline>) -> Vec<Inline> {
    let mut out: Vec<Inline> = Vec::with_capacity(v.len());
    for i in v {
        if let (Some(Inline::Text(prev, ps)), Inline::Text(t, s)) = (out.last_mut(), &i)
            && ps == s
        {
            prev.push_str(t);
            continue;
        }
        out.push(i);
    }
    out
}

/// The text of a node without formatting.
fn plain_text<'a>(node: &'a AstNode<'a>) -> String {
    let mut s = String::new();
    for d in node.descendants() {
        match &d.data.borrow().value {
            NodeValue::Text(t) => s.push_str(t),
            NodeValue::Code(c) => s.push_str(&c.literal),
            NodeValue::SoftBreak | NodeValue::LineBreak => s.push(' '),
            _ => {}
        }
    }
    s
}

/// The plain text of inlines (for the outline and headings' ids).
pub fn inline_text(inlines: &[Inline]) -> String {
    inlines
        .iter()
        .map(|i| match i {
            Inline::Text(t, _) => t.as_str(),
            Inline::Math { text, .. } => text.as_str(),
            Inline::Image { alt, .. } => alt.as_str(),
            Inline::Break => " ",
            Inline::FootnoteRef(_) => "",
        })
        .collect()
}

/// A heading in the document's outline.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Heading {
    pub level: u8,
    pub text: String,
    /// 1-based source line.
    pub line: usize,
}

/// The headings, in order (Navigation pane, table of contents).
pub fn outline(parsed: &Parsed) -> Vec<Heading> {
    parsed
        .blocks
        .iter()
        .filter_map(|b| match b {
            Block::Heading { level, inlines, line } => Some(Heading { level: *level, text: inline_text(inlines), line: *line }),
            _ => None,
        })
        .collect()
}

/// Words in the document, as MdCraft's status bar counts them: the text people read, without
/// front matter, markup, URLs and code fences' info strings.
pub fn word_count(parsed: &Parsed) -> usize {
    fn words(s: &str) -> usize {
        s.split_whitespace().filter(|w| w.chars().any(char::is_alphanumeric)).count()
    }
    fn blocks(bs: &[Block]) -> usize {
        bs.iter()
            .map(|b| match b {
                Block::Heading { inlines, .. } | Block::Paragraph { inlines, .. } => words(&inline_text(inlines)),
                Block::List { items, .. } => items.iter().map(|i| blocks(&i.blocks)).sum(),
                Block::Quote { blocks: inner, .. }
                | Block::Callout { blocks: inner, .. }
                | Block::Footnote { blocks: inner, .. }
                | Block::Styled { blocks: inner, .. } => blocks(inner),
                Block::Code { text, .. } => words(text),
                Block::Table { header, rows, .. } => header.iter().chain(rows.iter().flatten()).map(|c| words(&inline_text(c))).sum(),
                _ => 0,
            })
            .sum()
    }
    blocks(&parsed.blocks)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn mdcraft_dialect() {
        let p = parse(
            "---\ntitle: Report\ntheme: office\n---\n\n# Title\n\nSome **bold** and *it* and ==hi== and `code`.\n\n- [x] done\n- [ ] todo\n\n> [!IMPORTANT]\n> Read this.\n\n| A | B |\n|:--|--:|\n| 1 | 2 |\n\n<!-- pagebreak -->\n\n```mermaid\nflowchart LR\n```\n\n[TOC]\n",
        );
        assert_eq!(p.meta("title"), Some("Report"));
        assert!(matches!(&p.blocks[0], Block::Heading { level: 1, line: 6, .. }));
        let Block::Paragraph { inlines, .. } = &p.blocks[1] else { panic!("{:?}", p.blocks[1]) };
        assert!(inlines.iter().any(|i| matches!(i, Inline::Text(t, s) if t == "bold" && s.bold)));
        assert!(inlines.iter().any(|i| matches!(i, Inline::Text(t, s) if t == "hi" && s.highlight)));
        assert!(inlines.iter().any(|i| matches!(i, Inline::Text(t, s) if t == "code" && s.code)));
        let Block::List { items, .. } = &p.blocks[2] else { panic!() };
        assert_eq!(items[0].task, Some(true));
        assert_eq!(items[1].task, Some(false));
        assert!(matches!(&p.blocks[3], Block::Callout { kind, .. } if kind == "IMPORTANT"));
        assert!(matches!(&p.blocks[4], Block::Table { aligns, .. } if aligns == &[Align::Left, Align::Right]));
        assert!(matches!(&p.blocks[5], Block::PageBreak { .. }));
        assert!(matches!(&p.blocks[6], Block::Code { lang, .. } if lang == "mermaid"));
        assert!(matches!(&p.blocks[7], Block::Toc { .. }));
    }

    #[test]
    fn outline_and_words() {
        let p = parse("# One\n\nTwo words.\n\n## Three\n\n- four five\n");
        let o = outline(&p);
        assert_eq!(o.len(), 2);
        assert_eq!(o[1], Heading { level: 2, text: "Three".into(), line: 5 });
        assert_eq!(word_count(&p), 6);
    }

    #[test]
    fn links_images_math_emoji() {
        let p = parse("See [site](https://x.y) ![alt](a.png \"T\") $x^2$ :rocket:\n");
        let Block::Paragraph { inlines, .. } = &p.blocks[0] else { panic!() };
        assert!(inlines.iter().any(|i| matches!(i, Inline::Text(t, s) if t == "site" && s.link.as_deref() == Some("https://x.y"))));
        assert!(inlines.iter().any(|i| matches!(i, Inline::Image { url, alt, .. } if url == "a.png" && alt == "alt")));
        // attributes after a picture are dropped
        let p = parse("![Clip](clip.mp4){width=640}\n");
        assert!(matches!(&p.blocks[0], Block::Paragraph { inlines, .. } if inlines.len() == 1));
        assert!(inlines.iter().any(|i| matches!(i, Inline::Math { text, display: false } if text == "x^2")));
        assert!(inlines.iter().any(|i| matches!(i, Inline::Text(t, _) if t.contains('🚀'))));
    }
}

#[cfg(test)]
mod html_style_tests {
    use super::*;

    #[test]
    fn spans_style_their_text() {
        let p = parse("a <span style=\"color: #C00000; font-size: 14pt\">red</span> <u>under</u> b");
        let Block::Paragraph { inlines, .. } = &p.blocks[0] else { panic!("paragraph") };
        let red =
            inlines.iter().find_map(|i| if let Inline::Text(t, s) = i { (t == "red").then_some(s.clone()) } else { None }).expect("red");
        assert_eq!(red.color.as_deref(), Some("#C00000"));
        assert_eq!(red.size.as_deref(), Some("14pt"));
        let u =
            inlines.iter().find_map(|i| if let Inline::Text(t, s) = i { (t == "under").then_some(s.clone()) } else { None }).expect("u");
        assert!(u.underline);
        let b =
            inlines.iter().find_map(|i| if let Inline::Text(t, s) = i { t.contains('b').then_some(s.clone()) } else { None }).expect("b");
        assert_eq!(b, Style::default());
    }
}
