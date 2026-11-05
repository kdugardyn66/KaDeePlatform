//! SVG → a GPUI image at twice the size (sharp on Retina screens), in GPUI's BGRA order.

use std::sync::{Arc, LazyLock};

use gpui::RenderImage;

static FONTS: LazyLock<Arc<resvg::usvg::fontdb::Database>> = LazyLock::new(|| {
    let mut db = resvg::usvg::fontdb::Database::new();
    db.load_system_fonts();
    Arc::new(db)
});

/// Draw `svg` at `scale` × its size.
pub fn svg_image(svg: &str, scale: f32) -> Option<Arc<RenderImage>> {
    let opts = resvg::usvg::Options { fontdb: FONTS.clone(), ..Default::default() };
    let tree = resvg::usvg::Tree::from_str(svg, &opts).ok()?;
    let size = tree.size();
    let (w, h) = ((size.width() * scale).ceil() as u32, (size.height() * scale).ceil() as u32);
    let mut pixmap = resvg::tiny_skia::Pixmap::new(w.max(1), h.max(1))?;
    resvg::render(&tree, resvg::tiny_skia::Transform::from_scale(scale, scale), &mut pixmap.as_mut());
    let mut data = pixmap.take();
    // premultiplied RGBA → straight BGRA
    for px in data.chunks_exact_mut(4) {
        let a = px[3] as u32;
        if a > 0 && a < 255 {
            for c in &mut px[..3] {
                *c = ((*c as u32 * 255 + a / 2) / a).min(255) as u8;
            }
        }
        px.swap(0, 2);
    }
    let buffer = image::ImageBuffer::from_raw(w.max(1), h.max(1), data)?;
    Some(Arc::new(RenderImage::new(vec![image::Frame::new(buffer)])))
}

#[cfg(test)]
mod tests {
    #[test]
    fn draws_red_as_red() {
        let img = super::svg_image(
            "<svg xmlns=\"http://www.w3.org/2000/svg\" width=\"4\" height=\"4\"><rect width=\"4\" height=\"4\" fill=\"#ff0000\"/></svg>",
            2.0,
        )
        .expect("image");
        let bytes = img.as_bytes(0).expect("frame");
        // BGRA: blue 0, green 0, red 255
        assert_eq!(&bytes[..4], &[0, 0, 255, 255]);
    }
}
