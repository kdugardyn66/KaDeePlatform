//! OpenDocument text (.odt).

use std::collections::HashMap;
use std::fmt::Write as _;

use super::xml::esc;
use super::{Doc, Picture, Run, package, runs};
use crate::md::{Align, Block, Inline, ListItem, Style};

pub(crate) const NS: &str = "xmlns:office=\"urn:oasis:names:tc:opendocument:xmlns:office:1.0\" xmlns:style=\"urn:oasis:names:tc:opendocument:xmlns:style:1.0\" xmlns:text=\"urn:oasis:names:tc:opendocument:xmlns:text:1.0\" xmlns:table=\"urn:oasis:names:tc:opendocument:xmlns:table:1.0\" xmlns:draw=\"urn:oasis:names:tc:opendocument:xmlns:drawing:1.0\" xmlns:fo=\"urn:oasis:names:tc:opendocument:xmlns:xsl-fo-compatible:1.0\" xmlns:xlink=\"http://www.w3.org/1999/xlink\" xmlns:dc=\"http://purl.org/dc/elements/1.1/\" xmlns:meta=\"urn:oasis:names:tc:opendocument:xmlns:meta:1.0\" xmlns:svg=\"urn:oasis:names:tc:opendocument:xmlns:svg-compatible:1.0\" xmlns:presentation=\"urn:oasis:names:tc:opendocument:xmlns:presentation:1.0\" xmlns:loext=\"urn:org:documentfoundation:names:experimental:office:xmlns:loext:1.0\"";

/// Text in ODF: runs of spaces as `<text:s>`, tabs and line breaks as elements.
pub(crate) fn odf_text(s: &str) -> String {
    let mut out = String::new();
    let mut spaces = 0;
    let flush = |out: &mut String, n: &mut usize| {
        match *n {
            0 => {}
            1 => out.push(' '),
            k => {
                out.push(' ');
                let _ = write!(out, "<text:s text:c=\"{}\"/>", k - 1);
            }
        }
        *n = 0;
    };
    for c in s.chars() {
        match c {
            ' ' => spaces += 1,
            '\t' => {
                flush(&mut out, &mut spaces);
                out.push_str("<text:tab/>");
            }
            '\n' => {
                flush(&mut out, &mut spaces);
                out.push_str("<text:line-break/>");
            }
            c => {
                flush(&mut out, &mut spaces);
                out.push_str(&esc(&c.to_string()));
            }
        }
    }
    flush(&mut out, &mut spaces);
    out
}

/// Automatic text styles, one per combination of inline styles.
#[derive(Default)]
pub(crate) struct TextStyles {
    pub names: HashMap<String, String>,
    pub xml: String,
}

impl TextStyles {
    /// The style name for `s` (with `mono` for code), or `None` for plain text.
    pub fn name(&mut self, s: &Style, mono: &str) -> Option<String> {
        let mut p = String::new();
        if s.bold {
            p.push_str(" fo:font-weight=\"bold\" style:font-weight-complex=\"bold\"");
        }
        if s.italic {
            p.push_str(" fo:font-style=\"italic\"");
        }
        if s.strike {
            p.push_str(" style:text-line-through-style=\"solid\"");
        }
        if s.underline || s.link.is_some() {
            p.push_str(
                " style:text-underline-style=\"solid\" style:text-underline-width=\"auto\" style:text-underline-color=\"font-color\"",
            );
        }
        if s.highlight {
            p.push_str(" fo:background-color=\"#fff3a3\"");
        } else if let Some(c) = s.background.as_deref().and_then(super::parse_hex) {
            let _ = write!(p, " fo:background-color=\"#{c:06x}\"");
        }
        if s.code {
            let _ = write!(p, " style:font-name=\"{}\" fo:font-family=\"{}\" fo:background-color=\"#eff1f3\"", esc(mono), esc(mono));
        } else if let Some(f) = &s.font {
            let _ = write!(p, " fo:font-family=\"{}\"", esc(f));
        }
        if s.link.is_some() {
            p.push_str(" fo:color=\"#0969da\"");
        } else if let Some(c) = s.color.as_deref().and_then(super::parse_hex) {
            let _ = write!(p, " fo:color=\"#{c:06x}\"");
        }
        if let Some(pt) = s.size.as_deref().and_then(crate::md::style::points) {
            let _ = write!(p, " fo:font-size=\"{pt}pt\"");
        }
        if s.sup {
            p.push_str(" style:text-position=\"super 58%\"");
        } else if s.sub {
            p.push_str(" style:text-position=\"sub 58%\"");
        }
        if p.is_empty() {
            return None;
        }
        if let Some(n) = self.names.get(&p) {
            return Some(n.clone());
        }
        let name = format!("T{}", self.names.len() + 1);
        let _ = write!(self.xml, "<style:style style:name=\"{name}\" style:family=\"text\"><style:text-properties{p}/></style:style>");
        self.names.insert(p, name.clone());
        Some(name)
    }
}

struct Ctx<'a> {
    doc: &'a Doc,
    body: String,
    text: TextStyles,
    pictures: Vec<(String, Vec<u8>)>,
    mono: String,
    max_w_cm: f32,
    tables: usize,
}

pub(crate) fn picture_frame(name: &str, href: &str, w_cm: f32, h_cm: f32, anchor: &str) -> String {
    format!(
        "<draw:frame draw:name=\"{name}\" text:anchor-type=\"{anchor}\" svg:width=\"{w_cm:.3}cm\" svg:height=\"{h_cm:.3}cm\"><draw:image xlink:href=\"{href}\" xlink:type=\"simple\" xlink:show=\"embed\" xlink:actuate=\"onLoad\"/></draw:frame>"
    )
}

impl Ctx<'_> {
    fn picture(&mut self, p: &Picture) -> String {
        let n = self.pictures.len() + 1;
        let href = format!("Pictures/image{n}.{}", if p.ext == "jpeg" { "jpg" } else { "png" });
        self.pictures.push((href.clone(), p.data.clone()));
        let (w, h) = p.points(self.max_w_cm / 2.54 * 72.0);
        picture_frame(&format!("Image{n}"), &href, w / 72.0 * 2.54, h / 72.0 * 2.54, "as-char")
    }

    fn inlines(&mut self, inl: &[Inline], extra: Option<&Style>) -> String {
        let mut out = String::new();
        let mut open_link: Option<String> = None;
        for r in runs(inl) {
            match r {
                Run::Text(t, mut s) => {
                    if let Some(e) = extra {
                        s.bold |= e.bold;
                        s.italic |= e.italic;
                    }
                    if open_link.as_ref() != s.link.as_ref() {
                        if open_link.take().is_some() {
                            out.push_str("</text:a>");
                        }
                        if let Some(u) = &s.link {
                            let _ = write!(out, "<text:a xlink:type=\"simple\" xlink:href=\"{}\">", esc(u));
                            open_link = Some(u.clone());
                        }
                    }
                    match self.text.name(&s, &self.mono) {
                        Some(n) => {
                            let _ = write!(out, "<text:span text:style-name=\"{n}\">{}</text:span>", odf_text(&t));
                        }
                        None => out.push_str(&odf_text(&t)),
                    }
                }
                Run::Break => out.push_str("<text:line-break/>"),
                Run::Image(url, alt) => match self.doc.picture(&url) {
                    Some(p) => {
                        let f = self.picture(&p);
                        out.push_str(&f);
                    }
                    None => out.push_str(&odf_text(&format!("[{alt}]"))),
                },
            }
        }
        if open_link.is_some() {
            out.push_str("</text:a>");
        }
        out
    }

    fn p(&mut self, style: &str, content: &str) {
        let _ = write!(self.body, "<text:p text:style-name=\"{style}\">{content}</text:p>");
    }

    fn blocks(&mut self, blocks: &[Block], quote: bool) {
        for b in blocks {
            self.block(b, quote);
        }
    }

    fn block(&mut self, b: &Block, quote: bool) {
        let body_style = if quote { "Quotations" } else { "Text_20_body" };
        match b {
            Block::Heading { level, inlines, .. } => {
                let x = self.inlines(inlines, None);
                let l = (*level).clamp(1, 6);
                let _ = write!(self.body, "<text:h text:style-name=\"Heading_20_{l}\" text:outline-level=\"{l}\">{x}</text:h>");
            }
            Block::Paragraph { inlines, .. } => {
                let x = self.inlines(inlines, None);
                self.p(body_style, &x);
            }
            Block::List { ordered, start, items, .. } => self.list(*ordered, *start, items, quote, true),
            Block::Quote { blocks, .. } => self.blocks(blocks, true),
            Block::Callout { kind, title, blocks, .. } => {
                let t = title.clone().unwrap_or_else(|| {
                    let k = kind.to_lowercase();
                    let mut c = k.chars();
                    c.next().map(|f| f.to_uppercase().collect::<String>() + c.as_str()).unwrap_or_default()
                });
                let style = format!("Callout_{}", kind.to_lowercase());
                let _ = write!(
                    self.body,
                    "<text:p text:style-name=\"{style}\"><text:span text:style-name=\"Bold\">{}</text:span></text:p>",
                    odf_text(&t)
                );
                let before = self.body.len();
                self.blocks(blocks, false);
                let inner = self.body.split_off(before);
                self.body.push_str(&inner.replace("text:style-name=\"Text_20_body\"", &format!("text:style-name=\"{style}\"")));
            }
            Block::Code { info, text, .. } => {
                if let Some(p) = super::is_drawn(info, text) {
                    let f = self.picture(&p);
                    self.p("Figure", &f);
                    return;
                }
                let lines: Vec<String> = text.trim_end_matches('\n').split('\n').map(odf_text).collect();
                self.p("Preformatted_20_Text", &lines.join("<text:line-break/>"));
            }
            Block::Math { text, .. } => {
                let x = odf_text(&crate::md::math::to_unicode(text));
                self.p("Formula", &format!("<text:span text:style-name=\"Italic\">{x}</text:span>"));
            }
            Block::Table { aligns, header, rows, .. } => self.table(aligns, header, rows),
            Block::Rule { .. } => self.p("Horizontal_20_Line", ""),
            Block::PageBreak { .. } | Block::SlideBreak { .. } => self.p("PageBreak", ""),
            Block::Toc { .. } => {
                for h in crate::md::outline(&self.doc.parsed).iter().filter(|h| h.level <= 3) {
                    let x = odf_text(&h.text);
                    self.p(&format!("TOC{}", h.level), &x);
                }
            }
            Block::Html { text, .. } => {
                let mut t = String::new();
                let mut tag = false;
                for c in text.chars() {
                    match c {
                        '<' => tag = true,
                        '>' if tag => tag = false,
                        c if !tag => t.push(c),
                        _ => {}
                    }
                }
                if !t.trim().is_empty() {
                    let x = odf_text(t.trim());
                    self.p(body_style, &x);
                }
            }
            Block::Footnote { label, blocks, .. } => {
                let before = self.body.len();
                self.blocks(blocks, quote);
                let mark = format!("<text:span text:style-name=\"Sup\">[{}]</text:span> ", esc(label));
                if let Some(i) = self.body[before..].find('>') {
                    self.body.insert_str(before + i + 1, &mark);
                }
                let inner = self.body.split_off(before);
                self.body.push_str(&inner.replacen("text:style-name=\"Text_20_body\"", "text:style-name=\"Footnote\"", 1));
            }
        }
    }

    fn list(&mut self, ordered: bool, start: usize, items: &[ListItem], quote: bool, top: bool) {
        let style = if !top {
            String::new()
        } else if ordered {
            " text:style-name=\"LN\"".to_string()
        } else {
            " text:style-name=\"LB\"".to_string()
        };
        let _ = write!(self.body, "<text:list{style}>");
        for (i, it) in items.iter().enumerate() {
            if i == 0 && ordered && start != 1 {
                let _ = write!(self.body, "<text:list-item text:start-value=\"{start}\">");
            } else {
                self.body.push_str("<text:list-item>");
            }
            for (k, b) in it.blocks.iter().enumerate() {
                match b {
                    Block::Paragraph { inlines, .. } if k == 0 => {
                        let mut x = String::new();
                        if let Some(done) = it.task {
                            x.push_str(if done { "☑ " } else { "☐ " });
                        }
                        x.push_str(&self.inlines(inlines, None));
                        self.p(if quote { "Quotations" } else { "List_20_Contents" }, &x);
                    }
                    Block::List { ordered, start, items, .. } => self.list(*ordered, *start, items, quote, false),
                    other => self.block(other, quote),
                }
            }
            self.body.push_str("</text:list-item>");
        }
        self.body.push_str("</text:list>");
    }

    fn table(&mut self, aligns: &[Align], header: &[Vec<Inline>], rows: &[Vec<Vec<Inline>>]) {
        self.tables += 1;
        let n = header.len().max(1);
        let _ = write!(
            self.body,
            "<table:table table:name=\"Table{}\" table:style-name=\"Tbl\"><table:table-column table:number-columns-repeated=\"{n}\"/>",
            self.tables
        );
        let para = |i: usize| match aligns.get(i) {
            Some(Align::Center) => "Table_20_Center",
            Some(Align::Right) => "Table_20_Right",
            _ => "Table_20_Contents",
        };
        let bold = Style { bold: true, ..Style::default() };
        self.body.push_str("<table:table-header-rows><table:table-row>");
        for i in 0..n {
            let x = header.get(i).map(|c| self.inlines(c, Some(&bold))).unwrap_or_default();
            let _ = write!(
                self.body,
                "<table:table-cell table:style-name=\"TH\" office:value-type=\"string\"><text:p text:style-name=\"{}\">{x}</text:p></table:table-cell>",
                para(i)
            );
        }
        self.body.push_str("</table:table-row></table:table-header-rows>");
        for r in rows {
            self.body.push_str("<table:table-row>");
            for i in 0..n {
                let x = r.get(i).map(|c| self.inlines(c, None)).unwrap_or_default();
                let _ = write!(
                    self.body,
                    "<table:table-cell table:style-name=\"TC\" office:value-type=\"string\"><text:p text:style-name=\"{}\">{x}</text:p></table:table-cell>",
                    para(i)
                );
            }
            self.body.push_str("</table:table-row>");
        }
        self.body.push_str("</table:table>");
    }
}

fn styles(doc: &Doc) -> String {
    let (pw, ph) = doc.page_size();
    let m = doc.margin();
    let cm = |pt: f32| pt / 72.0 * 2.54;
    let font = esc(&doc.font());
    let size = doc.font_size();
    let hc = format!("#{}", doc.heading_color().to_lowercase());
    let mono = esc(doc.meta("monofont").unwrap_or("Liberation Mono"));
    let justify = if doc.meta("justify") == Some("true") { " fo:text-align=\"justify\"" } else { "" };
    let mut s = format!(
        "<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n<office:document-styles {NS} office:version=\"1.3\"><office:font-face-decls><style:font-face style:name=\"{font}\" svg:font-family=\"'{font}'\"/><style:font-face style:name=\"{mono}\" svg:font-family=\"'{mono}'\" style:font-pitch=\"fixed\"/></office:font-face-decls><office:styles>\
<style:default-style style:family=\"paragraph\"><style:paragraph-properties fo:margin-top=\"0cm\" fo:margin-bottom=\"0.2cm\"/><style:text-properties style:font-name=\"{font}\" fo:font-family=\"'{font}'\" fo:font-size=\"{size}pt\" fo:language=\"en\" fo:country=\"GB\"/></style:default-style>\
<style:style style:name=\"Standard\" style:family=\"paragraph\" style:class=\"text\"/>\
<style:style style:name=\"Text_20_body\" style:display-name=\"Text body\" style:family=\"paragraph\" style:parent-style-name=\"Standard\" style:class=\"text\"><style:paragraph-properties fo:margin-bottom=\"0.25cm\" fo:line-height=\"125%\"{justify}/></style:style>\
<style:style style:name=\"Heading\" style:family=\"paragraph\" style:parent-style-name=\"Standard\" style:next-style-name=\"Text_20_body\" style:class=\"text\"><style:paragraph-properties fo:margin-top=\"0.42cm\" fo:margin-bottom=\"0.21cm\" fo:keep-with-next=\"always\"/><style:text-properties fo:font-weight=\"bold\" style:font-weight-complex=\"bold\"/></style:style>"
    );
    let scale = [2.0f32, 1.5, 1.25, 1.0, 0.875, 0.85];
    for (i, k) in scale.iter().enumerate() {
        let l = i + 1;
        let color = if l <= 2 { hc.clone() } else { "#1f2328".into() };
        let border = if l <= 2 { " fo:border-bottom=\"0.5pt solid #d0d7de\" fo:padding-bottom=\"0.1cm\"" } else { "" };
        let _ = write!(
            s,
            "<style:style style:name=\"Heading_20_{l}\" style:display-name=\"Heading {l}\" style:family=\"paragraph\" style:parent-style-name=\"Heading\" style:next-style-name=\"Text_20_body\" style:default-outline-level=\"{l}\" style:class=\"text\"><style:paragraph-properties{border}/><style:text-properties fo:font-size=\"{:.1}pt\" fo:color=\"{color}\"/></style:style>",
            size * k
        );
    }
    let _ = write!(
        s,
        "<style:style style:name=\"Quotations\" style:family=\"paragraph\" style:parent-style-name=\"Text_20_body\" style:class=\"html\"><style:paragraph-properties fo:margin-left=\"0.6cm\" fo:padding-left=\"0.3cm\" fo:border-left=\"2.5pt solid #d0d7de\"/><style:text-properties fo:color=\"#59636e\" fo:font-style=\"italic\"/></style:style>\
<style:style style:name=\"Preformatted_20_Text\" style:display-name=\"Preformatted Text\" style:family=\"paragraph\" style:parent-style-name=\"Standard\" style:class=\"html\"><style:paragraph-properties fo:background-color=\"#f6f8fa\" fo:padding=\"0.2cm\" fo:margin-bottom=\"0.3cm\"/><style:text-properties style:font-name=\"{mono}\" fo:font-family=\"'{mono}'\" fo:font-size=\"9.5pt\"/></style:style>\
<style:style style:name=\"List_20_Contents\" style:display-name=\"List Contents\" style:family=\"paragraph\" style:parent-style-name=\"Standard\" style:class=\"list\"><style:paragraph-properties fo:margin-bottom=\"0.08cm\"/></style:style>\
<style:style style:name=\"Table_20_Contents\" style:display-name=\"Table Contents\" style:family=\"paragraph\" style:parent-style-name=\"Standard\" style:class=\"extra\"><style:paragraph-properties fo:margin-bottom=\"0cm\"/></style:style>\
<style:style style:name=\"Table_20_Center\" style:family=\"paragraph\" style:parent-style-name=\"Table_20_Contents\"><style:paragraph-properties fo:text-align=\"center\"/></style:style>\
<style:style style:name=\"Table_20_Right\" style:family=\"paragraph\" style:parent-style-name=\"Table_20_Contents\"><style:paragraph-properties fo:text-align=\"end\"/></style:style>\
<style:style style:name=\"Figure\" style:family=\"paragraph\" style:parent-style-name=\"Standard\"><style:paragraph-properties fo:text-align=\"center\" fo:margin-bottom=\"0.3cm\"/></style:style>\
<style:style style:name=\"Formula\" style:family=\"paragraph\" style:parent-style-name=\"Standard\"><style:paragraph-properties fo:text-align=\"center\"/></style:style>\
<style:style style:name=\"Footnote\" style:family=\"paragraph\" style:parent-style-name=\"Standard\"><style:text-properties fo:font-size=\"9pt\" fo:color=\"#59636e\"/></style:style>\
<style:style style:name=\"Horizontal_20_Line\" style:display-name=\"Horizontal Line\" style:family=\"paragraph\" style:parent-style-name=\"Standard\"><style:paragraph-properties fo:border-bottom=\"1pt solid #d0d7de\" fo:padding=\"0cm\" fo:margin-bottom=\"0.3cm\"/><style:text-properties fo:font-size=\"4pt\"/></style:style>\
<style:style style:name=\"PageBreak\" style:family=\"paragraph\" style:parent-style-name=\"Standard\"><style:paragraph-properties fo:break-after=\"page\"/><style:text-properties fo:font-size=\"2pt\"/></style:style>\
<style:style style:name=\"TOC1\" style:family=\"paragraph\" style:parent-style-name=\"Standard\"><style:paragraph-properties fo:margin-bottom=\"0.1cm\"/><style:text-properties fo:color=\"#0969da\"/></style:style>\
<style:style style:name=\"TOC2\" style:family=\"paragraph\" style:parent-style-name=\"TOC1\"><style:paragraph-properties fo:margin-left=\"0.6cm\"/></style:style>\
<style:style style:name=\"TOC3\" style:family=\"paragraph\" style:parent-style-name=\"TOC1\"><style:paragraph-properties fo:margin-left=\"1.2cm\"/></style:style>\
<style:style style:name=\"Bold\" style:family=\"text\"><style:text-properties fo:font-weight=\"bold\"/></style:style>\
<style:style style:name=\"Italic\" style:family=\"text\"><style:text-properties fo:font-style=\"italic\"/></style:style>\
<style:style style:name=\"Sup\" style:family=\"text\"><style:text-properties style:text-position=\"super 58%\"/></style:style>"
    );
    for (k, c) in [("note", "#0969da"), ("tip", "#1a7f37"), ("important", "#8250df"), ("warning", "#9a6700"), ("caution", "#d1242f")] {
        let _ = write!(
            s,
            "<style:style style:name=\"Callout_{k}\" style:family=\"paragraph\" style:parent-style-name=\"Text_20_body\"><style:paragraph-properties fo:margin-left=\"0.2cm\" fo:padding-left=\"0.3cm\" fo:border-left=\"3pt solid {c}\" fo:margin-bottom=\"0cm\" fo:padding-bottom=\"0.15cm\"/></style:style>"
        );
    }
    let _ = write!(
        s,
        "</office:styles><office:automatic-styles><style:page-layout style:name=\"pm1\"><style:page-layout-properties fo:page-width=\"{:.2}cm\" fo:page-height=\"{:.2}cm\" style:print-orientation=\"{}\" fo:margin-top=\"{m:.2}cm\" fo:margin-bottom=\"{m:.2}cm\" fo:margin-left=\"{m:.2}cm\" fo:margin-right=\"{m:.2}cm\"/></style:page-layout></office:automatic-styles><office:master-styles><style:master-page style:name=\"Standard\" style:page-layout-name=\"pm1\"/></office:master-styles></office:document-styles>",
        cm(pw),
        cm(ph),
        if pw > ph { "landscape" } else { "portrait" },
        m = cm(m)
    );
    s
}

pub(crate) fn manifest(mime: &str, pictures: &[(String, Vec<u8>)]) -> String {
    let mut m = format!(
        "<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n<manifest:manifest xmlns:manifest=\"urn:oasis:names:tc:opendocument:xmlns:manifest:1.0\" manifest:version=\"1.3\"><manifest:file-entry manifest:full-path=\"/\" manifest:version=\"1.3\" manifest:media-type=\"{mime}\"/><manifest:file-entry manifest:full-path=\"content.xml\" manifest:media-type=\"text/xml\"/><manifest:file-entry manifest:full-path=\"styles.xml\" manifest:media-type=\"text/xml\"/><manifest:file-entry manifest:full-path=\"meta.xml\" manifest:media-type=\"text/xml\"/>"
    );
    for (p, _) in pictures {
        let t = if p.ends_with(".jpg") { "image/jpeg" } else { "image/png" };
        let _ = write!(m, "<manifest:file-entry manifest:full-path=\"{p}\" manifest:media-type=\"{t}\"/>");
    }
    m.push_str("</manifest:manifest>");
    m
}

pub(crate) fn meta(doc: &Doc) -> String {
    let now = super::now_iso();
    format!(
        "<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n<office:document-meta {NS} office:version=\"1.3\"><office:meta><meta:generator>Kadee Markdown</meta:generator><dc:title>{}</dc:title><meta:initial-creator>{}</meta:initial-creator><dc:creator>{}</dc:creator><meta:creation-date>{now}</meta:creation-date><dc:date>{now}</dc:date></office:meta></office:document-meta>",
        esc(&doc.title),
        esc(&doc.author),
        esc(&doc.author)
    )
}

/// The list styles (bullets and numbers) as automatic styles.
pub(crate) fn list_styles() -> String {
    let mut s = String::from("<text:list-style style:name=\"LB\">");
    for l in 1..=10 {
        let _ = write!(
            s,
            "<text:list-level-style-bullet text:level=\"{l}\" text:bullet-char=\"{}\"><style:list-level-properties text:list-level-position-and-space-mode=\"label-alignment\"><style:list-level-label-alignment text:label-followed-by=\"listtab\" fo:text-indent=\"-0.5cm\" fo:margin-left=\"{:.2}cm\"/></style:list-level-properties></text:list-level-style-bullet>",
            ["•", "◦", "▪"][(l - 1) % 3],
            0.6 * l as f32 + 0.1
        );
    }
    s.push_str("</text:list-style><text:list-style style:name=\"LN\">");
    for l in 1..=10 {
        let _ = write!(
            s,
            "<text:list-level-style-number text:level=\"{l}\" style:num-suffix=\".\" style:num-format=\"{}\"><style:list-level-properties text:list-level-position-and-space-mode=\"label-alignment\"><style:list-level-label-alignment text:label-followed-by=\"listtab\" fo:text-indent=\"-0.6cm\" fo:margin-left=\"{:.2}cm\"/></style:list-level-properties></text:list-level-style-number>",
            ["1", "a", "i"][(l - 1) % 3],
            0.7 * l as f32 + 0.1
        );
    }
    s.push_str("</text:list-style>");
    s
}

pub fn write(doc: &Doc) -> Result<Vec<u8>, String> {
    let (pw, _) = doc.page_size();
    let mono = doc.meta("monofont").unwrap_or("Liberation Mono").to_string();
    let mut ctx = Ctx {
        doc,
        body: String::new(),
        text: TextStyles::default(),
        pictures: Vec::new(),
        mono,
        max_w_cm: (pw - 2.0 * doc.margin()) / 72.0 * 2.54,
        tables: 0,
    };
    if doc.slides.is_empty() {
        let blocks = doc.parsed.blocks.clone();
        ctx.blocks(&blocks, false);
    } else {
        for (i, s) in doc.slides.iter().enumerate() {
            if i > 0 {
                ctx.p("PageBreak", "");
            }
            let blocks = s.parsed.blocks.clone();
            ctx.blocks(&blocks, false);
            if let Some(n) = &s.notes {
                let x = odf_text(&format!("Notes: {n}"));
                ctx.p("Footnote", &x);
            }
        }
    }
    let content = format!(
        "<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n<office:document-content {NS} office:version=\"1.3\"><office:automatic-styles>{}{}<style:style style:name=\"Tbl\" style:family=\"table\"><style:table-properties style:width=\"{:.2}cm\" table:align=\"margins\" fo:margin-bottom=\"0.3cm\"/></style:style><style:style style:name=\"TH\" style:family=\"table-cell\"><style:table-cell-properties fo:background-color=\"#f2f4f7\" fo:padding=\"0.1cm\" fo:border=\"0.5pt solid #d0d7de\"/></style:style><style:style style:name=\"TC\" style:family=\"table-cell\"><style:table-cell-properties fo:padding=\"0.1cm\" fo:border=\"0.5pt solid #d0d7de\"/></style:style></office:automatic-styles><office:body><office:text>{}</office:text></office:body></office:document-content>",
        list_styles(),
        ctx.text.xml,
        ctx.max_w_cm,
        ctx.body
    );
    let mime = "application/vnd.oasis.opendocument.text";
    let mut files: Vec<(String, Vec<u8>)> = vec![
        ("mimetype".into(), mime.as_bytes().to_vec()),
        ("META-INF/manifest.xml".into(), manifest(mime, &ctx.pictures).into_bytes()),
        ("content.xml".into(), content.into_bytes()),
        ("styles.xml".into(), styles(doc).into_bytes()),
        ("meta.xml".into(), meta(doc).into_bytes()),
    ];
    files.extend(ctx.pictures);
    package(&files, &["mimetype"])
}
