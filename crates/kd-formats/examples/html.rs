//! Writes a Markdown file as a web page: `cargo run -p kd-formats --example html -- in.md out.html`.
fn main() {
    let mut a = std::env::args().skip(1);
    let (src, out) = (a.next().expect("in.md"), a.next().expect("out.html"));
    let text = std::fs::read_to_string(src).expect("read");
    std::fs::write(out, kd_formats::md::html::to_html(&text, &Default::default())).expect("write");
}
