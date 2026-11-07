//! Word (.docx, Office Open XML).

use std::fmt::Write as _;

use super::xml::esc;
use super::{Doc, Picture, Run, package, runs};
use crate::md::para::{BorderStyle, LineSpacing, ParaAlign, ParaStyle, Special};
use crate::md::{Align, Block, ListItem, Style};

const W: &str = "http://schemas.openxmlformats.org/wordprocessingml/2006/main";
const R: &str = "http://schemas.openxmlformats.org/officeDocument/2006/relationships";

struct Ctx<'a> {
    doc: &'a Doc,
    body: String,
    rels: Vec<(String, String, String, bool)>, // id, type, target, external
    media: Vec<(String, Vec<u8>)>,
    nums: Vec<(usize, usize)>, // (numId, start) for ordered lists
    pic_id: usize,
    bookmark: usize,
    max_w_emu: i64,
}

/// A Word bookmark name for a heading's anchor (`next-steps` → `h_next_steps`).
fn bookmark(anchor: &str) -> String {
    let b: String = anchor.chars().map(|c| if c.is_alphanumeric() { c } else { '_' }).collect();
    format!("h_{}", b.chars().take(36).collect::<String>())
}

impl Ctx<'_> {
    fn rel(&mut self, kind: &str, target: &str, external: bool) -> String {
        let id = format!("rId{}", self.rels.len() + 10);
        self.rels.push((id.clone(), kind.into(), target.into(), external));
        id
    }

    fn picture_run(&mut self, p: &Picture, max_pt: f32) -> String {
        self.pic_id += 1;
        let n = self.pic_id;
        let name = format!("media/image{n}.{}", if p.ext == "jpeg" { "jpg" } else { "png" });
        self.media.push((format!("word/{name}"), p.data.clone()));
        let rid = self.rel("http://schemas.openxmlformats.org/officeDocument/2006/relationships/image", &name, false);
        let (w, h) = p.points(max_pt);
        let (cx, cy) = ((w * 12700.0) as i64, (h * 12700.0) as i64);
        let (cx, cy) = if cx > self.max_w_emu { (self.max_w_emu, cy * self.max_w_emu / cx.max(1)) } else { (cx, cy) };
        format!(
            "<w:r><w:drawing><wp:inline distT=\"0\" distB=\"0\" distL=\"0\" distR=\"0\"><wp:extent cx=\"{cx}\" cy=\"{cy}\"/><wp:docPr id=\"{n}\" name=\"Picture {n}\"/><wp:cNvGraphicFramePr><a:graphicFrameLocks noChangeAspect=\"1\"/></wp:cNvGraphicFramePr><a:graphic><a:graphicData uri=\"http://schemas.openxmlformats.org/drawingml/2006/picture\"><pic:pic><pic:nvPicPr><pic:cNvPr id=\"{n}\" name=\"image{n}\"/><pic:cNvPicPr/></pic:nvPicPr><pic:blipFill><a:blip r:embed=\"{rid}\"/><a:stretch><a:fillRect/></a:stretch></pic:blipFill><pic:spPr><a:xfrm><a:off x=\"0\" y=\"0\"/><a:ext cx=\"{cx}\" cy=\"{cy}\"/></a:xfrm><a:prstGeom prst=\"rect\"><a:avLst/></a:prstGeom></pic:spPr></pic:pic></a:graphicData></a:graphic></wp:inline></w:drawing></w:r>"
        )
    }

    fn rpr(s: &Style, extra: &str) -> String {
        let mut r = String::new();
        if s.code {
            r.push_str("<w:rStyle w:val=\"CodeChar\"/>");
        } else if s.link.is_some() {
            r.push_str("<w:rStyle w:val=\"Hyperlink\"/>");
        }
        if let Some(f) = &s.font {
            let f = esc(f);
            let _ = write!(r, "<w:rFonts w:ascii=\"{f}\" w:hAnsi=\"{f}\" w:cs=\"{f}\"/>");
        }
        if s.bold {
            r.push_str("<w:b/>");
        }
        if s.italic {
            r.push_str("<w:i/>");
        }
        if s.strike {
            r.push_str("<w:strike/>");
        }
        if let Some(c) = s.color.as_deref().and_then(super::parse_hex) {
            let _ = write!(r, "<w:color w:val=\"{c:06X}\"/>");
        }
        if let Some(pt) = s.size.as_deref().and_then(crate::md::style::points) {
            let _ = write!(r, "<w:sz w:val=\"{}\"/>", (pt * 2.0).round() as i32);
        }
        if s.highlight {
            r.push_str("<w:highlight w:val=\"yellow\"/>");
        }
        if s.underline {
            r.push_str("<w:u w:val=\"single\"/>");
        }
        if let Some(c) = s.background.as_deref().and_then(super::parse_hex) {
            let _ = write!(r, "<w:shd w:val=\"clear\" w:color=\"auto\" w:fill=\"{c:06X}\"/>");
        }
        if s.sup {
            r.push_str("<w:vertAlign w:val=\"superscript\"/>");
        } else if s.sub {
            r.push_str("<w:vertAlign w:val=\"subscript\"/>");
        }
        r.push_str(extra);
        if r.is_empty() { r } else { format!("<w:rPr>{r}</w:rPr>") }
    }

    fn text_run(text: &str, s: &Style, extra: &str) -> String {
        let mut out = String::new();
        for (i, part) in text.split('\n').enumerate() {
            if i > 0 {
                out.push_str("<w:r><w:br/></w:r>");
            }
            if !part.is_empty() {
                let _ = write!(out, "<w:r>{}<w:t xml:space=\"preserve\">{}</w:t></w:r>", Self::rpr(s, extra), esc(part));
            }
        }
        out
    }

    fn inlines(&mut self, inl: &[crate::md::Inline], extra: &str) -> String {
        let mut out = String::new();
        let mut pending_link: Option<(String, String)> = None; // url, xml
        let flush = |out: &mut String, link: &mut Option<(String, String)>, this: &mut Self| {
            if let Some((url, xml)) = link.take() {
                if let Some(anchor) = url.strip_prefix('#') {
                    let _ = write!(out, "<w:hyperlink w:anchor=\"{}\">{xml}</w:hyperlink>", esc(&bookmark(anchor)));
                } else {
                    let id = this.rel("http://schemas.openxmlformats.org/officeDocument/2006/relationships/hyperlink", &url, true);
                    let _ = write!(out, "<w:hyperlink r:id=\"{id}\">{xml}</w:hyperlink>");
                }
            }
        };
        for r in runs(inl) {
            match r {
                Run::Text(t, s) => {
                    let xml = Self::text_run(&t, &s, extra);
                    match (&s.link, &mut pending_link) {
                        (Some(u), Some((lu, lx))) if u == lu => lx.push_str(&xml),
                        (Some(u), _) => {
                            flush(&mut out, &mut pending_link, self);
                            pending_link = Some((u.clone(), xml));
                        }
                        (None, _) => {
                            flush(&mut out, &mut pending_link, self);
                            out.push_str(&xml);
                        }
                    }
                }
                Run::Break => {
                    flush(&mut out, &mut pending_link, self);
                    out.push_str("<w:r><w:br/></w:r>");
                }
                Run::Image(url, alt) => {
                    flush(&mut out, &mut pending_link, self);
                    match self.doc.picture(&url) {
                        Some(p) => {
                            let x = self.picture_run(&p, 451.0);
                            out.push_str(&x);
                        }
                        None => out.push_str(&Self::text_run(&format!("[{alt}]"), &Style::default(), "")),
                    }
                }
            }
        }
        flush(&mut out, &mut pending_link, self);
        out
    }

    #[allow(clippy::too_many_arguments)]
    fn para(&mut self, style: Option<&str>, ppr_extra: &str, content: &str) {
        let mut ppr = String::new();
        if let Some(s) = style {
            let _ = write!(ppr, "<w:pStyle w:val=\"{s}\"/>");
        }
        ppr.push_str(ppr_extra);
        let ppr = if ppr.is_empty() { String::new() } else { format!("<w:pPr>{ppr}</w:pPr>") };
        let _ = write!(self.body, "<w:p>{ppr}{content}</w:p>");
    }

    fn blocks(&mut self, blocks: &[Block], quote: usize, list: Option<(usize, usize)>) {
        for b in blocks {
            self.block(b, quote, list);
        }
    }

    fn quote_ppr(quote: usize) -> String {
        if quote == 0 {
            return String::new();
        }
        format!(
            "<w:pBdr><w:left w:val=\"single\" w:sz=\"18\" w:space=\"8\" w:color=\"D0D7DE\"/></w:pBdr><w:ind w:left=\"{}\"/>",
            quote * 360
        )
    }

    fn block(&mut self, b: &Block, quote: usize, list: Option<(usize, usize)>) {
        let qp = Self::quote_ppr(quote);
        match b {
            Block::Heading { level, inlines, .. } => {
                let x = self.inlines(inlines, "");
                let name: String = super::plain(inlines);
                let id = bookmark(&crate::md::html::anchor(&name));
                self.bookmark += 1;
                let n = self.bookmark;
                let bm = format!("<w:bookmarkStart w:id=\"{n}\" w:name=\"{}\"/>{x}<w:bookmarkEnd w:id=\"{n}\"/>", esc(&id));
                self.para(Some(&format!("Heading{}", (*level).clamp(1, 6))), "", &bm);
            }
            Block::Paragraph { inlines, .. } => {
                let x = self.inlines(inlines, "");
                let numpr = list
                    .map(|(id, lvl)| format!("<w:numPr><w:ilvl w:val=\"{lvl}\"/><w:numId w:val=\"{id}\"/></w:numPr>"))
                    .unwrap_or_default();
                let style = if quote > 0 {
                    Some("Quote")
                } else if list.is_some() {
                    Some("ListParagraph")
                } else {
                    None
                };
                self.para(style, &format!("{numpr}{qp}"), &x);
            }
            Block::List { ordered, start, items, .. } => {
                let level = list.map(|(_, l)| l + 1).unwrap_or(0).min(8);
                let num = if *ordered {
                    let id = 3 + self.nums.len();
                    self.nums.push((id, *start));
                    id
                } else {
                    1
                };
                for it in items {
                    self.item(it, num, level, quote);
                }
            }
            Block::Quote { blocks, .. } => self.blocks(blocks, quote + 1, list),
            Block::Styled { style, blocks, .. } => {
                let before = self.body.len();
                self.blocks(blocks, quote, list);
                let inner = self.body.split_off(before);
                self.body.push_str(&styled_paras(&inner, style));
            }
            Block::Callout { kind, title, blocks, .. } => {
                let color = match kind.to_lowercase().as_str() {
                    "tip" => "1A7F37",
                    "important" => "8250DF",
                    "warning" => "9A6700",
                    "caution" => "D1242F",
                    _ => "0969DA",
                };
                let t = title.clone().unwrap_or_else(|| {
                    let k = kind.to_lowercase();
                    let mut c = k.chars();
                    c.next().map(|f| f.to_uppercase().collect::<String>() + c.as_str()).unwrap_or_default()
                });
                let bdr = format!(
                    "<w:pBdr><w:left w:val=\"single\" w:sz=\"24\" w:space=\"8\" w:color=\"{color}\"/></w:pBdr><w:ind w:left=\"240\"/>"
                );
                let x = Self::text_run(&t, &Style { bold: true, color: Some(format!("#{color}")), ..Style::default() }, "");
                self.para(None, &bdr, &x);
                let before = self.body.len();
                self.blocks(blocks, 0, None);
                // the callout's border on its paragraphs
                let inner = self.body.split_off(before);
                self.body.push_str(
                    &inner
                        .replace("<w:p><w:pPr>", &format!("<w:p><w:pPr>{bdr}"))
                        .replace("<w:p><w:r>", &format!("<w:p><w:pPr>{bdr}</w:pPr><w:r>")),
                );
            }
            Block::Code { lang, info, text, .. } => {
                if let Some(p) = super::is_drawn(info, text) {
                    let x = self.picture_run(&p, 451.0);
                    self.para(Some("Figure"), "", &x);
                    return;
                }
                let _ = lang;
                let lines: Vec<&str> = text.trim_end_matches('\n').split('\n').collect();
                let mut x = String::new();
                for (i, l) in lines.iter().enumerate() {
                    if i > 0 {
                        x.push_str("<w:r><w:br/></w:r>");
                    }
                    let _ = write!(x, "<w:r><w:t xml:space=\"preserve\">{}</w:t></w:r>", esc(l));
                }
                self.para(Some("Code"), &qp, &x);
            }
            Block::Math { text, .. } => {
                let x = Self::text_run(&crate::md::math::to_unicode(text), &Style { italic: true, ..Style::default() }, "");
                self.para(None, "<w:jc w:val=\"center\"/>", &x);
            }
            Block::Table { aligns, header, rows, .. } => self.table(aligns, header, rows),
            Block::Rule { .. } => {
                self.para(None, "<w:pBdr><w:bottom w:val=\"single\" w:sz=\"8\" w:space=\"1\" w:color=\"D0D7DE\"/></w:pBdr>", "");
            }
            Block::PageBreak { .. } | Block::SlideBreak { .. } => self.para(None, "", "<w:r><w:br w:type=\"page\"/></w:r>"),
            Block::Toc { .. } => {
                let field = "<w:r><w:fldChar w:fldCharType=\"begin\" w:dirty=\"true\"/></w:r><w:r><w:instrText xml:space=\"preserve\"> TOC \\o \"1-3\" \\h \\z \\u </w:instrText></w:r><w:r><w:fldChar w:fldCharType=\"separate\"/></w:r>";
                self.para(None, "", field);
                for h in crate::md::outline(&self.doc.parsed).iter().filter(|h| h.level <= 3) {
                    let ind = format!("<w:ind w:left=\"{}\"/>", (h.level as usize - 1) * 360);
                    let x = Self::text_run(&h.text, &Style::default(), "");
                    self.para(Some("TOC"), &ind, &x);
                }
                self.para(None, "", "<w:r><w:fldChar w:fldCharType=\"end\"/></w:r>");
            }
            Block::Html { text, .. } => {
                let t: String = strip_tags(text);
                if !t.trim().is_empty() {
                    let x = Self::text_run(t.trim(), &Style::default(), "");
                    self.para(None, &qp, &x);
                }
            }
            Block::Footnote { label, blocks, .. } => {
                let first = self.body.len();
                self.blocks(blocks, quote, None);
                let lbl = Self::text_run(&format!("[{label}] "), &Style { sup: true, ..Style::default() }, "");
                if let Some(i) = self.body[first..].find("<w:r>") {
                    self.body.insert_str(first + i, &lbl);
                }
            }
        }
    }

    fn item(&mut self, it: &ListItem, num: usize, level: usize, quote: usize) {
        for (k, b) in it.blocks.iter().enumerate() {
            match b {
                Block::Paragraph { inlines, .. } if k == 0 => {
                    let mut x = String::new();
                    if let Some(done) = it.task {
                        x.push_str(&Self::text_run(if done { "☑ " } else { "☐ " }, &Style::default(), ""));
                    }
                    x.push_str(&self.inlines(inlines, ""));
                    let ppr = format!("<w:numPr><w:ilvl w:val=\"{level}\"/><w:numId w:val=\"{num}\"/></w:numPr>{}", Self::quote_ppr(quote));
                    self.para(Some("ListParagraph"), &ppr, &x);
                }
                Block::List { .. } => self.block(b, quote, Some((num, level))),
                other => {
                    let ind = format!("<w:ind w:left=\"{}\"/>", 720 + level * 360);
                    let before = self.body.len();
                    self.block(other, quote, None);
                    let inner = self.body.split_off(before);
                    self.body.push_str(&inner.replacen("<w:p><w:pPr>", &format!("<w:p><w:pPr>{ind}"), 1));
                }
            }
        }
    }

    #[allow(clippy::too_many_arguments)]
    fn table(&mut self, aligns: &[Align], header: &[Vec<crate::md::Inline>], rows: &[Vec<Vec<crate::md::Inline>>]) {
        let n = header.len().max(1);
        let col_w = 9000 / n;
        let mut x = String::from(
            "<w:tbl><w:tblPr><w:tblStyle w:val=\"TableGrid\"/><w:tblW w:w=\"5000\" w:type=\"pct\"/><w:tblLook w:val=\"04A0\" w:firstRow=\"1\" w:lastRow=\"0\" w:firstColumn=\"0\" w:lastColumn=\"0\" w:noHBand=\"0\" w:noVBand=\"1\"/></w:tblPr><w:tblGrid>",
        );
        for _ in 0..n {
            let _ = write!(x, "<w:gridCol w:w=\"{col_w}\"/>");
        }
        x.push_str("</w:tblGrid>");
        let jc = |i: usize| match aligns.get(i) {
            Some(Align::Center) => "<w:jc w:val=\"center\"/>",
            Some(Align::Right) => "<w:jc w:val=\"right\"/>",
            _ => "",
        };
        let all: Vec<(bool, Vec<Vec<crate::md::Inline>>)> =
            std::iter::once((true, header.to_vec())).chain(rows.iter().map(|r| (false, r.clone()))).collect();
        for (head, row) in all {
            x.push_str("<w:tr>");
            if head {
                x.push_str("<w:trPr><w:tblHeader/></w:trPr>");
            }
            for i in 0..n {
                let cell = row.get(i).cloned().unwrap_or_default();
                let content = self.inlines(&cell, if head { "<w:b/>" } else { "" });
                let shade = if head { "<w:shd w:val=\"clear\" w:color=\"auto\" w:fill=\"F2F4F7\"/>" } else { "" };
                let _ = write!(
                    x,
                    "<w:tc><w:tcPr><w:tcW w:w=\"{col_w}\" w:type=\"dxa\"/>{shade}</w:tcPr><w:p><w:pPr><w:spacing w:before=\"40\" w:after=\"40\"/>{}</w:pPr>{content}</w:p></w:tc>",
                    jc(i)
                );
            }
            x.push_str("</w:tr>");
        }
        x.push_str("</w:tbl>");
        self.body.push_str(&x);
        self.para(None, "<w:spacing w:after=\"0\"/>", "");
    }
}

fn strip_tags(html: &str) -> String {
    let mut out = String::new();
    let mut tag = false;
    for c in html.chars() {
        match c {
            '<' => tag = true,
            '>' if tag => tag = false,
            c if !tag => out.push(c),
            _ => {}
        }
    }
    out
}

fn styles(doc: &Doc) -> String {
    let font = esc(&doc.font());
    let size = (doc.font_size() * 2.0).round() as i32;
    let hc = doc.heading_color();
    let mono = esc(doc.meta("monofont").unwrap_or("Consolas"));
    let justify = if doc.meta("justify") == Some("true") { "<w:jc w:val=\"both\"/>" } else { "" };
    let mut s = format!(
        "<?xml version=\"1.0\" encoding=\"UTF-8\" standalone=\"yes\"?>\n<w:styles xmlns:w=\"{W}\"><w:docDefaults><w:rPrDefault><w:rPr><w:rFonts w:ascii=\"{font}\" w:hAnsi=\"{font}\" w:eastAsia=\"{font}\" w:cs=\"{font}\"/><w:sz w:val=\"{size}\"/><w:szCs w:val=\"{size}\"/><w:lang w:val=\"en-GB\"/></w:rPr></w:rPrDefault><w:pPrDefault><w:pPr><w:spacing w:after=\"160\" w:line=\"276\" w:lineRule=\"auto\"/></w:pPr></w:pPrDefault></w:docDefaults>\
<w:style w:type=\"paragraph\" w:default=\"1\" w:styleId=\"Normal\"><w:name w:val=\"Normal\"/><w:qFormat/><w:pPr>{justify}</w:pPr></w:style>"
    );
    let scale = [2.0f32, 1.5, 1.25, 1.0, 0.875, 0.85];
    for (i, k) in scale.iter().enumerate() {
        let hp = (doc.font_size() * k * 2.0).round() as i32;
        let lvl = i + 1;
        let color = if lvl <= 2 { hc.clone() } else { "1F2328".into() };
        let border =
            if lvl <= 2 { "<w:pBdr><w:bottom w:val=\"single\" w:sz=\"4\" w:space=\"4\" w:color=\"D0D7DE\"/></w:pBdr>" } else { "" };
        let _ = write!(
            s,
            "<w:style w:type=\"paragraph\" w:styleId=\"Heading{lvl}\"><w:name w:val=\"heading {lvl}\"/><w:basedOn w:val=\"Normal\"/><w:next w:val=\"Normal\"/><w:uiPriority w:val=\"9\"/><w:qFormat/><w:pPr><w:keepNext/><w:keepLines/>{border}<w:spacing w:before=\"{}\" w:after=\"120\"/><w:outlineLvl w:val=\"{}\"/></w:pPr><w:rPr><w:b/><w:color w:val=\"{color}\"/><w:sz w:val=\"{}\"/><w:szCs w:val=\"{}\"/></w:rPr></w:style>",
            if lvl == 1 { 360 } else { 240 },
            i,
            hp,
            hp
        );
    }
    let _ = write!(
        s,
        "<w:style w:type=\"paragraph\" w:styleId=\"Title\"><w:name w:val=\"Title\"/><w:basedOn w:val=\"Normal\"/><w:qFormat/><w:rPr><w:b/><w:color w:val=\"{hc}\"/><w:sz w:val=\"56\"/></w:rPr></w:style>\
<w:style w:type=\"paragraph\" w:styleId=\"Quote\"><w:name w:val=\"Quote\"/><w:basedOn w:val=\"Normal\"/><w:qFormat/><w:rPr><w:i/><w:color w:val=\"59636E\"/></w:rPr></w:style>\
<w:style w:type=\"paragraph\" w:styleId=\"Code\"><w:name w:val=\"Code\"/><w:basedOn w:val=\"Normal\"/><w:pPr><w:shd w:val=\"clear\" w:color=\"auto\" w:fill=\"F6F8FA\"/><w:spacing w:after=\"160\" w:line=\"240\" w:lineRule=\"auto\"/><w:ind w:left=\"120\" w:right=\"120\"/></w:pPr><w:rPr><w:rFonts w:ascii=\"{mono}\" w:hAnsi=\"{mono}\" w:cs=\"{mono}\"/><w:sz w:val=\"19\"/></w:rPr></w:style>\
<w:style w:type=\"character\" w:styleId=\"CodeChar\"><w:name w:val=\"Code Char\"/><w:rPr><w:rFonts w:ascii=\"{mono}\" w:hAnsi=\"{mono}\" w:cs=\"{mono}\"/><w:sz w:val=\"19\"/><w:shd w:val=\"clear\" w:color=\"auto\" w:fill=\"EFF1F3\"/></w:rPr></w:style>\
<w:style w:type=\"character\" w:styleId=\"Hyperlink\"><w:name w:val=\"Hyperlink\"/><w:rPr><w:color w:val=\"0969DA\"/><w:u w:val=\"single\"/></w:rPr></w:style>\
<w:style w:type=\"paragraph\" w:styleId=\"ListParagraph\"><w:name w:val=\"List Paragraph\"/><w:basedOn w:val=\"Normal\"/><w:pPr><w:spacing w:after=\"60\"/><w:contextualSpacing/></w:pPr></w:style>\
<w:style w:type=\"paragraph\" w:styleId=\"Figure\"><w:name w:val=\"Figure\"/><w:basedOn w:val=\"Normal\"/><w:pPr><w:jc w:val=\"center\"/></w:pPr></w:style>\
<w:style w:type=\"paragraph\" w:styleId=\"TOC\"><w:name w:val=\"toc 1\"/><w:basedOn w:val=\"Normal\"/><w:pPr><w:spacing w:after=\"60\"/></w:pPr><w:rPr><w:color w:val=\"0969DA\"/></w:rPr></w:style>\
<w:style w:type=\"table\" w:styleId=\"TableGrid\"><w:name w:val=\"Table Grid\"/><w:tblPr><w:tblBorders><w:top w:val=\"single\" w:sz=\"4\" w:space=\"0\" w:color=\"D0D7DE\"/><w:left w:val=\"single\" w:sz=\"4\" w:space=\"0\" w:color=\"D0D7DE\"/><w:bottom w:val=\"single\" w:sz=\"4\" w:space=\"0\" w:color=\"D0D7DE\"/><w:right w:val=\"single\" w:sz=\"4\" w:space=\"0\" w:color=\"D0D7DE\"/><w:insideH w:val=\"single\" w:sz=\"4\" w:space=\"0\" w:color=\"D0D7DE\"/><w:insideV w:val=\"single\" w:sz=\"4\" w:space=\"0\" w:color=\"D0D7DE\"/></w:tblBorders><w:tblCellMar><w:left w:w=\"108\" w:type=\"dxa\"/><w:right w:w=\"108\" w:type=\"dxa\"/></w:tblCellMar></w:tblPr></w:style>\
</w:styles>"
    );
    s
}

fn numbering(nums: &[(usize, usize)]) -> String {
    let mut s = format!("<?xml version=\"1.0\" encoding=\"UTF-8\" standalone=\"yes\"?>\n<w:numbering xmlns:w=\"{W}\">");
    let bullets = ["•", "◦", "▪"];
    s.push_str("<w:abstractNum w:abstractNumId=\"0\"><w:multiLevelType w:val=\"hybridMultilevel\"/>");
    for l in 0..9 {
        let _ = write!(
            s,
            "<w:lvl w:ilvl=\"{l}\"><w:start w:val=\"1\"/><w:numFmt w:val=\"bullet\"/><w:lvlText w:val=\"{}\"/><w:lvlJc w:val=\"left\"/><w:pPr><w:ind w:left=\"{}\" w:hanging=\"360\"/></w:pPr></w:lvl>",
            bullets[l % 3],
            720 + l * 360
        );
    }
    s.push_str("</w:abstractNum><w:abstractNum w:abstractNumId=\"1\"><w:multiLevelType w:val=\"hybridMultilevel\"/>");
    let fmts = ["decimal", "lowerLetter", "lowerRoman"];
    for l in 0..9 {
        let _ = write!(
            s,
            "<w:lvl w:ilvl=\"{l}\"><w:start w:val=\"1\"/><w:numFmt w:val=\"{}\"/><w:lvlText w:val=\"%{}.\"/><w:lvlJc w:val=\"left\"/><w:pPr><w:ind w:left=\"{}\" w:hanging=\"360\"/></w:pPr></w:lvl>",
            fmts[l % 3],
            l + 1,
            720 + l * 360
        );
    }
    s.push_str("</w:abstractNum><w:num w:numId=\"1\"><w:abstractNumId w:val=\"0\"/></w:num>");
    for (id, start) in nums {
        let _ = write!(
            s,
            "<w:num w:numId=\"{id}\"><w:abstractNumId w:val=\"1\"/><w:lvlOverride w:ilvl=\"0\"><w:startOverride w:val=\"{start}\"/></w:lvlOverride></w:num>"
        );
    }
    s.push_str("</w:numbering>");
    s
}

pub fn write(doc: &Doc) -> Result<Vec<u8>, String> {
    let (pw, ph) = doc.page_size();
    let m = doc.margin();
    let text_w = pw - 2.0 * m;
    let mut ctx = Ctx {
        doc,
        body: String::new(),
        rels: Vec::new(),
        media: Vec::new(),
        nums: Vec::new(),
        pic_id: 0,
        bookmark: 0,
        max_w_emu: (text_w * 12700.0) as i64,
    };
    if doc.slides.is_empty() {
        let blocks = doc.parsed.blocks.clone();
        ctx.blocks(&blocks, 0, None);
    } else {
        // a deck as a document: a page per slide
        for (i, s) in doc.slides.iter().enumerate() {
            if i > 0 {
                ctx.para(None, "", "<w:r><w:br w:type=\"page\"/></w:r>");
            }
            let blocks = s.parsed.blocks.clone();
            ctx.blocks(&blocks, 0, None);
            if let Some(n) = &s.notes {
                let x =
                    Ctx::text_run(&format!("Notes: {n}"), &Style { italic: true, color: Some("#59636E".into()), ..Style::default() }, "");
                ctx.para(None, "", &x);
            }
        }
    }
    let tw = |pt: f32| (pt * 20.0).round() as i32;
    let sect = format!(
        "<w:sectPr><w:pgSz w:w=\"{}\" w:h=\"{}\"{}/><w:pgMar w:top=\"{m2}\" w:right=\"{m2}\" w:bottom=\"{m2}\" w:left=\"{m2}\" w:header=\"708\" w:footer=\"708\" w:gutter=\"0\"/></w:sectPr>",
        tw(pw),
        tw(ph),
        if pw > ph { " w:orient=\"landscape\"" } else { "" },
        m2 = tw(m)
    );
    let document = format!(
        "<?xml version=\"1.0\" encoding=\"UTF-8\" standalone=\"yes\"?>\n<w:document xmlns:w=\"{W}\" xmlns:r=\"{R}\" xmlns:wp=\"http://schemas.openxmlformats.org/drawingml/2006/wordprocessingDrawing\" xmlns:a=\"http://schemas.openxmlformats.org/drawingml/2006/main\" xmlns:pic=\"http://schemas.openxmlformats.org/drawingml/2006/picture\"><w:body>{}{sect}</w:body></w:document>",
        ctx.body
    );
    let mut rels = String::from(
        "<?xml version=\"1.0\" encoding=\"UTF-8\" standalone=\"yes\"?>\n<Relationships xmlns=\"http://schemas.openxmlformats.org/package/2006/relationships\"><Relationship Id=\"rId1\" Type=\"http://schemas.openxmlformats.org/officeDocument/2006/relationships/styles\" Target=\"styles.xml\"/><Relationship Id=\"rId2\" Type=\"http://schemas.openxmlformats.org/officeDocument/2006/relationships/numbering\" Target=\"numbering.xml\"/><Relationship Id=\"rId3\" Type=\"http://schemas.openxmlformats.org/officeDocument/2006/relationships/settings\" Target=\"settings.xml\"/>",
    );
    for (id, kind, target, ext) in &ctx.rels {
        let mode = if *ext { " TargetMode=\"External\"" } else { "" };
        let _ = write!(rels, "<Relationship Id=\"{id}\" Type=\"{kind}\" Target=\"{}\"{mode}/>", esc(target));
    }
    rels.push_str("</Relationships>");
    let content_types = "<?xml version=\"1.0\" encoding=\"UTF-8\" standalone=\"yes\"?>\n<Types xmlns=\"http://schemas.openxmlformats.org/package/2006/content-types\"><Default Extension=\"rels\" ContentType=\"application/vnd.openxmlformats-package.relationships+xml\"/><Default Extension=\"xml\" ContentType=\"application/xml\"/><Default Extension=\"png\" ContentType=\"image/png\"/><Default Extension=\"jpg\" ContentType=\"image/jpeg\"/><Override PartName=\"/word/document.xml\" ContentType=\"application/vnd.openxmlformats-officedocument.wordprocessingml.document.main+xml\"/><Override PartName=\"/word/styles.xml\" ContentType=\"application/vnd.openxmlformats-officedocument.wordprocessingml.styles+xml\"/><Override PartName=\"/word/numbering.xml\" ContentType=\"application/vnd.openxmlformats-officedocument.wordprocessingml.numbering+xml\"/><Override PartName=\"/word/settings.xml\" ContentType=\"application/vnd.openxmlformats-officedocument.wordprocessingml.settings+xml\"/><Override PartName=\"/docProps/core.xml\" ContentType=\"application/vnd.openxmlformats-package.core-properties+xml\"/><Override PartName=\"/docProps/app.xml\" ContentType=\"application/vnd.openxmlformats-officedocument.extended-properties+xml\"/></Types>";
    let root_rels = "<?xml version=\"1.0\" encoding=\"UTF-8\" standalone=\"yes\"?>\n<Relationships xmlns=\"http://schemas.openxmlformats.org/package/2006/relationships\"><Relationship Id=\"rId1\" Type=\"http://schemas.openxmlformats.org/officeDocument/2006/relationships/officeDocument\" Target=\"word/document.xml\"/><Relationship Id=\"rId2\" Type=\"http://schemas.openxmlformats.org/package/2006/relationships/metadata/core-properties\" Target=\"docProps/core.xml\"/><Relationship Id=\"rId3\" Type=\"http://schemas.openxmlformats.org/officeDocument/2006/relationships/extended-properties\" Target=\"docProps/app.xml\"/></Relationships>";
    let now = super::now_iso();
    let core = format!(
        "<?xml version=\"1.0\" encoding=\"UTF-8\" standalone=\"yes\"?>\n<cp:coreProperties xmlns:cp=\"http://schemas.openxmlformats.org/package/2006/metadata/core-properties\" xmlns:dc=\"http://purl.org/dc/elements/1.1/\" xmlns:dcterms=\"http://purl.org/dc/terms/\" xmlns:xsi=\"http://www.w3.org/2001/XMLSchema-instance\"><dc:title>{}</dc:title><dc:creator>{}</dc:creator><dcterms:created xsi:type=\"dcterms:W3CDTF\">{now}</dcterms:created><dcterms:modified xsi:type=\"dcterms:W3CDTF\">{now}</dcterms:modified></cp:coreProperties>",
        esc(&doc.title),
        esc(&doc.author)
    );
    let app = "<?xml version=\"1.0\" encoding=\"UTF-8\" standalone=\"yes\"?>\n<Properties xmlns=\"http://schemas.openxmlformats.org/officeDocument/2006/extended-properties\"><Application>Kadee Markdown</Application></Properties>";
    let settings = format!(
        "<?xml version=\"1.0\" encoding=\"UTF-8\" standalone=\"yes\"?>\n<w:settings xmlns:w=\"{W}\"><w:updateFields w:val=\"true\"/><w:defaultTabStop w:val=\"720\"/><w:compat><w:compatSetting w:name=\"compatibilityMode\" w:uri=\"http://schemas.microsoft.com/office/word\" w:val=\"15\"/></w:compat></w:settings>"
    );
    let mut files: Vec<(String, Vec<u8>)> = vec![
        ("[Content_Types].xml".into(), content_types.as_bytes().to_vec()),
        ("_rels/.rels".into(), root_rels.as_bytes().to_vec()),
        ("docProps/core.xml".into(), core.into_bytes()),
        ("docProps/app.xml".into(), app.as_bytes().to_vec()),
        ("word/document.xml".into(), document.into_bytes()),
        ("word/styles.xml".into(), styles(doc).into_bytes()),
        ("word/numbering.xml".into(), numbering(&ctx.nums).into_bytes()),
        ("word/settings.xml".into(), settings.into_bytes()),
        ("word/_rels/document.xml.rels".into(), rels.into_bytes()),
    ];
    files.extend(ctx.media);
    package(&files, &[])
}

/// The order of a paragraph's properties in `<w:pPr>` (Word is strict about it).
const PPR_ORDER: [&str; 36] = [
    "pStyle",
    "keepNext",
    "keepLines",
    "pageBreakBefore",
    "framePr",
    "widowControl",
    "numPr",
    "suppressLineNumbers",
    "pBdr",
    "shd",
    "tabs",
    "suppressAutoHyphens",
    "kinsoku",
    "wordWrap",
    "overflowPunct",
    "topLinePunct",
    "autoSpaceDE",
    "autoSpaceDN",
    "bidi",
    "adjustRightInd",
    "snapToGrid",
    "spacing",
    "ind",
    "contextualSpacing",
    "mirrorIndents",
    "suppressOverlap",
    "jc",
    "textDirection",
    "textAlignment",
    "textboxTightWrap",
    "outlineLvl",
    "divId",
    "cnfStyle",
    "rPr",
    "sectPr",
    "pPrChange",
];

/// The elements of a `<w:pPr>`: (name, xml).
fn ppr_elements(inner: &str) -> Vec<(String, String)> {
    let mut out = Vec::new();
    let mut i = 0;
    while let Some(p) = inner[i..].find("<w:").map(|p| p + i) {
        let name_end = inner[p + 3..].find([' ', '/', '>']).map(|e| p + 3 + e).unwrap_or(inner.len());
        let name = inner[p + 3..name_end].to_string();
        let tag_end = inner[p..].find('>').map(|e| p + e + 1).unwrap_or(inner.len());
        let end = if inner[..tag_end].ends_with("/>") {
            tag_end
        } else {
            let close = format!("</w:{name}>");
            inner[tag_end..].find(&close).map(|e| tag_end + e + close.len()).unwrap_or(inner.len())
        };
        out.push((name, inner[p..end].to_string()));
        i = end;
    }
    out
}

/// A paragraph's formatting as `<w:pPr>` elements.
fn style_ppr(style: &ParaStyle) -> Vec<(String, String)> {
    let tw = |cm: f32| (cm * 566.93).round() as i32;
    let mut v: Vec<(String, String)> = Vec::new();
    let mut add = |n: &str, x: String| v.push((n.to_string(), x));
    if style.keep_with_next {
        add("keepNext", "<w:keepNext/>".into());
    }
    if style.keep_together {
        add("keepLines", "<w:keepLines/>".into());
    }
    if style.page_break_before {
        add("pageBreakBefore", "<w:pageBreakBefore/>".into());
    }
    if let Some(b) = style.border {
        let val = match b.style {
            BorderStyle::Solid => "single",
            BorderStyle::Dashed => "dashed",
            BorderStyle::Dotted => "dotted",
            BorderStyle::Double => "double",
        };
        let sz = (b.width * 8.0).round().clamp(2.0, 96.0) as i32;
        let side = |s: &str| format!("<w:{s} w:val=\"{val}\" w:sz=\"{sz}\" w:space=\"4\" w:color=\"{:06X}\"/>", b.color);
        add("pBdr", format!("<w:pBdr>{}{}{}{}</w:pBdr>", side("top"), side("left"), side("bottom"), side("right")));
    }
    if let Some(c) = style.background {
        add("shd", format!("<w:shd w:val=\"clear\" w:color=\"auto\" w:fill=\"{c:06X}\"/>"));
    }
    if style.rtl {
        add("bidi", "<w:bidi/>".into());
    }
    let mut sp = String::new();
    if let Some(b) = style.before {
        let _ = write!(sp, " w:before=\"{}\"", tw(b));
    }
    if let Some(a) = style.after {
        let _ = write!(sp, " w:after=\"{}\"", tw(a));
    }
    match style.line {
        Some(LineSpacing::Multiple(m)) => {
            let _ = write!(sp, " w:line=\"{}\" w:lineRule=\"auto\"", (m * 240.0).round() as i32);
        }
        Some(LineSpacing::Exactly(pt)) => {
            let _ = write!(sp, " w:line=\"{}\" w:lineRule=\"exact\"", (pt * 20.0).round() as i32);
        }
        None => {}
    }
    if !sp.is_empty() {
        add("spacing", format!("<w:spacing{sp}/>"));
    }
    let mut ind = String::new();
    if style.left.is_some() || matches!(style.special, Some(Special::Hanging(_))) {
        let _ = write!(ind, " w:left=\"{}\"", tw(style.left_total()));
    }
    if let Some(r) = style.right {
        let _ = write!(ind, " w:right=\"{}\"", tw(r));
    }
    match style.special {
        Some(Special::FirstLine(f)) => {
            let _ = write!(ind, " w:firstLine=\"{}\"", tw(f));
        }
        Some(Special::Hanging(h)) => {
            let _ = write!(ind, " w:hanging=\"{}\"", tw(h));
        }
        None => {}
    }
    if !ind.is_empty() {
        add("ind", format!("<w:ind{ind}/>"));
    }
    if let Some(a) = style.align {
        let val = match a {
            ParaAlign::Left => "left",
            ParaAlign::Center => "center",
            ParaAlign::Right => "right",
            ParaAlign::Justify => "both",
        };
        add("jc", format!("<w:jc w:val=\"{val}\"/>"));
    }
    v
}

/// The paragraphs in `xml` with `style` added to their properties.
fn styled_paras(xml: &str, style: &ParaStyle) -> String {
    let ours = style_ppr(style);
    if ours.is_empty() {
        return xml.to_string();
    }
    let mut out = String::with_capacity(xml.len() + 64);
    let mut rest = xml;
    while let Some(p) = rest.find("<w:p>") {
        out.push_str(&rest[..p + 5]);
        rest = &rest[p + 5..];
        let mut els = Vec::new();
        if let Some(r) = rest.strip_prefix("<w:pPr>")
            && let Some(e) = r.find("</w:pPr>")
        {
            els = ppr_elements(&r[..e]);
            rest = &r[e + 8..];
        }
        for (name, x) in &ours {
            els.retain(|(n, _)| n != name);
            els.push((name.clone(), x.clone()));
        }
        els.sort_by_key(|(n, _)| PPR_ORDER.iter().position(|o| o == n).unwrap_or(PPR_ORDER.len()));
        out.push_str("<w:pPr>");
        for (_, x) in els {
            out.push_str(&x);
        }
        out.push_str("</w:pPr>");
    }
    out.push_str(rest);
    out
}
