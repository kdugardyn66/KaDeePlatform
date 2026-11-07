//! A Markdown document saved in other formats: Word (.docx), OpenDocument text (.odt),
//! PowerPoint (.pptx), OpenDocument presentation (.odp) and PDF. Written directly (no office
//! suite needed) from the parsed document: headings, inline styles, links, lists and task
//! lists, quotes, callouts, code, tables, pictures, charts and diagrams (as pictures), math (as
//! Unicode text), page breaks, and for decks one slide per `<!-- slide -->` with its notes.

mod docx;
pub(crate) mod flow;
mod odp;
mod odt;
mod pdf;
mod pptx;
pub(crate) mod xml;

use std::io::{Cursor, Write};
use std::path::{Path, PathBuf};

use crate::md::{self, Inline, Parsed, Style, library::ThemeInfo};

/// The formats a document can be saved in.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Format {
    Docx,
    Odt,
    Pptx,
    Odp,
    Pdf,
}

impl Format {
    pub fn extension(self) -> &'static str {
        match self {
            Format::Docx => "docx",
            Format::Odt => "odt",
            Format::Pptx => "pptx",
            Format::Odp => "odp",
            Format::Pdf => "pdf",
        }
    }

    pub fn from_extension(ext: &str) -> Option<Format> {
        Some(match ext.to_ascii_lowercase().as_str() {
            "docx" => Format::Docx,
            "odt" => Format::Odt,
            "pptx" => Format::Pptx,
            "odp" => Format::Odp,
            "pdf" => Format::Pdf,
            _ => return None,
        })
    }
}

/// What an export needs besides the text.
#[derive(Debug, Clone, Default)]
pub struct Input {
    pub text: String,
    /// The document's folder, for pictures with relative paths.
    pub base_dir: Option<PathBuf>,
    /// Used when the front matter has no `author`.
    pub author: String,
}

/// The document in `format`.
pub fn export(format: Format, input: &Input) -> Result<Vec<u8>, String> {
    let doc = Doc::new(input);
    match format {
        Format::Docx => docx::write(&doc),
        Format::Odt => odt::write(&doc),
        Format::Pptx => pptx::write(&doc),
        Format::Odp => odp::write(&doc),
        Format::Pdf => pdf::write(&doc),
    }
}

/// The parsed document with what every writer looks up.
pub(crate) struct Doc {
    pub parsed: Parsed,
    pub title: String,
    pub author: String,
    pub base_dir: Option<PathBuf>,
    pub theme: Option<&'static ThemeInfo>,
    /// The slides, when the document is a deck.
    pub slides: Vec<SlideDoc>,
    pub front: Vec<(String, String)>,
}

pub(crate) struct SlideDoc {
    pub parsed: Parsed,
    pub notes: Option<String>,
    pub class: String,
    pub background: Option<String>,
}

impl Doc {
    fn new(input: &Input) -> Doc {
        let text = &input.text;
        let mut parsed = md::parse(text);
        // videos, audio and included files: links with their name
        md::media::as_links(&mut parsed.blocks);
        // numbered headings, the table of contents and the abstract
        md::props::apply(&mut parsed);
        let front = parsed.front_matter.clone();
        let get = |k: &str| front.iter().find(|(key, _)| key == k).map(|(_, v)| v.clone());
        let title = get("title").or_else(|| md::outline(&parsed).first().map(|h| h.text.clone())).unwrap_or_else(|| "Document".into());
        let author =
            get("author").map(|a| md::props::list(&a).join(", ")).filter(|a| !a.is_empty()).unwrap_or_else(|| input.author.clone());
        let theme = get("theme").as_deref().and_then(md::library::theme);
        let slides = md::slides::slides(text)
            .iter()
            .map(|s| {
                let (content, notes) = s.content_and_notes(text);
                let mut parsed = md::parse(content);
                md::media::as_links(&mut parsed.blocks);
                SlideDoc {
                    parsed,
                    notes: notes.map(str::to_string),
                    class: s.class().to_string(),
                    background: s.attr("data-background-color").map(str::to_string),
                }
            })
            .collect();
        Doc { parsed, title, author, base_dir: input.base_dir.clone(), theme, slides, front }
    }

    /// The keywords (`keywords: [a, b]`), comma-separated.
    pub fn keywords(&self) -> String {
        self.meta("keywords").map(|k| md::props::list(k).join(", ")).unwrap_or_default()
    }

    /// How deep the table of contents goes.
    pub fn toc_depth(&self) -> u8 {
        md::props::toc_depth(&self.front)
    }

    /// The headings' font (`sansfont`), when it is set.
    pub fn heading_font(&self) -> Option<&str> {
        self.meta("sansfont").filter(|f| !f.trim().is_empty())
    }

    pub fn meta(&self, key: &str) -> Option<&str> {
        self.front.iter().find(|(k, _)| k == key).map(|(_, v)| v.as_str())
    }

    /// The page setup: paper, orientation and margins (`geometry`, or the older keys).
    pub fn page(&self) -> md::page::PageSetup {
        md::page::PageSetup::from_front(&|k| self.meta(k).map(str::to_string))
    }

    /// Page size in points (width, height).
    pub fn page_size(&self) -> (f32, f32) {
        self.page().size()
    }

    /// Page margins in points.
    pub fn margins(&self) -> md::page::Margins {
        self.page().margins
    }

    /// The colour of headings (the document theme's), as `RRGGBB`.
    pub fn heading_color(&self) -> String {
        match self.theme {
            Some(t) if !t.deck && t.id != "github" => format!("{:06X}", t.colors[0]),
            _ => "1F2328".into(),
        }
    }

    /// The body font family.
    pub fn font(&self) -> String {
        self.meta("mainfont")
            .map(str::to_string)
            .or_else(|| self.theme.map(|t| t.font.to_string()))
            .filter(|f| f != "system-ui")
            .unwrap_or_else(|| "Calibri".into())
    }

    /// The body size in points.
    pub fn font_size(&self) -> f32 {
        self.meta("fontsize").and_then(md::style::points).unwrap_or(11.0)
    }

    /// A picture from a link (local file) or a drawn diagram.
    pub fn picture(&self, url: &str) -> Option<Picture> {
        let url = url.trim_start_matches('<').trim_end_matches('>');
        if url.starts_with("http://") || url.starts_with("https://") {
            return Picture::from_web(url);
        }
        if url.contains("://") || url.starts_with("data:") {
            return None;
        }
        let url = url.trim_start_matches('<').trim_end_matches('>');
        let url = url.split(['#', '?']).next().unwrap_or(url);
        let p = Path::new(url);
        let p = if p.is_absolute() { p.to_path_buf() } else { self.base_dir.as_ref()?.join(p) };
        Picture::from_file(&p)
    }

    /// A slide deck's colours: (background, text, headings) as `RRGGBB`.
    pub fn slide_colors(&self, s: &SlideDoc) -> (String, String, String) {
        let hex = |c: u32| format!("{c:06X}");
        let lum = |c: u32| {
            let (r, g, b) = ((c >> 16) & 255, (c >> 8) & 255, c & 255);
            (0.299 * r as f32 + 0.587 * g as f32 + 0.114 * b as f32) / 255.0
        };
        let (mut bg, mut head) = match self.theme.filter(|t| t.deck) {
            Some(t) => {
                let bg = t.colors[0];
                let head = if (lum(t.colors[1]) - lum(bg)).abs() >= (lum(t.colors[2]) - lum(bg)).abs() { t.colors[1] } else { t.colors[2] };
                (bg, head)
            }
            None => (0xFFFFFF, 0x1F2328),
        };
        if s.class == "section"
            && let Some(t) = self.theme.filter(|t| t.deck)
        {
            bg = t.colors[1];
            head = if lum(bg) < 0.5 { 0xFFFFFF } else { 0x111111 };
        }
        if let Some(c) = s.background.as_deref().and_then(parse_hex) {
            bg = c;
        }
        let fg = if lum(bg) < 0.5 { 0xF3F4F6 } else { 0x1F2328 };
        if (lum(head) - lum(bg)).abs() < 0.3 {
            head = fg;
        }
        (hex(bg), hex(fg), hex(head))
    }
}

/// `#rgb` or `#rrggbb`.
pub(crate) fn parse_hex(c: &str) -> Option<u32> {
    let h = c.trim().strip_prefix('#')?;
    match h.len() {
        6 => u32::from_str_radix(h, 16).ok(),
        3 => {
            let n = u32::from_str_radix(h, 16).ok()?;
            Some((((n >> 8) & 0xF) * 0x11) << 16 | (((n >> 4) & 0xF) * 0x11) << 8 | ((n & 0xF) * 0x11))
        }
        _ => None,
    }
}

/// A picture to embed: PNG or JPEG bytes and its size in pixels.
#[derive(Debug, Clone)]
pub(crate) struct Picture {
    pub data: Vec<u8>,
    /// "png" or "jpeg".
    pub ext: &'static str,
    pub width: u32,
    pub height: u32,
}

impl Picture {
    /// A picture on the web (fetched, with a time limit).
    fn from_web(url: &str) -> Option<Picture> {
        let agent: ureq::Agent = ureq::Agent::config_builder().timeout_global(Some(std::time::Duration::from_secs(10))).build().into();
        let mut resp = agent.get(url).call().ok()?;
        let data = resp.body_mut().with_config().limit(25 * 1024 * 1024).read_to_vec().ok()?;
        let path = url.split(['?', '#']).next().unwrap_or(url);
        let ext = path.rsplit_once('.').map(|(_, e)| e.to_ascii_lowercase()).filter(|e| e.len() <= 4).unwrap_or_default();
        Self::from_bytes(data, &ext)
    }

    fn from_file(p: &Path) -> Option<Picture> {
        let ext = p.extension()?.to_str()?.to_ascii_lowercase();
        let data = std::fs::read(p).ok()?;
        Self::from_bytes(data, &ext)
    }

    fn from_bytes(data: Vec<u8>, ext: &str) -> Option<Picture> {
        let ext = if ext.is_empty() || !matches!(ext, "png" | "jpg" | "jpeg" | "svg") {
            match image::guess_format(&data) {
                Ok(image::ImageFormat::Png) => "png".to_string(),
                Ok(image::ImageFormat::Jpeg) => "jpg".to_string(),
                _ if data.starts_with(b"<svg") || data.starts_with(b"<?xml") => "svg".to_string(),
                _ => ext.to_string(),
            }
        } else {
            ext.to_string()
        };
        if ext == "svg" {
            return svg_png(std::str::from_utf8(&data).ok()?, 2.0);
        }
        let img = image::load_from_memory(&data).ok()?;
        let (width, height) = (img.width(), img.height());
        match ext.as_str() {
            "png" => Some(Picture { data, ext: "png", width, height }),
            "jpg" | "jpeg" => Some(Picture { data, ext: "jpeg", width, height }),
            _ => {
                let mut out = Cursor::new(Vec::new());
                img.write_to(&mut out, image::ImageFormat::Png).ok()?;
                Some(Picture { data: out.into_inner(), ext: "png", width, height })
            }
        }
    }

    /// A chart or diagram as a picture (drawn at twice its size; `width` / `height` are its
    /// size in CSS pixels).
    pub fn diagram(info: &str, code: &str) -> Option<Picture> {
        let svg = md::diagram::render(info, code, false)?;
        let mut p = svg_png(&svg.svg, 2.0)?;
        p.width = svg.width as u32;
        p.height = svg.height as u32;
        Some(p)
    }

    /// Size in points for `max_w` points wide at most (96 pixels per inch).
    pub fn points(&self, max_w: f32) -> (f32, f32) {
        let (w, h) = (self.width as f32 * 0.75, self.height as f32 * 0.75);
        if w > max_w { (max_w, h * max_w / w) } else { (w, h) }
    }
}

/// An SVG drawn as a PNG picture at `scale` × its size.
pub(crate) fn svg_png(svg: &str, scale: f32) -> Option<Picture> {
    use std::sync::{Arc, LazyLock};
    static FONTS: LazyLock<Arc<resvg::usvg::fontdb::Database>> = LazyLock::new(|| {
        let mut db = resvg::usvg::fontdb::Database::new();
        db.load_system_fonts();
        Arc::new(db)
    });
    let opts = resvg::usvg::Options { fontdb: FONTS.clone(), ..Default::default() };
    let tree = resvg::usvg::Tree::from_str(svg, &opts).ok()?;
    let size = tree.size();
    let (w, h) = ((size.width() * scale).ceil() as u32, (size.height() * scale).ceil() as u32);
    let mut pixmap = resvg::tiny_skia::Pixmap::new(w.max(1), h.max(1))?;
    pixmap.fill(resvg::tiny_skia::Color::WHITE);
    resvg::render(&tree, resvg::tiny_skia::Transform::from_scale(scale, scale), &mut pixmap.as_mut());
    let data = pixmap.encode_png().ok()?;
    Some(Picture { data, ext: "png", width: (size.width()) as u32, height: (size.height()) as u32 })
}

/// One piece of inline content, flattened for the writers.
#[derive(Debug, Clone)]
pub(crate) enum Run {
    Text(String, Style),
    Break,
    /// A picture inside a paragraph (alt text when it cannot be embedded).
    Image(String, String),
}

/// Inline content as runs: math as Unicode italics, footnote references as `[n]`, emoji
/// shortcodes already resolved by the parser.
pub(crate) fn runs(inlines: &[Inline]) -> Vec<Run> {
    let mut out = Vec::new();
    for i in inlines {
        match i {
            Inline::Text(t, s) => out.push(Run::Text(t.clone(), s.clone())),
            Inline::Break => out.push(Run::Break),
            Inline::Image { url, alt, .. } => out.push(Run::Image(url.clone(), alt.clone())),
            Inline::Math { text, display } => {
                let s = Style { italic: true, ..Style::default() };
                if *display {
                    out.push(Run::Break);
                }
                out.push(Run::Text(md::math::to_unicode(text), s));
                if *display {
                    out.push(Run::Break);
                }
            }
            Inline::FootnoteRef(l) => out.push(Run::Text(format!("[{l}]"), Style { sup: true, ..Style::default() })),
        }
    }
    out
}

/// Plain text of inlines (titles of slides, alt texts).
pub(crate) fn plain(inlines: &[Inline]) -> String {
    md::inline_text(inlines)
}

/// A zip package; `stored` files (ODF's `mimetype`) are not compressed.
pub(crate) fn package(files: &[(String, Vec<u8>)], stored: &[&str]) -> Result<Vec<u8>, String> {
    let mut zip = zip::ZipWriter::new(Cursor::new(Vec::new()));
    for (name, data) in files {
        let method = if stored.contains(&name.as_str()) { zip::CompressionMethod::Stored } else { zip::CompressionMethod::Deflated };
        let opts = zip::write::SimpleFileOptions::default().compression_method(method);
        zip.start_file(name.as_str(), opts).map_err(|e| e.to_string())?;
        zip.write_all(data).map_err(|e| e.to_string())?;
    }
    zip.finish().map(|c| c.into_inner()).map_err(|e| e.to_string())
}

/// The code blocks drawn as pictures.
pub(crate) fn is_drawn(info: &str, text: &str) -> Option<Picture> {
    Picture::diagram(info, text)
}

/// Today's date for document properties (`2026-11-05T00:00:00Z`).
pub(crate) fn now_iso() -> String {
    chrono::Utc::now().format("%Y-%m-%dT%H:%M:%SZ").to_string()
}

#[cfg(test)]
pub(crate) mod tests {
    use super::*;

    /// A document with everything the writers handle.
    pub const SAMPLE: &str = "---\ntitle: Sample\nauthor: Ada\ntheme: report\n---\n\n# Sample report\n\nSome **bold**, *italic*, `code`, ~~gone~~, ==marked==, H~2~O, x^2^, <span style=\"color: #C00000\">red</span> and a [link](https://example.com). Math $E = mc^2$.[^1]\n\n## Lists\n\n- one\n- two\n  - nested\n\n1. first\n2. second\n\n- [x] done\n- [ ] todo\n\n> A quote.\n\n> [!TIP]\n> A tip.\n\n```rust\nfn main() {}\n```\n\n| A | B |\n|:--|--:|\n| 1 | 2 |\n\n<!-- pagebreak -->\n\n```mermaid\nflowchart LR\n  A --> B\n```\n\n---\n\n[^1]: A footnote.\n";

    pub const DECK: &str = "---\ntheme: aurora\n---\n\n<!-- slide class=\"title\" -->\n\n# Deck\n\nSubtitle\n\nNote: hello\n\n<!-- slide -->\n\n## Points\n\n- one\n- two\n\n<!-- slide data-background-color=\"#ffffff\" -->\n\n## Table\n\n| A | B |\n|---|---|\n| 1 | 2 |\n";

    pub fn input(text: &str) -> Input {
        Input { text: text.into(), base_dir: None, author: "Kris".into() }
    }

    #[test]
    fn every_format_writes_something() {
        for f in [Format::Docx, Format::Odt, Format::Pptx, Format::Odp, Format::Pdf] {
            for t in [SAMPLE, DECK] {
                let bytes = export(f, &input(t)).unwrap_or_else(|e| panic!("{f:?}: {e}"));
                assert!(bytes.len() > 500, "{f:?}");
            }
        }
    }

    #[test]
    fn diagrams_become_pictures() {
        let p = Picture::diagram("mermaid", "flowchart LR\n  A --> B").expect("picture");
        assert_eq!(&p.data[1..4], b"PNG");
        assert!(p.width > 50);
    }
}
