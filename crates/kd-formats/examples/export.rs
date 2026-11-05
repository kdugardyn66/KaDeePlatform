//! Saves a Markdown file in another format: `cargo run -p kd-formats --example export -- in.md out.docx`.
use kd_formats::export::{Format, Input, export};

fn main() {
    let mut a = std::env::args().skip(1);
    let (src, out) = (a.next().expect("in.md"), a.next().expect("out.ext"));
    let text = std::fs::read_to_string(&src).expect("read");
    let ext = std::path::Path::new(&out).extension().and_then(|e| e.to_str()).unwrap_or("");
    let format = Format::from_extension(ext).expect("a format: docx, odt, pptx, odp or pdf");
    let input = Input { text, base_dir: std::path::Path::new(&src).parent().map(|p| p.to_path_buf()), author: "Kadee".into() };
    std::fs::write(&out, export(format, &input).expect("export")).expect("write");
}
