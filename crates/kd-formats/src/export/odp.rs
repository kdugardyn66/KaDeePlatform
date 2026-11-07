//! OpenDocument presentation (.odp): the same slides as the PowerPoint export, with notes.

use std::fmt::Write as _;

use super::flow::{self, Item, Kind, Para, SLIDE_H, SLIDE_W, SlideBox};
use super::odt::{NS, TextStyles, manifest, meta, odf_text, picture_frame};
use super::xml::esc;
use super::{Doc, Picture, Run, package};
use crate::md::Style;

struct Ctx {
    text: TextStyles,
    paras: Vec<String>,
    pictures: Vec<(String, Vec<u8>)>,
    font: String,
    mono: String,
}

impl Ctx {
    /// An automatic paragraph style (alignment, indent, space before).
    fn para_style(&mut self, center: bool, indent_pt: f32, space_pt: f32) -> String {
        let key = format!("{center}-{indent_pt:.0}-{space_pt:.0}");
        if let Some(i) = self.paras.iter().position(|k| *k == key) {
            return format!("P{}", i + 1);
        }
        self.paras.push(key);
        format!("P{}", self.paras.len())
    }

    fn para_styles_xml(&self) -> String {
        let mut s = String::new();
        for (i, k) in self.paras.iter().enumerate() {
            let mut parts = k.split('-');
            let center = parts.next() == Some("true");
            let indent: f32 = parts.next().and_then(|v| v.parse().ok()).unwrap_or(0.0);
            let space: f32 = parts.next().and_then(|v| v.parse().ok()).unwrap_or(0.0);
            let _ = write!(
                s,
                "<style:style style:name=\"P{}\" style:family=\"paragraph\"><style:paragraph-properties fo:text-align=\"{}\" fo:margin-left=\"{indent}pt\" fo:text-indent=\"{}pt\" fo:margin-top=\"{space}pt\"/></style:style>",
                i + 1,
                if center { "center" } else { "start" },
                if indent > 0.0 { -indent.min(18.0) } else { 0.0 }
            );
        }
        s
    }

    fn picture(&mut self, p: &Picture, x: f32, y: f32, w: f32, h: f32) -> String {
        let n = self.pictures.len() + 1;
        let href = format!("Pictures/image{n}.{}", if p.ext == "jpeg" { "jpg" } else { "png" });
        self.pictures.push((href.clone(), p.data.clone()));
        let f = picture_frame(&format!("Picture {n}"), &href, w / 72.0 * 2.54, h / 72.0 * 2.54, "paragraph");
        f.replacen("<draw:frame ", &format!("<draw:frame draw:style-name=\"gr1\" svg:x=\"{x:.1}pt\" svg:y=\"{y:.1}pt\" "), 1)
    }

    fn runs(&mut self, rs: &[Run], size: f32, color: &str, extra: &Style) -> String {
        let mut out = String::new();
        for r in rs {
            match r {
                Run::Text(t, s) => {
                    let mut st = s.clone();
                    st.bold |= extra.bold;
                    st.italic |= extra.italic;
                    st.code |= extra.code;
                    if st.color.is_none() {
                        st.color = Some(format!("#{color}"));
                    }
                    if st.size.is_none() {
                        st.size = Some(format!("{size:.1}pt"));
                    }
                    if st.font.is_none() && !st.code {
                        st.font = Some(self.font.clone());
                    }
                    let mono = self.mono.clone();
                    let name = self.text.name(&st, &mono);
                    let body = odf_text(t);
                    match name {
                        Some(n) => {
                            let _ = write!(out, "<text:span text:style-name=\"{n}\">{body}</text:span>");
                        }
                        None => out.push_str(&body),
                    }
                }
                Run::Break => out.push_str("<text:line-break/>"),
                Run::Image(_, alt) => out.push_str(&odf_text(&format!("[{alt}]"))),
            }
        }
        out
    }

    #[allow(clippy::too_many_arguments)]
    fn text_box(
        &mut self,
        x: f32,
        y: f32,
        w: f32,
        h: f32,
        paras: &[Para],
        size: f32,
        center: bool,
        title: bool,
        fg: &str,
        head: &str,
    ) -> String {
        let mut body = String::new();
        for p in paras {
            let s = if title { size * 1.8 } else { size * flow::scale(p.kind) };
            let color = match p.kind {
                Kind::Heading(_) => head.to_string(),
                Kind::Callout(c) => format!("{c:06X}"),
                _ => fg.to_string(),
            };
            let extra = Style {
                bold: title || matches!(p.kind, Kind::Heading(_)),
                italic: p.kind == Kind::Quote,
                code: p.kind == Kind::Code,
                ..Style::default()
            };
            let mut rs = p.runs.clone();
            if let Some(b) = p.bullet.as_ref().filter(|b| !b.is_empty()) {
                rs.insert(0, Run::Text(format!("{b}  "), Style::default()));
            }
            let indent = p.level as f32 * s * 1.2 + if matches!(p.kind, Kind::Quote | Kind::Callout(_)) { s } else { 0.0 };
            let ps = self.para_style(center || p.centred(), indent, s * 0.3);
            let x = self.runs(&rs, s, &color, &extra);
            let _ = write!(body, "<text:p text:style-name=\"{ps}\">{x}</text:p>");
        }
        let anchor = if center { "middle" } else { "top" };
        let _ = anchor;
        format!(
            "<draw:frame draw:style-name=\"{}\" svg:x=\"{x:.1}pt\" svg:y=\"{y:.1}pt\" svg:width=\"{w:.1}pt\" svg:height=\"{h:.1}pt\"><draw:text-box>{body}</draw:text-box></draw:frame>",
            if center { "grc" } else { "gr1" }
        )
    }
}

/// A table drawn as SVG (an ODP table frame needs much more than its cells).
fn table_svg(header: &[Vec<Run>], rows: &[Vec<Vec<Run>>], w: f32, size: f32, fg: &str) -> String {
    let n = header.len().max(1);
    let row_h = size * 1.8;
    let h = row_h * (rows.len() + 1) as f32;
    let col = w / n as f32;
    let mut s = format!(
        "<svg xmlns=\"http://www.w3.org/2000/svg\" width=\"{w}\" height=\"{h}\" viewBox=\"0 0 {w} {h}\" font-family=\"Inter, Noto Sans, DejaVu Sans, sans-serif\" font-size=\"{size}\" fill=\"#{fg}\">"
    );
    let all: Vec<(bool, &[Vec<Run>])> = std::iter::once((true, header)).chain(rows.iter().map(|r| (false, r.as_slice()))).collect();
    for (r, (head, cells)) in all.iter().enumerate() {
        let y = r as f32 * row_h;
        if *head {
            let _ = write!(s, "<rect x=\"0\" y=\"0\" width=\"{w}\" height=\"{row_h}\" fill=\"#808080\" fill-opacity=\"0.2\"/>");
        }
        for i in 0..n {
            let t = cells.get(i).map(|c| flow::runs_text(c)).unwrap_or_default();
            let _ = write!(
                s,
                "<text x=\"{:.1}\" y=\"{:.1}\"{}>{}</text>",
                i as f32 * col + 6.0,
                y + row_h * 0.65,
                if *head { " font-weight=\"bold\"" } else { "" },
                esc(&t)
            );
        }
        let _ = write!(s, "<line x1=\"0\" y1=\"{y}\" x2=\"{w}\" y2=\"{y}\" stroke=\"#9aa4ae\"/>");
    }
    let _ = write!(s, "<line x1=\"0\" y1=\"{h}\" x2=\"{w}\" y2=\"{h}\" stroke=\"#9aa4ae\"/>");
    for i in 0..=n {
        let x = (i as f32 * col).min(w - 0.5).max(0.5);
        let _ = write!(s, "<line x1=\"{x}\" y1=\"0\" x2=\"{x}\" y2=\"{h}\" stroke=\"#9aa4ae\"/>");
    }
    s.push_str("</svg>");
    s
}

pub fn write(doc: &Doc) -> Result<Vec<u8>, String> {
    let slides = super::pptx::slides_of(doc);
    let font = doc
        .meta("mainfont")
        .map(str::to_string)
        .or_else(|| doc.theme.map(|t| t.font.to_string()))
        .filter(|f| f != "system-ui")
        .unwrap_or_else(|| "Liberation Sans".into());
    let mut ctx = Ctx {
        text: TextStyles::default(),
        paras: Vec::new(),
        pictures: Vec::new(),
        font,
        mono: doc.meta("monofont").unwrap_or("Liberation Mono").into(),
    };
    let mut pages = String::new();
    let mut page_styles = String::new();
    for (i, s) in slides.iter().enumerate() {
        let (bg, fg, head) = doc.slide_colors(s);
        let _ = write!(
            page_styles,
            "<style:style style:name=\"dp{}\" style:family=\"drawing-page\"><style:drawing-page-properties draw:fill=\"solid\" draw:fill-color=\"#{}\" presentation:background-visible=\"true\" presentation:background-objects-visible=\"true\"/></style:style>",
            i + 1,
            bg.to_lowercase()
        );
        let mut frames = String::new();
        for b in flow::slide_layout(doc, s) {
            match b {
                SlideBox::Text { x, y, w, h, paras, size, center, title } => {
                    frames.push_str(&ctx.text_box(x, y, w, h, &paras, size, center, title, &fg, &head))
                }
                SlideBox::Picture { x, y, w, h, picture } => frames.push_str(&ctx.picture(&picture, x, y, w, h)),
                SlideBox::Table { x, y, w, h, item: Item::Table { header, rows, .. }, size } => {
                    let svg = table_svg(&header, &rows, w, size, &fg);
                    if let Some(p) = super::svg_png(&svg, 2.0) {
                        let ph = h.min(p.height as f32);
                        frames.push_str(&ctx.picture(&p, x, y, w, ph));
                    }
                }
                SlideBox::Table { .. } => {}
            }
        }
        let notes = s
            .notes
            .as_deref()
            .map(|n| n.split('\n').map(|l| format!("<text:p>{}</text:p>", odf_text(l))).collect::<String>())
            .unwrap_or_default();
        let _ = write!(
            pages,
            "<draw:page draw:name=\"Slide {}\" draw:style-name=\"dp{}\" draw:master-page-name=\"Default\">{frames}<presentation:notes><draw:frame presentation:class=\"notes\" svg:x=\"2cm\" svg:y=\"14cm\" svg:width=\"17cm\" svg:height=\"12cm\"><draw:text-box>{notes}</draw:text-box></draw:frame></presentation:notes></draw:page>",
            i + 1,
            i + 1
        );
    }
    let content = format!(
        "<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n<office:document-content {NS} office:version=\"1.3\"><office:automatic-styles>{page_styles}<style:style style:name=\"gr1\" style:family=\"graphic\"><style:graphic-properties draw:stroke=\"none\" draw:fill=\"none\" draw:auto-grow-height=\"true\" draw:textarea-vertical-align=\"top\" fo:padding=\"0pt\" fo:wrap-option=\"wrap\"/></style:style><style:style style:name=\"grc\" style:family=\"graphic\"><style:graphic-properties draw:stroke=\"none\" draw:fill=\"none\" draw:auto-grow-height=\"false\" draw:textarea-vertical-align=\"middle\" fo:padding=\"0pt\" fo:wrap-option=\"wrap\"/></style:style>{}{}</office:automatic-styles><office:body><office:presentation>{pages}</office:presentation></office:body></office:document-content>",
        ctx.para_styles_xml(),
        ctx.text.xml
    );
    let styles = format!(
        "<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n<office:document-styles {NS} office:version=\"1.3\"><office:styles><style:default-style style:family=\"graphic\"><style:graphic-properties draw:stroke=\"none\"/></style:default-style></office:styles><office:automatic-styles><style:page-layout style:name=\"PM1\"><style:page-layout-properties fo:margin-top=\"0cm\" fo:margin-bottom=\"0cm\" fo:margin-left=\"0cm\" fo:margin-right=\"0cm\" fo:page-width=\"{SLIDE_W}pt\" fo:page-height=\"{SLIDE_H}pt\" style:print-orientation=\"landscape\"/></style:page-layout><style:style style:name=\"dp0\" style:family=\"drawing-page\"><style:drawing-page-properties draw:fill=\"solid\" draw:fill-color=\"#ffffff\"/></style:style></office:automatic-styles><office:master-styles><style:master-page style:name=\"Default\" style:page-layout-name=\"PM1\" draw:style-name=\"dp0\"/></office:master-styles></office:document-styles>"
    );
    let mime = "application/vnd.oasis.opendocument.presentation";
    let mut files: Vec<(String, Vec<u8>)> = vec![
        ("mimetype".into(), mime.as_bytes().to_vec()),
        ("META-INF/manifest.xml".into(), manifest(mime, &ctx.pictures).into_bytes()),
        ("content.xml".into(), content.into_bytes()),
        ("styles.xml".into(), styles.into_bytes()),
        ("meta.xml".into(), meta(doc).into_bytes()),
    ];
    files.extend(ctx.pictures);
    package(&files, &["mimetype"])
}
