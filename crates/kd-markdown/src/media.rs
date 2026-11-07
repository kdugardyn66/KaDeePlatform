//! Thumbnails for video cards in the preview: a frame of a local video (with ffmpeg, when it is
//! installed) or the picture YouTube and Vimeo show for a video. Loaded in the background as a
//! GPUI asset; the card shows without a picture until (or unless) one arrives.

use std::path::PathBuf;
use std::process::{Command, Stdio};
use std::sync::Arc;

use gpui::{App, Asset, RenderImage};

/// Where a thumbnail comes from.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum ThumbSource {
    Local(PathBuf),
    YouTube(String),
    Vimeo(String),
}

pub struct Thumb;

impl Asset for Thumb {
    type Source = ThumbSource;
    type Output = Option<Arc<RenderImage>>;

    fn load(source: Self::Source, cx: &mut App) -> impl Future<Output = Self::Output> + Send + 'static {
        let executor = cx.background_executor().clone();
        async move { executor.spawn(async move { load(&source) }).await }
    }
}

fn load(source: &ThumbSource) -> Option<Arc<RenderImage>> {
    let bytes = match source {
        ThumbSource::Local(p) => video_frame(p)?,
        ThumbSource::YouTube(id) => {
            // the large picture first, the standard one always exists
            ["maxresdefault", "hqdefault"].iter().find_map(|q| fetch(&format!("https://i.ytimg.com/vi/{id}/{q}.jpg")))?
        }
        ThumbSource::Vimeo(id) => {
            let json = fetch(&format!("https://vimeo.com/api/oembed.json?url=https%3A%2F%2Fvimeo.com%2F{id}&width=960"))?;
            let v: serde_json::Value = serde_json::from_slice(&json).ok()?;
            fetch(v.get("thumbnail_url")?.as_str()?)?
        }
    };
    to_render_image(&bytes)
}

/// A picture on the web, for the preview.
pub struct WebPicture;

impl Asset for WebPicture {
    type Source = String;
    type Output = Option<Arc<RenderImage>>;

    fn load(url: Self::Source, cx: &mut App) -> impl Future<Output = Self::Output> + Send + 'static {
        let executor = cx.background_executor().clone();
        async move {
            executor
                .spawn(async move {
                    let agent: ureq::Agent =
                        ureq::Agent::config_builder().timeout_global(Some(std::time::Duration::from_secs(10))).build().into();
                    let mut resp = agent.get(&url).call().ok()?;
                    let bytes = resp.body_mut().with_config().limit(25 * 1024 * 1024).read_to_vec().ok()?;
                    if bytes.starts_with(b"<svg") || bytes.starts_with(b"<?xml") {
                        return crate::raster::svg_image(std::str::from_utf8(&bytes).ok()?, 2.0);
                    }
                    to_render_image(&bytes)
                })
                .await
        }
    }
}

/// A frame one second in (or the first one) as PNG bytes, with ffmpeg.
fn video_frame(path: &std::path::Path) -> Option<Vec<u8>> {
    let ffmpeg = ffmpeg()?;
    for seek in ["1", "0"] {
        let mut cmd = Command::new(&ffmpeg);
        cmd.args(["-hide_banner", "-loglevel", "error", "-ss", seek, "-i"])
            .arg(path)
            .args(["-frames:v", "1", "-vf", "scale=960:-2", "-f", "image2pipe", "-c:v", "png", "-"])
            .stdin(Stdio::null())
            .stderr(Stdio::null());
        #[cfg(windows)]
        {
            use std::os::windows::process::CommandExt;
            cmd.creation_flags(0x0800_0000); // no console window
        }
        if let Ok(out) = cmd.output()
            && out.status.success()
            && !out.stdout.is_empty()
        {
            return Some(out.stdout);
        }
    }
    None
}

/// ffmpeg on the PATH or where package managers put it (apps started from the Dock or Finder
/// do not get the shell's PATH).
fn ffmpeg() -> Option<PathBuf> {
    let name = if cfg!(windows) { "ffmpeg.exe" } else { "ffmpeg" };
    if let Some(paths) = std::env::var_os("PATH") {
        for dir in std::env::split_paths(&paths) {
            let p = dir.join(name);
            if p.is_file() {
                return Some(p);
            }
        }
    }
    ["/opt/homebrew/bin/ffmpeg", "/usr/local/bin/ffmpeg", "/usr/bin/ffmpeg", "/snap/bin/ffmpeg"]
        .iter()
        .map(PathBuf::from)
        .find(|p| p.is_file())
}

/// Whether local videos get a picture.
pub fn has_ffmpeg() -> bool {
    use std::sync::OnceLock;
    static FOUND: OnceLock<bool> = OnceLock::new();
    *FOUND.get_or_init(|| ffmpeg().is_some())
}

fn fetch(url: &str) -> Option<Vec<u8>> {
    let agent: ureq::Agent = ureq::Agent::config_builder().timeout_global(Some(std::time::Duration::from_secs(8))).build().into();
    let mut resp = agent.get(url).call().ok()?;
    if resp.status() != 200 {
        return None;
    }
    let bytes = resp.body_mut().with_config().limit(8 * 1024 * 1024).read_to_vec().ok()?;
    // YouTube answers a missing size with a small grey placeholder
    (bytes.len() > 2000).then_some(bytes)
}

/// PNG or JPEG bytes → a GPUI image (BGRA).
fn to_render_image(bytes: &[u8]) -> Option<Arc<RenderImage>> {
    let img = image::load_from_memory(bytes).ok()?.to_rgba8();
    let (w, h) = img.dimensions();
    let mut data = img.into_raw();
    for px in data.chunks_exact_mut(4) {
        px.swap(0, 2);
    }
    let buffer = image::ImageBuffer::from_raw(w, h, data)?;
    Some(Arc::new(RenderImage::new(vec![image::Frame::new(buffer)])))
}

#[cfg(test)]
mod tests {
    #[test]
    fn decodes_png_to_bgra() {
        let mut png = std::io::Cursor::new(Vec::new());
        image::RgbaImage::from_pixel(2, 2, image::Rgba([255, 0, 0, 255])).write_to(&mut png, image::ImageFormat::Png).expect("png");
        let img = super::to_render_image(png.get_ref()).expect("image");
        assert_eq!(&img.as_bytes(0).expect("frame")[..4], &[0, 0, 255, 255]);
    }
}
