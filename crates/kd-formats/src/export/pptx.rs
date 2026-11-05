//! PowerPoint (.pptx): one slide per Markdown slide (a document without slides becomes one
//! slide per level-1 or level-2 heading), with speaker notes.

use std::fmt::Write as _;

use super::flow::{self, Item, Kind, Para, SLIDE_H, SLIDE_W, SlideBox};
use super::xml::esc;
use super::{Doc, Run, SlideDoc, package};
use crate::md::{Align, Style};

const NS: &str = "xmlns:a=\"http://schemas.openxmlformats.org/drawingml/2006/main\" xmlns:r=\"http://schemas.openxmlformats.org/officeDocument/2006/relationships\" xmlns:p=\"http://schemas.openxmlformats.org/presentationml/2006/main\"";
const HEAD: &str = "<?xml version=\"1.0\" encoding=\"UTF-8\" standalone=\"yes\"?>\n";
const GRP: &str = "<p:nvGrpSpPr><p:cNvPr id=\"1\" name=\"\"/><p:cNvGrpSpPr/><p:nvPr/></p:nvGrpSpPr><p:grpSpPr><a:xfrm><a:off x=\"0\" y=\"0\"/><a:ext cx=\"0\" cy=\"0\"/><a:chOff x=\"0\" y=\"0\"/><a:chExt cx=\"0\" cy=\"0\"/></a:xfrm></p:grpSpPr>";
const CLRMAP: &str = "<p:clrMap bg1=\"lt1\" tx1=\"dk1\" bg2=\"lt2\" tx2=\"dk2\" accent1=\"accent1\" accent2=\"accent2\" accent3=\"accent3\" accent4=\"accent4\" accent5=\"accent5\" accent6=\"accent6\" hlink=\"hlink\" folHlink=\"folHlink\"/>";

fn emu(pt: f32) -> i64 {
    (pt * 12700.0).round() as i64
}

/// The slides of a document without separators: a new slide at every level-1 or -2 heading.
pub(crate) fn slides_of(doc: &Doc) -> Vec<SlideDoc> {
    if !doc.slides.is_empty() {
        return doc
            .slides
            .iter()
            .map(|s| SlideDoc {
                parsed: s.parsed.clone(),
                notes: s.notes.clone(),
                class: s.class.clone(),
                background: s.background.clone(),
            })
            .collect();
    }
    let mut out: Vec<SlideDoc> = Vec::new();
    let mut cur: Vec<crate::md::Block> = Vec::new();
    for b in &doc.parsed.blocks {
        let starts = matches!(b, crate::md::Block::Heading { level, .. } if *level <= 2);
        if starts && !cur.is_empty() {
            out.push(SlideDoc {
                parsed: crate::md::Parsed { front_matter: Vec::new(), blocks: std::mem::take(&mut cur) },
                notes: None,
                class: String::new(),
                background: None,
            });
        }
        if !matches!(b, crate::md::Block::PageBreak { .. } | crate::md::Block::Toc { .. }) {
            cur.push(b.clone());
        }
    }
    if !cur.is_empty() || out.is_empty() {
        out.push(SlideDoc {
            parsed: crate::md::Parsed { front_matter: Vec::new(), blocks: cur },
            notes: None,
            class: String::new(),
            background: None,
        });
    }
    if let Some(first) = out.first_mut()
        && first.parsed.blocks.len() <= 3
    {
        first.class = "title".into();
    }
    out
}

struct Slide {
    xml: String,
    rels: Vec<(String, String, String, bool)>,
    media: Vec<(String, Vec<u8>)>,
}

struct Ctx<'a> {
    fg: String,
    head: String,
    font: String,
    mono: String,
    slide: &'a mut Slide,
    next_id: usize,
    media_no: &'a mut usize,
}

impl Ctx<'_> {
    fn id(&mut self) -> usize {
        self.next_id += 1;
        self.next_id
    }

    fn rel(&mut self, kind: &str, target: &str, external: bool) -> String {
        let id = format!("rId{}", self.slide.rels.len() + 3);
        self.slide.rels.push((id.clone(), kind.into(), target.into(), external));
        id
    }

    fn run(&mut self, t: &str, s: &Style, size: f32, color: &str, mono: bool) -> String {
        let mut a = format!(" lang=\"en-GB\" sz=\"{}\"", (size * 100.0).round() as i32);
        if s.bold {
            a.push_str(" b=\"1\"");
        }
        if s.italic {
            a.push_str(" i=\"1\"");
        }
        if s.underline || s.link.is_some() {
            a.push_str(" u=\"sng\"");
        }
        if s.strike {
            a.push_str(" strike=\"sngStrike\"");
        }
        if s.sup {
            a.push_str(" baseline=\"30000\"");
        } else if s.sub {
            a.push_str(" baseline=\"-25000\"");
        }
        let color = s
            .color
            .as_deref()
            .and_then(super::parse_hex)
            .map(|c| format!("{c:06X}"))
            .unwrap_or_else(|| if s.link.is_some() { "4493F8".into() } else { color.to_string() });
        let mut inner = format!("<a:solidFill><a:srgbClr val=\"{color}\"/></a:solidFill>");
        if s.highlight {
            inner.push_str("<a:highlight><a:srgbClr val=\"FFF3A3\"/></a:highlight>");
        }
        let font = if mono || s.code { self.mono.clone() } else { s.font.clone().unwrap_or_else(|| self.font.clone()) };
        let _ = write!(inner, "<a:latin typeface=\"{}\"/><a:cs typeface=\"{}\"/>", esc(&font), esc(&font));
        if let Some(u) = &s.link
            && !u.starts_with('#')
        {
            let id = self.rel("http://schemas.openxmlformats.org/officeDocument/2006/relationships/hyperlink", u, true);
            let _ = write!(inner, "<a:hlinkClick r:id=\"{id}\"/>");
        }
        format!("<a:r><a:rPr{a}>{inner}</a:rPr><a:t>{}</a:t></a:r>", esc(t))
    }

    fn runs(&mut self, rs: &[Run], size: f32, color: &str, mono: bool) -> String {
        let mut out = String::new();
        for r in rs {
            match r {
                Run::Text(t, s) => {
                    for (i, part) in t.split('\n').enumerate() {
                        if i > 0 {
                            out.push_str("<a:br/>");
                        }
                        if !part.is_empty() {
                            out.push_str(&self.run(part, s, size, color, mono));
                        }
                    }
                }
                Run::Break => out.push_str("<a:br/>"),
                Run::Image(_, alt) => out.push_str(&self.run(&format!("[{alt}]"), &Style::default(), size, color, mono)),
            }
        }
        out
    }

    fn paras(&mut self, paras: &[Para], size: f32, center: bool, title: bool) -> String {
        let mut out = String::new();
        for p in paras {
            let s = if title { size * 1.8 } else { size * flow::scale(p.kind) };
            let color = match p.kind {
                Kind::Heading(_) => self.head.clone(),
                Kind::Callout(c) => format!("{c:06X}"),
                Kind::Quote | Kind::Small => self.fg.clone(),
                _ => self.fg.clone(),
            };
            let algn = if center || p.kind == Kind::Center { " algn=\"ctr\"" } else { "" };
            let bullet = match &p.bullet {
                Some(b) if !b.is_empty() => {
                    let indent = emu(s * 1.1);
                    let mar = emu((p.level as f32) * s * 1.2);
                    if b.ends_with('.') {
                        format!(
                            " marL=\"{mar}\" indent=\"-{indent}\"><a:buFont typeface=\"+mj-lt\"/><a:buAutoNum type=\"arabicPeriod\" startAt=\"{}\"/>",
                            b.trim_end_matches('.').parse::<i32>().unwrap_or(1)
                        )
                    } else {
                        format!(" marL=\"{mar}\" indent=\"-{indent}\"><a:buFont typeface=\"Arial\"/><a:buChar char=\"{}\"/>", esc(b))
                    }
                }
                Some(_) => format!(" marL=\"{}\" indent=\"0\"><a:buNone/>", emu((p.level as f32) * s * 1.2)),
                None => match p.kind {
                    Kind::Quote | Kind::Callout(_) => format!(" marL=\"{}\" indent=\"0\"><a:buNone/>", emu(s)),
                    _ => "><a:buNone/>".to_string(),
                },
            };
            let spc = format!("<a:spcBef><a:spcPts val=\"{}\"/></a:spcBef>", (s * 30.0) as i32);
            // spacing goes before the bullet elements in a:pPr
            let ppr = {
                let (attrs, rest) = bullet.split_once('>').unwrap_or((&bullet, ""));
                format!("<a:pPr{algn}{attrs}>{spc}{rest}</a:pPr>")
            };
            let mut rs = p.runs.clone();
            if matches!(p.kind, Kind::Heading(_)) || title {
                for r in rs.iter_mut() {
                    if let Run::Text(_, st) = r {
                        st.bold = true;
                    }
                }
            }
            if p.kind == Kind::Quote {
                for r in rs.iter_mut() {
                    if let Run::Text(_, st) = r {
                        st.italic = true;
                    }
                }
            }
            let body = self.runs(&rs, s, &color, p.kind == Kind::Code);
            let _ = write!(out, "<a:p>{ppr}{body}<a:endParaRPr lang=\"en-GB\" sz=\"{}\"/></a:p>", (s * 100.0) as i32);
        }
        out
    }

    #[allow(clippy::too_many_arguments)]
    fn text_box(&mut self, x: f32, y: f32, w: f32, h: f32, paras: &[Para], size: f32, center: bool, title: bool) {
        let id = self.id();
        let body = self.paras(paras, size, center, title);
        let anchor = if center { "ctr" } else { "t" };
        let _ = write!(
            self.slide.xml,
            "<p:sp><p:nvSpPr><p:cNvPr id=\"{id}\" name=\"{} {id}\"/><p:cNvSpPr txBox=\"1\"/><p:nvPr/></p:nvSpPr><p:spPr><a:xfrm><a:off x=\"{}\" y=\"{}\"/><a:ext cx=\"{}\" cy=\"{}\"/></a:xfrm><a:prstGeom prst=\"rect\"><a:avLst/></a:prstGeom><a:noFill/></p:spPr><p:txBody><a:bodyPr wrap=\"square\" lIns=\"0\" tIns=\"0\" rIns=\"0\" bIns=\"0\" anchor=\"{anchor}\"><a:spAutoFit/></a:bodyPr><a:lstStyle/>{body}</p:txBody></p:sp>",
            if title { "Title" } else { "Text" },
            emu(x),
            emu(y),
            emu(w),
            emu(h)
        );
    }

    fn picture(&mut self, x: f32, y: f32, w: f32, h: f32, p: &super::Picture) {
        *self.media_no += 1;
        let name = format!("image{}.{}", self.media_no, if p.ext == "jpeg" { "jpg" } else { "png" });
        self.slide.media.push((format!("ppt/media/{name}"), p.data.clone()));
        let rid = self.rel("http://schemas.openxmlformats.org/officeDocument/2006/relationships/image", &format!("../media/{name}"), false);
        let id = self.id();
        let _ = write!(
            self.slide.xml,
            "<p:pic><p:nvPicPr><p:cNvPr id=\"{id}\" name=\"Picture {id}\"/><p:cNvPicPr><a:picLocks noChangeAspect=\"1\"/></p:cNvPicPr><p:nvPr/></p:nvPicPr><p:blipFill><a:blip r:embed=\"{rid}\"/><a:stretch><a:fillRect/></a:stretch></p:blipFill><p:spPr><a:xfrm><a:off x=\"{}\" y=\"{}\"/><a:ext cx=\"{}\" cy=\"{}\"/></a:xfrm><a:prstGeom prst=\"rect\"><a:avLst/></a:prstGeom></p:spPr></p:pic>",
            emu(x),
            emu(y),
            emu(w),
            emu(h)
        );
    }

    #[allow(clippy::too_many_arguments)]
    fn table(&mut self, x: f32, y: f32, w: f32, h: f32, aligns: &[Align], header: &[Vec<Run>], rows: &[Vec<Vec<Run>>], size: f32) {
        let n = header.len().max(1);
        let col = emu(w / n as f32);
        let row_h = emu(h / (rows.len() + 1) as f32);
        let id = self.id();
        let mut t = format!(
            "<p:graphicFrame><p:nvGraphicFramePr><p:cNvPr id=\"{id}\" name=\"Table {id}\"/><p:cNvGraphicFramePr><a:graphicFrameLocks noGrp=\"1\"/></p:cNvGraphicFramePr><p:nvPr/></p:nvGraphicFramePr><p:xfrm><a:off x=\"{}\" y=\"{}\"/><a:ext cx=\"{}\" cy=\"{}\"/></p:xfrm><a:graphic><a:graphicData uri=\"http://schemas.openxmlformats.org/drawingml/2006/table\"><a:tbl><a:tblPr firstRow=\"1\" bandRow=\"1\"/><a:tblGrid>",
            emu(x),
            emu(y),
            col * n as i64,
            row_h * (rows.len() as i64 + 1)
        );
        for _ in 0..n {
            let _ = write!(t, "<a:gridCol w=\"{col}\"/>");
        }
        t.push_str("</a:tblGrid>");
        let line = |side: &str| format!("<a:{side} w=\"9525\"><a:solidFill><a:srgbClr val=\"9AA4AE\"/></a:solidFill></a:{side}>");
        let borders = format!("{}{}{}{}", line("lnL"), line("lnR"), line("lnT"), line("lnB"));
        let all: Vec<(bool, &[Vec<Run>])> = std::iter::once((true, header)).chain(rows.iter().map(|r| (false, r.as_slice()))).collect();
        let fg = self.fg.clone();
        for (head, cells) in all {
            let _ = write!(t, "<a:tr h=\"{row_h}\">");
            for i in 0..n {
                let mut rs = cells.get(i).cloned().unwrap_or_default();
                if head {
                    for r in rs.iter_mut() {
                        if let Run::Text(_, s) = r {
                            s.bold = true;
                        }
                    }
                }
                let algn = match aligns.get(i) {
                    Some(Align::Center) => " algn=\"ctr\"",
                    Some(Align::Right) => " algn=\"r\"",
                    _ => "",
                };
                let body = self.runs(&rs, size, &fg, false);
                let fill = if head {
                    "<a:solidFill><a:srgbClr val=\"808080\"><a:alpha val=\"25000\"/></a:srgbClr></a:solidFill>"
                } else {
                    "<a:noFill/>"
                };
                let _ = write!(
                    t,
                    "<a:tc><a:txBody><a:bodyPr/><a:lstStyle/><a:p><a:pPr{algn}/>{body}<a:endParaRPr lang=\"en-GB\" sz=\"{}\"/></a:p></a:txBody><a:tcPr marL=\"68580\" marR=\"68580\" marT=\"34290\" marB=\"34290\">{borders}{fill}</a:tcPr></a:tc>",
                    (size * 100.0) as i32
                );
            }
            t.push_str("</a:tr>");
        }
        t.push_str("</a:tbl></a:graphicData></a:graphic></p:graphicFrame>");
        self.slide.xml.push_str(&t);
    }
}

fn theme(name: &str) -> String {
    format!(
        "{HEAD}<a:theme xmlns:a=\"http://schemas.openxmlformats.org/drawingml/2006/main\" name=\"{name}\"><a:themeElements><a:clrScheme name=\"Kadee\"><a:dk1><a:srgbClr val=\"1F2328\"/></a:dk1><a:lt1><a:srgbClr val=\"FFFFFF\"/></a:lt1><a:dk2><a:srgbClr val=\"44546A\"/></a:dk2><a:lt2><a:srgbClr val=\"E7E6E6\"/></a:lt2><a:accent1><a:srgbClr val=\"2F5DD6\"/></a:accent1><a:accent2><a:srgbClr val=\"E8743B\"/></a:accent2><a:accent3><a:srgbClr val=\"19A979\"/></a:accent3><a:accent4><a:srgbClr val=\"ED4A7B\"/></a:accent4><a:accent5><a:srgbClr val=\"945ECF\"/></a:accent5><a:accent6><a:srgbClr val=\"13A4B4\"/></a:accent6><a:hlink><a:srgbClr val=\"0969DA\"/></a:hlink><a:folHlink><a:srgbClr val=\"8250DF\"/></a:folHlink></a:clrScheme><a:fontScheme name=\"Kadee\"><a:majorFont><a:latin typeface=\"Calibri\"/><a:ea typeface=\"\"/><a:cs typeface=\"\"/></a:majorFont><a:minorFont><a:latin typeface=\"Calibri\"/><a:ea typeface=\"\"/><a:cs typeface=\"\"/></a:minorFont></a:fontScheme><a:fmtScheme name=\"Kadee\"><a:fillStyleLst><a:solidFill><a:schemeClr val=\"phClr\"/></a:solidFill><a:solidFill><a:schemeClr val=\"phClr\"/></a:solidFill><a:solidFill><a:schemeClr val=\"phClr\"/></a:solidFill></a:fillStyleLst><a:lnStyleLst><a:ln w=\"6350\"><a:solidFill><a:schemeClr val=\"phClr\"/></a:solidFill></a:ln><a:ln w=\"12700\"><a:solidFill><a:schemeClr val=\"phClr\"/></a:solidFill></a:ln><a:ln w=\"19050\"><a:solidFill><a:schemeClr val=\"phClr\"/></a:solidFill></a:ln></a:lnStyleLst><a:effectStyleLst><a:effectStyle><a:effectLst/></a:effectStyle><a:effectStyle><a:effectLst/></a:effectStyle><a:effectStyle><a:effectLst/></a:effectStyle></a:effectStyleLst><a:bgFillStyleLst><a:solidFill><a:schemeClr val=\"phClr\"/></a:solidFill><a:solidFill><a:schemeClr val=\"phClr\"/></a:solidFill><a:solidFill><a:schemeClr val=\"phClr\"/></a:solidFill></a:bgFillStyleLst></a:fmtScheme></a:themeElements><a:objectDefaults/><a:extraClrSchemeLst/></a:theme>"
    )
}

pub fn write(doc: &Doc) -> Result<Vec<u8>, String> {
    let slides = slides_of(doc);
    let font = doc.theme.map(|t| t.font.to_string()).filter(|f| f != "system-ui").unwrap_or_else(|| "Calibri".into());
    let font = doc.meta("mainfont").map(str::to_string).unwrap_or(font);
    let mono = doc.meta("monofont").unwrap_or("Consolas").to_string();
    let mut files: Vec<(String, Vec<u8>)> = Vec::new();
    let mut media_no = 0usize;
    let mut overrides = String::new();
    let mut pres_rels = String::new();
    let mut sld_ids = String::new();
    for (i, s) in slides.iter().enumerate() {
        let n = i + 1;
        let (bg, fg, head) = doc.slide_colors(s);
        let mut slide = Slide { xml: String::new(), rels: Vec::new(), media: Vec::new() };
        {
            let mut ctx = Ctx {
                fg: fg.clone(),
                head,
                font: font.clone(),
                mono: mono.clone(),
                slide: &mut slide,
                next_id: 1,
                media_no: &mut media_no,
            };
            for b in flow::slide_layout(doc, s) {
                match b {
                    SlideBox::Text { x, y, w, h, paras, size, center, title } => ctx.text_box(x, y, w, h, &paras, size, center, title),
                    SlideBox::Picture { x, y, w, h, picture } => ctx.picture(x, y, w, h, &picture),
                    SlideBox::Table { x, y, w, h, item: Item::Table { aligns, header, rows }, size } => {
                        ctx.table(x, y, w, h, &aligns, &header, &rows, size)
                    }
                    SlideBox::Table { .. } => {}
                }
            }
        }
        let xml = format!(
            "{HEAD}<p:sld {NS}><p:cSld><p:bg><p:bgPr><a:solidFill><a:srgbClr val=\"{bg}\"/></a:solidFill><a:effectLst/></p:bgPr></p:bg><p:spTree>{GRP}{}</p:spTree></p:cSld><p:clrMapOvr><a:masterClrMapping/></p:clrMapOvr></p:sld>",
            slide.xml
        );
        let mut rels = format!(
            "{HEAD}<Relationships xmlns=\"http://schemas.openxmlformats.org/package/2006/relationships\"><Relationship Id=\"rId1\" Type=\"http://schemas.openxmlformats.org/officeDocument/2006/relationships/slideLayout\" Target=\"../slideLayouts/slideLayout1.xml\"/><Relationship Id=\"rId2\" Type=\"http://schemas.openxmlformats.org/officeDocument/2006/relationships/notesSlide\" Target=\"../notesSlides/notesSlide{n}.xml\"/>"
        );
        for (id, kind, target, ext) in &slide.rels {
            let mode = if *ext { " TargetMode=\"External\"" } else { "" };
            let _ = write!(rels, "<Relationship Id=\"{id}\" Type=\"{kind}\" Target=\"{}\"{mode}/>", esc(target));
        }
        rels.push_str("</Relationships>");
        files.push((format!("ppt/slides/slide{n}.xml"), xml.into_bytes()));
        files.push((format!("ppt/slides/_rels/slide{n}.xml.rels"), rels.into_bytes()));
        files.extend(slide.media);
        // speaker notes
        let notes: String = s
            .notes
            .as_deref()
            .unwrap_or("")
            .split('\n')
            .map(|l| format!("<a:p><a:r><a:rPr lang=\"en-GB\"/><a:t>{}</a:t></a:r></a:p>", esc(l)))
            .collect();
        let notes_xml = format!(
            "{HEAD}<p:notes {NS}><p:cSld><p:spTree>{GRP}<p:sp><p:nvSpPr><p:cNvPr id=\"2\" name=\"Notes\"/><p:cNvSpPr><a:spLocks noGrp=\"1\"/></p:cNvSpPr><p:nvPr><p:ph type=\"body\" idx=\"1\"/></p:nvPr></p:nvSpPr><p:spPr/><p:txBody><a:bodyPr/><a:lstStyle/>{}</p:txBody></p:sp></p:spTree></p:cSld><p:clrMapOvr><a:masterClrMapping/></p:clrMapOvr></p:notes>",
            if notes.is_empty() { "<a:p><a:endParaRPr lang=\"en-GB\"/></a:p>".to_string() } else { notes }
        );
        files.push((format!("ppt/notesSlides/notesSlide{n}.xml"), notes_xml.into_bytes()));
        files.push((
            format!("ppt/notesSlides/_rels/notesSlide{n}.xml.rels"),
            format!("{HEAD}<Relationships xmlns=\"http://schemas.openxmlformats.org/package/2006/relationships\"><Relationship Id=\"rId1\" Type=\"http://schemas.openxmlformats.org/officeDocument/2006/relationships/notesMaster\" Target=\"../notesMasters/notesMaster1.xml\"/><Relationship Id=\"rId2\" Type=\"http://schemas.openxmlformats.org/officeDocument/2006/relationships/slide\" Target=\"../slides/slide{n}.xml\"/></Relationships>").into_bytes(),
        ));
        let _ = write!(
            overrides,
            "<Override PartName=\"/ppt/slides/slide{n}.xml\" ContentType=\"application/vnd.openxmlformats-officedocument.presentationml.slide+xml\"/><Override PartName=\"/ppt/notesSlides/notesSlide{n}.xml\" ContentType=\"application/vnd.openxmlformats-officedocument.presentationml.notesSlide+xml\"/>"
        );
        let _ = write!(
            pres_rels,
            "<Relationship Id=\"rId{}\" Type=\"http://schemas.openxmlformats.org/officeDocument/2006/relationships/slide\" Target=\"slides/slide{n}.xml\"/>",
            10 + i
        );
        let _ = write!(sld_ids, "<p:sldId id=\"{}\" r:id=\"rId{}\"/>", 256 + i, 10 + i);
    }
    let presentation = format!(
        "{HEAD}<p:presentation {NS} saveSubsetFonts=\"1\"><p:sldMasterIdLst><p:sldMasterId id=\"2147483648\" r:id=\"rId1\"/></p:sldMasterIdLst><p:notesMasterIdLst><p:notesMasterId r:id=\"rId6\"/></p:notesMasterIdLst><p:sldIdLst>{sld_ids}</p:sldIdLst><p:sldSz cx=\"{}\" cy=\"{}\"/><p:notesSz cx=\"6858000\" cy=\"9144000\"/></p:presentation>",
        emu(SLIDE_W),
        emu(SLIDE_H)
    );
    let rel = |id: &str, kind: &str, target: &str| {
        format!(
            "<Relationship Id=\"{id}\" Type=\"http://schemas.openxmlformats.org/officeDocument/2006/relationships/{kind}\" Target=\"{target}\"/>"
        )
    };
    let pres_rels = format!(
        "{HEAD}<Relationships xmlns=\"http://schemas.openxmlformats.org/package/2006/relationships\">{}{}{}{}{}{}{pres_rels}</Relationships>",
        rel("rId1", "slideMaster", "slideMasters/slideMaster1.xml"),
        rel("rId2", "theme", "theme/theme1.xml"),
        rel("rId3", "presProps", "presProps.xml"),
        rel("rId4", "viewProps", "viewProps.xml"),
        rel("rId5", "tableStyles", "tableStyles.xml"),
        rel("rId6", "notesMaster", "notesMasters/notesMaster1.xml"),
    );
    let master = format!(
        "{HEAD}<p:sldMaster {NS}><p:cSld><p:bg><p:bgRef idx=\"1001\"><a:schemeClr val=\"bg1\"/></p:bgRef></p:bg><p:spTree>{GRP}</p:spTree></p:cSld>{CLRMAP}<p:sldLayoutIdLst><p:sldLayoutId id=\"2147483649\" r:id=\"rId1\"/></p:sldLayoutIdLst><p:txStyles><p:titleStyle/><p:bodyStyle/><p:otherStyle/></p:txStyles></p:sldMaster>"
    );
    let layout = format!(
        "{HEAD}<p:sldLayout {NS} type=\"blank\" preserve=\"1\"><p:cSld name=\"Blank\"><p:spTree>{GRP}</p:spTree></p:cSld><p:clrMapOvr><a:masterClrMapping/></p:clrMapOvr></p:sldLayout>"
    );
    let notes_master = format!(
        "{HEAD}<p:notesMaster {NS}><p:cSld><p:bg><p:bgRef idx=\"1001\"><a:schemeClr val=\"bg1\"/></p:bgRef></p:bg><p:spTree>{GRP}<p:sp><p:nvSpPr><p:cNvPr id=\"2\" name=\"Notes Placeholder\"/><p:cNvSpPr><a:spLocks noGrp=\"1\"/></p:cNvSpPr><p:nvPr><p:ph type=\"body\" sz=\"quarter\" idx=\"1\"/></p:nvPr></p:nvSpPr><p:spPr><a:xfrm><a:off x=\"685800\" y=\"4400550\"/><a:ext cx=\"5486400\" cy=\"3600450\"/></a:xfrm><a:prstGeom prst=\"rect\"><a:avLst/></a:prstGeom></p:spPr><p:txBody><a:bodyPr/><a:lstStyle/><a:p><a:endParaRPr lang=\"en-GB\"/></a:p></p:txBody></p:sp></p:spTree></p:cSld>{CLRMAP}</p:notesMaster>"
    );
    let rels_doc = |inner: String| {
        format!("{HEAD}<Relationships xmlns=\"http://schemas.openxmlformats.org/package/2006/relationships\">{inner}</Relationships>")
    };
    let now = super::now_iso();
    let core = format!(
        "{HEAD}<cp:coreProperties xmlns:cp=\"http://schemas.openxmlformats.org/package/2006/metadata/core-properties\" xmlns:dc=\"http://purl.org/dc/elements/1.1/\" xmlns:dcterms=\"http://purl.org/dc/terms/\" xmlns:xsi=\"http://www.w3.org/2001/XMLSchema-instance\"><dc:title>{}</dc:title><dc:creator>{}</dc:creator><dcterms:created xsi:type=\"dcterms:W3CDTF\">{now}</dcterms:created><dcterms:modified xsi:type=\"dcterms:W3CDTF\">{now}</dcterms:modified></cp:coreProperties>",
        esc(&doc.title),
        esc(&doc.author)
    );
    let ct = |part: &str, kind: &str| {
        format!("<Override PartName=\"{part}\" ContentType=\"application/vnd.openxmlformats-officedocument.{kind}+xml\"/>")
    };
    let content_types = format!(
        "{HEAD}<Types xmlns=\"http://schemas.openxmlformats.org/package/2006/content-types\"><Default Extension=\"rels\" ContentType=\"application/vnd.openxmlformats-package.relationships+xml\"/><Default Extension=\"xml\" ContentType=\"application/xml\"/><Default Extension=\"png\" ContentType=\"image/png\"/><Default Extension=\"jpg\" ContentType=\"image/jpeg\"/>{}{}{}{}{}{}{}{}{}{}<Override PartName=\"/docProps/core.xml\" ContentType=\"application/vnd.openxmlformats-package.core-properties+xml\"/>{overrides}</Types>",
        ct("/ppt/presentation.xml", "presentationml.presentation.main"),
        ct("/ppt/slideMasters/slideMaster1.xml", "presentationml.slideMaster"),
        ct("/ppt/slideLayouts/slideLayout1.xml", "presentationml.slideLayout"),
        ct("/ppt/notesMasters/notesMaster1.xml", "presentationml.notesMaster"),
        ct("/ppt/theme/theme1.xml", "theme"),
        ct("/ppt/theme/theme2.xml", "theme"),
        ct("/ppt/presProps.xml", "presentationml.presProps"),
        ct("/ppt/viewProps.xml", "presentationml.viewProps"),
        ct("/ppt/tableStyles.xml", "presentationml.tableStyles"),
        ct("/docProps/app.xml", "extended-properties"),
    );
    let mut all: Vec<(String, Vec<u8>)> = vec![
        ("[Content_Types].xml".into(), content_types.into_bytes()),
        (
            "_rels/.rels".into(),
            rels_doc(format!(
                "{}<Relationship Id=\"rId2\" Type=\"http://schemas.openxmlformats.org/package/2006/relationships/metadata/core-properties\" Target=\"docProps/core.xml\"/>{}",
                rel("rId1", "officeDocument", "ppt/presentation.xml"),
                rel("rId3", "extended-properties", "docProps/app.xml")
            ))
            .into_bytes(),
        ),
        ("docProps/core.xml".into(), core.into_bytes()),
        (
            "docProps/app.xml".into(),
            format!("{HEAD}<Properties xmlns=\"http://schemas.openxmlformats.org/officeDocument/2006/extended-properties\"><Application>Kadee Markdown</Application><Slides>{}</Slides></Properties>", slides.len()).into_bytes(),
        ),
        ("ppt/presentation.xml".into(), presentation.into_bytes()),
        ("ppt/_rels/presentation.xml.rels".into(), pres_rels.into_bytes()),
        ("ppt/slideMasters/slideMaster1.xml".into(), master.into_bytes()),
        (
            "ppt/slideMasters/_rels/slideMaster1.xml.rels".into(),
            rels_doc(format!("{}{}", rel("rId1", "slideLayout", "../slideLayouts/slideLayout1.xml"), rel("rId2", "theme", "../theme/theme1.xml"))).into_bytes(),
        ),
        ("ppt/slideLayouts/slideLayout1.xml".into(), layout.into_bytes()),
        ("ppt/slideLayouts/_rels/slideLayout1.xml.rels".into(), rels_doc(rel("rId1", "slideMaster", "../slideMasters/slideMaster1.xml")).into_bytes()),
        ("ppt/notesMasters/notesMaster1.xml".into(), notes_master.into_bytes()),
        ("ppt/notesMasters/_rels/notesMaster1.xml.rels".into(), rels_doc(rel("rId1", "theme", "../theme/theme2.xml")).into_bytes()),
        ("ppt/theme/theme1.xml".into(), theme("Kadee").into_bytes()),
        ("ppt/theme/theme2.xml".into(), theme("Kadee notes").into_bytes()),
        ("ppt/presProps.xml".into(), format!("{HEAD}<p:presentationPr {NS}/>").into_bytes()),
        (
            "ppt/viewProps.xml".into(),
            format!("{HEAD}<p:viewPr {NS}><p:normalViewPr><p:restoredLeft sz=\"15620\"/><p:restoredTop sz=\"94660\"/></p:normalViewPr><p:gridSpacing cx=\"76200\" cy=\"76200\"/></p:viewPr>").into_bytes(),
        ),
        (
            "ppt/tableStyles.xml".into(),
            format!("{HEAD}<a:tblStyleLst xmlns:a=\"http://schemas.openxmlformats.org/drawingml/2006/main\" def=\"{{5C22544A-7EE6-4342-B048-85BDC9FD1C3A}}\"/>").into_bytes(),
        ),
    ];
    all.extend(files);
    package(&all, &[])
}
