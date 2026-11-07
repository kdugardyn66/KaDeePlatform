//! A document as one HTML page: for File › Export as web page, Print and Export as PDF (the
//! page is printed by the browser). MdCraft's document styles, the theme's colours and font,
//! diagrams and charts as SVG, math as Unicode, `[TOC]`, page breaks, cover pages, watermarks
//! and slide decks (one 16:9 frame per slide, with its notes).

use std::fmt::Write as _;

use comrak::nodes::{NodeHtmlBlock, NodeValue};
use comrak::{Anchorizer, Arena, parse_document};

use super::{diagram, edit, library, math, media, parse, slides};

const MARKDOWN_CSS: &str = include_str!("../../assets/markdown.css");

const EXTRA_CSS: &str = r#"
body { margin: 0; background: var(--page-bg, #f3f4f6); }
.markdown-body { box-sizing: border-box; padding: 48px 56px 64px; margin: 24px auto; box-shadow: 0 1px 6px rgba(0,0,0,.12); }
.markdown-body h1, .markdown-body h2, .markdown-body h3 { color: var(--heading, inherit); }
.markdown-body .page-break { border-top: 1px dashed var(--border); margin: 32px 0; }
.markdown-body .cover { min-height: 60vh; display: flex; flex-direction: column; justify-content: center; }
.markdown-body .cover h1 { font-size: 2.6em; border: 0; }
.markdown-body .toc ul { list-style: none; padding-left: 1.2em; }
.markdown-body .toc > ul { padding-left: 0; }
.markdown-body .math-display { text-align: center; margin: 0 0 16px; font-size: 1.1em; }
.markdown-body svg { max-width: 100%; height: auto; display: block; margin: 0 auto 16px; }
.markdown-body .cols { display: flex; gap: 24px; } .markdown-body .cols > div { flex: 1; }
.markdown-body .kpis { display: flex; flex-wrap: wrap; gap: 12px; list-style: none; padding: 0; }
.markdown-body .kpis li { flex: 1 1 160px; border: 1px solid var(--border); border-radius: 8px; padding: 10px 14px; }
.markdown-body .badge { display: inline-block; padding: 1px 8px; border-radius: 10px; font-size: .85em; background: #e5e7eb; }
.markdown-body .badge.green { background: #dcfce7; } .markdown-body .badge.amber { background: #fef3c7; } .markdown-body .badge.red { background: #fee2e2; }
.markdown-body .right { text-align: right; } .markdown-body .center { text-align: center; } .markdown-body .muted { color: var(--fg-muted); } .markdown-body .small { font-size: .85em; }
.markdown-body video, .markdown-body iframe { max-width: 100%; }
.markdown-body video.media { display: block; width: 100%; max-width: 720px; margin: 0 auto 16px; border-radius: 8px; background: #000; }
.markdown-body audio.media { display: block; width: 100%; max-width: 560px; margin: 0 auto 16px; }
.markdown-body .media.online { display: block; position: relative; width: 100%; max-width: 720px; aspect-ratio: 16 / 9; margin: 0 auto 16px; }
.markdown-body .media.online iframe { position: absolute; inset: 0; width: 100%; height: 100%; border: 0; border-radius: 8px; }
.markdown-body a.file-link { display: inline-flex; align-items: center; gap: 12px; padding: 10px 16px 10px 10px; border: 1px solid var(--border); border-radius: 8px; text-decoration: none; color: inherit; }
.markdown-body a.file-link:hover { background: rgba(127,127,127,.08); }
.markdown-body a.file-link small { color: var(--fg-muted); }
.markdown-body .file-icon { display: inline-flex; align-items: flex-end; justify-content: center; width: 34px; height: 42px; padding-bottom: 6px; box-sizing: border-box; border-radius: 4px; background: #2f5dd6; color: #fff; font: 700 9px/1 sans-serif; clip-path: polygon(0 0, 70% 0, 100% 22%, 100% 100%, 0 100%); }
@media print { .markdown-body .media.online, .markdown-body video.media, .markdown-body audio.media { display: none; } }
.watermark { position: fixed; inset: 0; display: flex; align-items: center; justify-content: center; pointer-events: none; font: 700 120px/1 sans-serif; color: rgba(160,160,160,.18); transform: rotate(-30deg); z-index: 5; }
.deck section.slide { box-sizing: border-box; width: min(960px, 100%); aspect-ratio: 16 / 9; margin: 24px auto 8px; padding: 40px 56px; border-radius: 6px; box-shadow: 0 1px 6px rgba(0,0,0,.15); display: flex; flex-direction: column; justify-content: center; overflow: hidden; }
.deck section.slide.title, .deck section.slide.section, .deck section.slide.end, .deck section.slide.stat, .deck section.slide.quote { text-align: center; }
.deck section.slide h1 { font-size: 2.4em; border: 0; } .deck section.slide h2 { border: 0; }
.deck aside.notes { width: min(960px, 100%); margin: 0 auto 24px; color: var(--fg-muted); font-size: 14px; }
@media print {
  body { background: none; }
  .markdown-body { box-shadow: none; margin: 0; max-width: none; padding: 0; }
  .markdown-body .page-break { border: 0; margin: 0; break-after: page; }
  .deck section.slide { box-shadow: none; break-after: page; margin: 0 auto; }
  .deck aside.notes { display: none; }
}
"#;

#[derive(Debug, Clone, Default)]
pub struct HtmlOptions {
    /// Dark page colours.
    pub dark: bool,
    /// Open the print window when the page has loaded.
    pub print: bool,
}

fn esc(s: &str) -> String {
    s.replace('&', "&amp;").replace('<', "&lt;").replace('>', "&gt;").replace('"', "&quot;")
}

/// The Markdown → HTML of one part of a document (no front matter).
fn fragment(md: &str, opts: &HtmlOptions) -> String {
    // page breaks and the table of contents
    let mut src = String::with_capacity(md.len());
    for line in md.split_inclusive('\n') {
        let t = line.trim();
        if t.starts_with("<!--") && t.contains("pagebreak") {
            src.push_str("<div class=\"page-break\"></div>\n");
        } else if t.starts_with("![") && t.ends_with('}') && t.contains("){") {
            // `![…](…){width=640}`: drop the attributes
            let cut = t.rfind("){").map(|p| p + 1).unwrap_or(t.len());
            src.push_str(&t[..cut]);
            src.push('\n');
        } else if let Some(path) = media::import_path(t) {
            // an included file: its icon and name, linked to it
            let _ = writeln!(
                src,
                "<p class=\"include\"><a class=\"file-link\" href=\"{}\"><span class=\"file-icon\">{}</span><span><b>{}</b><br><small>{}</small></span></a></p>\n",
                esc(path),
                esc(&media::extension(path).to_uppercase()),
                esc(&media::file_name(path)),
                media::file_kind(path)
            );
        } else {
            src.push_str(line);
        }
    }
    let mut o = parse::options();
    o.extension.header_id_prefix = Some(String::new());
    o.render.r#unsafe = true;
    let arena = Arena::new();
    let root = parse_document(&arena, &src, &o);
    for node in root.descendants().collect::<Vec<_>>() {
        let mut data = node.data.borrow_mut();
        let replacement = match &data.value {
            NodeValue::CodeBlock(cb) => diagram::render(&cb.info, &cb.literal, opts.dark).map(|svg| {
                NodeValue::HtmlBlock(NodeHtmlBlock { block_type: 6, literal: format!("<figure class=\"diagram\">{}</figure>\n", svg.svg) })
            }),
            NodeValue::Image(link) => media::kind(&link.url).map(|m| {
                let url = link.url.trim_start_matches('<').trim_end_matches('>').to_string();
                NodeValue::HtmlInline(match media::embed_url(&m) {
                    Some(embed) => format!(
                        "<span class=\"media video online\"><iframe src=\"{}\" title=\"{}\" allow=\"accelerometer; autoplay; clipboard-write; encrypted-media; picture-in-picture\" allowfullscreen></iframe></span>",
                        esc(&embed),
                        esc(&link.title)
                    ),
                    None if m.is_audio() => format!("<audio class=\"media audio\" controls preload=\"metadata\" src=\"{}\"></audio>", esc(&url)),
                    None => format!("<video class=\"media video\" controls preload=\"metadata\" src=\"{}\"></video>", esc(&url)),
                })
            }),
            NodeValue::Math(m) => {
                let u = esc(&math::to_unicode(&m.literal));
                Some(if m.display_math {
                    NodeValue::HtmlInline(format!("<span class=\"math-display\" style=\"display:block\">{u}</span>"))
                } else {
                    NodeValue::HtmlInline(format!("<span class=\"math\">{u}</span>"))
                })
            }
            _ => None,
        };
        if let Some(r) = replacement {
            let media = matches!(data.value, NodeValue::Image(_));
            data.value = r;
            drop(data);
            if media {
                for c in node.children().collect::<Vec<_>>() {
                    c.detach();
                }
            }
        }
    }
    let mut html = String::new();
    let _ = comrak::format_html(root, &o, &mut html);
    if html.contains("<p>[TOC]</p>") {
        html = html.replace("<p>[TOC]</p>", &toc(md));
    }
    html
}

/// The anchor of a heading, as GitHub (and comrak) make it: `Next steps!` → `next-steps`.
pub fn anchor(text: &str) -> String {
    Anchorizer::new().anchorize(text)
}

/// `[TOC]`: the headings (levels 1–3) as links.
fn toc(md: &str) -> String {
    let mut a = Anchorizer::new();
    let mut out = String::from("<nav class=\"toc\"><ul>");
    for h in parse::outline(&parse::parse(md)) {
        let id = a.anchorize(&h.text);
        if h.level <= 3 {
            let _ = write!(out, "<li style=\"margin-left:{}em\"><a href=\"#{id}\">{}</a></li>", (h.level - 1) as f32 * 1.2, esc(&h.text));
        }
    }
    out.push_str("</ul></nav>");
    out
}

/// The whole page.
pub fn to_html(text: &str, opts: &HtmlOptions) -> String {
    // comments are for the authors, not the readers
    let text = &*super::comments::strip(text);
    let fm = edit::front_matter(text);
    let get = |k: &str| fm.iter().find(|(key, _)| key == k).map(|(_, v)| v.clone());
    let parsed = parse::parse(text);
    let title = get("title").or_else(|| parse::outline(&parsed).first().map(|h| h.text.clone())).unwrap_or_else(|| "Document".into());
    let theme = get("theme").as_deref().and_then(library::theme);
    let all = slides::slides(text);
    let deck = !all.is_empty();

    let mut vars = String::new();
    if let Some(t) = theme {
        let _ = write!(vars, "--heading: #{:06x};", if t.deck { t.colors[1] } else { t.colors[0] });
    }
    let font = get("mainfont").or_else(|| theme.map(|t| t.font.to_string()));
    let font_css = font.map(|f| format!(".markdown-body {{ font-family: \"{f}\", system-ui, sans-serif; }}")).unwrap_or_default();
    let size_css = get("fontsize").map(|s| format!(".markdown-body {{ font-size: {s}; }}")).unwrap_or_default();
    let mono_css = get("monofont")
        .map(|f| format!(".markdown-body code, .markdown-body pre {{ font-family: \"{f}\", monospace; }}"))
        .unwrap_or_default();
    let justify = if get("justify").as_deref() == Some("true") { ".markdown-body p { text-align: justify; }" } else { "" };
    let paper = match (get("paper").as_deref(), get("orientation").as_deref()) {
        (Some("letter"), Some("landscape")) => "letter landscape",
        (Some("letter"), _) => "letter",
        (Some("legal"), _) => "legal",
        (Some("a5"), Some("landscape")) => "A5 landscape",
        (Some("a5"), _) => "A5",
        (_, Some("landscape")) => "A4 landscape",
        _ => "A4",
    };
    let margin = match get("margins").as_deref() {
        Some("narrow") => "1.27cm",
        Some("wide") => "3.5cm",
        _ => "2.5cm",
    };
    let page_css =
        if deck { "@page { size: 297mm 167mm; margin: 0; }".to_string() } else { format!("@page {{ size: {paper}; margin: {margin}; }}") };

    let mut body = String::new();
    if deck {
        body.push_str("<div class=\"deck markdown-body\" style=\"max-width:none;box-shadow:none;background:none;padding:0\">");
        let deck_theme = theme.filter(|t| t.deck);
        for s in &all {
            let (content, notes) = s.content_and_notes(text);
            let bg = s
                .attr("data-background-color")
                .map(str::to_string)
                .or_else(|| deck_theme.map(|t| format!("#{:06x}", t.colors[0])))
                .unwrap_or_else(|| if opts.dark { "#0d1117".into() } else { "#ffffff".into() });
            let fg = deck_theme.map(|t| format!("color:#{:06x};", t.colors[2])).unwrap_or_default();
            let _ = write!(
                body,
                "<section class=\"slide {}\" style=\"background:{bg};{fg}\">{}</section>",
                esc(s.class()),
                fragment(content, opts)
            );
            if let Some(n) = notes {
                let _ = write!(body, "<aside class=\"notes\"><b>Notes:</b> {}</aside>", fragment(n, opts));
            }
        }
        body.push_str("</div>");
    } else {
        let start = edit::body_start(text);
        let _ = write!(body, "<article class=\"markdown-body\">{}</article>", fragment(&text[start..], opts));
    }
    let watermark =
        get("watermark").filter(|w| !w.is_empty()).map(|w| format!("<div class=\"watermark\">{}</div>", esc(&w))).unwrap_or_default();
    let print =
        if opts.print { "<script>window.addEventListener('load', () => setTimeout(() => window.print(), 300));</script>" } else { "" };
    let theme_attr = if opts.dark { " data-theme=\"dark\"" } else { "" };
    format!(
        "<!doctype html>\n<html lang=\"en\"{theme_attr}>\n<head>\n<meta charset=\"utf-8\">\n<meta name=\"viewport\" content=\"width=device-width, initial-scale=1\">\n<meta name=\"generator\" content=\"Kadee Markdown\">\n<title>{}</title>\n<style>\n{MARKDOWN_CSS}\n{EXTRA_CSS}\n:root {{ {vars} }}\n{font_css}\n{size_css}\n{mono_css}\n{justify}\n{page_css}\n</style>\n</head>\n<body>\n{watermark}{body}\n{print}\n</body>\n</html>\n",
        esc(&title)
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_document_becomes_a_page() {
        let md = "---\ntitle: Report\ntheme: report\nwatermark: DRAFT\n---\n\n# Intro\n\n[TOC]\n\n## Part\n\n$E = mc^2$\n\n<!-- pagebreak -->\n\n```mermaid\nflowchart LR\n  A --> B\n```\n";
        let h = to_html(md, &HtmlOptions::default());
        assert!(h.contains("<title>Report</title>"));
        assert!(h.contains("<nav class=\"toc\">") && h.contains("href=\"#part\""));
        assert!(h.contains("id=\"part\""));
        assert!(h.contains("mc²"));
        assert!(h.contains("class=\"page-break\""));
        assert!(h.contains("<svg"));
        assert!(h.contains("class=\"watermark\">DRAFT"));
        assert!(!h.contains("theme: report"));
    }

    #[test]
    fn a_deck_becomes_slides() {
        let md = "# One\n\n<!-- slide class=\"end\" -->\n\n## Two\n\nNote: hello\n";
        let h = to_html(md, &HtmlOptions { dark: false, print: true });
        assert_eq!(h.matches("<section class=\"slide").count(), 2);
        assert!(h.contains("slide end"));
        assert!(h.contains("<aside class=\"notes\">"));
        assert!(h.contains("window.print()"));
    }
}
