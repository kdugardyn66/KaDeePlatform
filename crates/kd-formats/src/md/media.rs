//! Video, audio and included files in Markdown.
//!
//! A video or audio file is written as a picture whose link is the media file
//! (`![Intro](media/intro.mp4)`), an online video as a picture linking to YouTube or Vimeo, or as
//! a `<video>`, `<audio>` or `<iframe>` tag. `@import "file"` includes a file; it is shown as the
//! file's icon and name, linked to the file (not its contents).

use super::{Block, Inline, Style};

/// What a link points at.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Media {
    /// A video file (local or on the web).
    Video,
    /// An audio file.
    Audio,
    /// A YouTube video: its id.
    YouTube(String),
    /// A Vimeo video: its id.
    Vimeo(String),
}

impl Media {
    pub fn is_audio(&self) -> bool {
        matches!(self, Media::Audio)
    }

    /// The service's name for online videos.
    pub fn service(&self) -> Option<&'static str> {
        match self {
            Media::YouTube(_) => Some("YouTube"),
            Media::Vimeo(_) => Some("Vimeo"),
            _ => None,
        }
    }
}

pub const VIDEO_EXTENSIONS: [&str; 12] = ["mp4", "m4v", "mov", "webm", "mkv", "avi", "ogv", "wmv", "mpg", "mpeg", "3gp", "flv"];
pub const AUDIO_EXTENSIONS: [&str; 10] = ["mp3", "wav", "ogg", "oga", "m4a", "flac", "aac", "opus", "wma", "aiff"];

/// The link without `<…>`, a query or a fragment.
fn bare(url: &str) -> &str {
    let u = url.trim().trim_start_matches('<').trim_end_matches('>');
    u.split(['?', '#']).next().unwrap_or(u)
}

/// The lower-case extension of a link.
pub fn extension(url: &str) -> String {
    let b = bare(url);
    let name = b.rsplit(['/', '\\']).next().unwrap_or(b);
    name.rsplit_once('.').map(|(_, e)| e.to_ascii_lowercase()).unwrap_or_default()
}

/// The YouTube id in a watch, short, embed or youtu.be link.
pub fn youtube_id(url: &str) -> Option<String> {
    let u = url.trim();
    let after = |marker: &str| u.find(marker).map(|p| &u[p + marker.len()..]);
    let id = if u.contains("youtube.com/watch") || u.contains("youtube-nocookie.com/watch") {
        let q = u.split_once('?')?.1;
        q.split('&').find_map(|kv| kv.strip_prefix("v="))?
    } else {
        after("youtu.be/")
            .or_else(|| after("youtube.com/embed/"))
            .or_else(|| after("youtube.com/shorts/"))
            .or_else(|| after("youtube.com/live/"))
            .or_else(|| after("youtube-nocookie.com/embed/"))?
    };
    let id: String = id.chars().take_while(|c| c.is_ascii_alphanumeric() || *c == '-' || *c == '_').collect();
    (id.len() >= 6).then_some(id)
}

/// The Vimeo id in a vimeo.com or player.vimeo.com link.
pub fn vimeo_id(url: &str) -> Option<String> {
    let p = url.find("vimeo.com/")?;
    let rest = &url[p + "vimeo.com/".len()..];
    let rest = rest.strip_prefix("video/").unwrap_or(rest);
    let id: String = rest.chars().take_while(|c| c.is_ascii_digit()).collect();
    (!id.is_empty()).then_some(id)
}

/// Whether a link is a video or audio file, or an online video.
pub fn kind(url: &str) -> Option<Media> {
    if let Some(id) = youtube_id(url) {
        return Some(Media::YouTube(id));
    }
    if let Some(id) = vimeo_id(url) {
        return Some(Media::Vimeo(id));
    }
    let ext = extension(url);
    if VIDEO_EXTENSIONS.contains(&ext.as_str()) {
        Some(Media::Video)
    } else if AUDIO_EXTENSIONS.contains(&ext.as_str()) {
        Some(Media::Audio)
    } else {
        None
    }
}

/// The address an online video plays at inside a page.
pub fn embed_url(m: &Media) -> Option<String> {
    match m {
        Media::YouTube(id) => Some(format!("https://www.youtube-nocookie.com/embed/{id}")),
        Media::Vimeo(id) => Some(format!("https://player.vimeo.com/video/{id}")),
        _ => None,
    }
}

/// The address to open an online video at.
pub fn watch_url(m: &Media, url: &str) -> String {
    match m {
        Media::YouTube(id) => format!("https://www.youtube.com/watch?v={id}"),
        Media::Vimeo(id) => format!("https://vimeo.com/{id}"),
        _ => url.to_string(),
    }
}

/// The file name in a link (`media/intro.mp4` → `intro.mp4`).
pub fn file_name(url: &str) -> String {
    let b = bare(url);
    let name = b.trim_end_matches(['/', '\\']).rsplit(['/', '\\']).next().unwrap_or(b);
    percent_decode(name)
}

fn percent_decode(s: &str) -> String {
    let b = s.as_bytes();
    let mut out = Vec::with_capacity(b.len());
    let mut i = 0;
    while i < b.len() {
        if b[i] == b'%'
            && i + 2 < b.len()
            && let Ok(v) = u8::from_str_radix(&s[i + 1..i + 3], 16)
        {
            out.push(v);
            i += 3;
            continue;
        }
        out.push(b[i]);
        i += 1;
    }
    String::from_utf8_lossy(&out).into_owned()
}

/// The `src` of a `<video>`, `<audio>`, `<source>` or `<iframe>` tag in an HTML block, and
/// whether the tag was `<audio>`.
pub fn html_media(html: &str) -> Option<(String, bool)> {
    let t = html.trim_start();
    let lower = t.to_ascii_lowercase();
    if !(lower.starts_with("<video") || lower.starts_with("<audio") || lower.starts_with("<iframe")) {
        return None;
    }
    let audio = lower.starts_with("<audio");
    let src = attr(t, "src")?;
    Some((src, audio))
}

/// An attribute's value in a tag (`src="…"`, `src='…'` or `src=…`).
fn attr(html: &str, name: &str) -> Option<String> {
    let lower = html.to_ascii_lowercase();
    let mut from = 0;
    while let Some(p) = lower[from..].find(name).map(|p| p + from) {
        let before = lower[..p].chars().last().unwrap_or(' ');
        let rest = html[p + name.len()..].trim_start();
        from = p + name.len();
        if !(before.is_whitespace()) || !rest.starts_with('=') {
            continue;
        }
        let v = rest[1..].trim_start();
        let (q, v) = match v.chars().next() {
            Some(q @ ('"' | '\'')) => (Some(q), &v[1..]),
            _ => (None, v),
        };
        let end = match q {
            Some(q) => v.find(q),
            None => v.find(|c: char| c.is_whitespace() || c == '>'),
        }
        .unwrap_or(v.len());
        return Some(v[..end].to_string());
    }
    None
}

/// `@import "file"` (or `@import file`) on its own: the file.
pub fn import_path(text: &str) -> Option<&str> {
    let r = text.trim().strip_prefix("@import")?.trim();
    let r = r.strip_prefix('"').map(|r| r.split('"').next().unwrap_or(r)).unwrap_or_else(|| r.split_whitespace().next().unwrap_or(r));
    (!r.is_empty()).then_some(r)
}

/// A paragraph that is only an `@import`: the file.
pub fn import_of(inlines: &[Inline]) -> Option<&str> {
    match inlines {
        [Inline::Text(t, _)] => import_path(t),
        _ => None,
    }
}

/// How a file kind is called (`report.pdf` → "PDF document").
pub fn file_kind(url: &str) -> &'static str {
    match extension(url).as_str() {
        "md" | "markdown" => "Markdown document",
        "mdd" => "Markdown document template",
        "mds" | "mdp" => "Markdown slide deck template",
        "pdf" => "PDF document",
        "doc" | "docx" | "odt" | "rtf" | "pages" => "Document",
        "xls" | "xlsx" | "ods" | "numbers" => "Spreadsheet",
        "csv" | "tsv" => "Table (CSV)",
        "ppt" | "pptx" | "odp" | "key" => "Presentation",
        "txt" | "log" => "Text file",
        "json" | "yaml" | "yml" | "toml" | "xml" => "Data file",
        "png" | "jpg" | "jpeg" | "gif" | "webp" | "svg" | "bmp" | "tif" | "tiff" | "heic" => "Picture",
        "zip" | "7z" | "rar" | "tar" | "gz" => "Archive",
        "html" | "htm" => "Web page",
        "rs" | "py" | "js" | "ts" | "c" | "cpp" | "h" | "java" | "go" | "rb" | "sh" | "swift" | "kt" | "cs" | "sql" => "Source code",
        e if VIDEO_EXTENSIONS.contains(&e) => "Video",
        e if AUDIO_EXTENSIONS.contains(&e) => "Audio",
        _ => "File",
    }
}

fn link_text(text: String, url: &str) -> Inline {
    Inline::Text(text, Style { link: Some(url.trim_start_matches('<').trim_end_matches('>').to_string()), ..Style::default() })
}

/// For formats that cannot play media or include files (Word, OpenDocument, PowerPoint,
/// PDF): videos, audio and included files become links with their name.
pub fn as_links(blocks: &mut [Block]) {
    for b in blocks.iter_mut() {
        match b {
            Block::Paragraph { inlines, line } => {
                if let Some(path) = import_of(inlines) {
                    let path = path.to_string();
                    *b = Block::Paragraph {
                        inlines: vec![link_text(format!("{}: {}", file_kind(&path), file_name(&path)), &path)],
                        line: *line,
                    };
                    continue;
                }
                inline_links(inlines);
            }
            Block::Heading { inlines, .. } => inline_links(inlines),
            Block::List { items, .. } => {
                for it in items {
                    as_links(&mut it.blocks);
                }
            }
            Block::Quote { blocks, .. } | Block::Callout { blocks, .. } | Block::Styled { blocks, .. } => as_links(blocks),
            Block::Table { header, rows, .. } => {
                for c in header.iter_mut() {
                    inline_links(c);
                }
                for r in rows.iter_mut() {
                    for c in r.iter_mut() {
                        inline_links(c);
                    }
                }
            }
            Block::Html { text, line } => {
                if let Some((src, audio)) = html_media(text) {
                    let m = kind(&src).unwrap_or(if audio { Media::Audio } else { Media::Video });
                    let label = media_label(&m, "", &src);
                    *b = Block::Paragraph { inlines: vec![link_text(label, &watch_url(&m, &src))], line: *line };
                }
            }
            _ => {}
        }
    }
}

/// "Video: Intro (intro.mp4)", "YouTube video: Launch", "Audio: podcast.mp3" …
pub fn media_label(m: &Media, alt: &str, url: &str) -> String {
    let what = match m {
        Media::Audio => "Audio".to_string(),
        Media::Video => "Video".to_string(),
        m => format!("{} video", m.service().unwrap_or("Online")),
    };
    let name = if matches!(m, Media::YouTube(_) | Media::Vimeo(_)) { String::new() } else { file_name(url) };
    match (alt.trim().is_empty(), name.is_empty()) {
        (false, false) if alt.trim() != name => format!("{what}: {} ({name})", alt.trim()),
        (false, _) => format!("{what}: {}", alt.trim()),
        (true, false) => format!("{what}: {name}"),
        (true, true) => format!("{what}: {url}"),
    }
}

fn inline_links(inlines: &mut [Inline]) {
    for i in inlines.iter_mut() {
        if let Inline::Image { url, alt, .. } = i
            && let Some(m) = kind(url)
        {
            let label = media_label(&m, alt, url);
            *i = link_text(label, &watch_url(&m, url));
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn recognises_media() {
        assert_eq!(kind("media/intro.mp4"), Some(Media::Video));
        assert_eq!(kind("<my clips/a b.MOV>"), Some(Media::Video));
        assert_eq!(kind("song.mp3?x=1"), Some(Media::Audio));
        assert_eq!(kind("https://www.youtube.com/watch?v=dQw4w9WgXcQ&t=1"), Some(Media::YouTube("dQw4w9WgXcQ".into())));
        assert_eq!(kind("https://youtu.be/dQw4w9WgXcQ"), Some(Media::YouTube("dQw4w9WgXcQ".into())));
        assert_eq!(kind("https://vimeo.com/76979871"), Some(Media::Vimeo("76979871".into())));
        assert_eq!(kind("picture.png"), None);
        assert_eq!(file_name("media/my%20clip.mp4"), "my clip.mp4");
    }

    #[test]
    fn finds_html_media_and_imports() {
        assert_eq!(html_media("<video controls src=\"a.mp4\" width=640></video>"), Some(("a.mp4".into(), false)));
        assert_eq!(
            html_media("<iframe width='560' src='https://www.youtube.com/embed/abcdefg'></iframe>").map(|x| x.0),
            Some("https://www.youtube.com/embed/abcdefg".into())
        );
        assert_eq!(import_path("@import \"notes/a b.md\""), Some("notes/a b.md"));
        assert_eq!(import_path("@import data.csv"), Some("data.csv"));
        assert_eq!(import_path("import x"), None);
    }

    #[test]
    fn exports_get_links() {
        let mut p = crate::md::parse("![Intro](intro.mp4)\n\n@import \"report.pdf\"\n\n<video src=\"b.webm\"></video>\n");
        as_links(&mut p.blocks);
        let texts: Vec<String> = p
            .blocks
            .iter()
            .filter_map(|b| match b {
                Block::Paragraph { inlines, .. } => match inlines.as_slice() {
                    [Inline::Text(t, s)] => Some(format!("{t} -> {}", s.link.clone().unwrap_or_default())),
                    _ => None,
                },
                _ => None,
            })
            .collect();
        assert_eq!(
            texts,
            vec!["Video: Intro (intro.mp4) -> intro.mp4", "PDF document: report.pdf -> report.pdf", "Video: b.webm -> b.webm"]
        );
    }
}
