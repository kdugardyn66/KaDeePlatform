//! Writes every library diagram and chart that is drawn natively as an SVG file into the folder
//! given on the command line (to look at them).
use kd_formats::md::{diagram, library};

fn main() {
    let dir = std::env::args().nth(1).unwrap_or_else(|| ".".into());
    for s in library::DIAGRAMS.iter().chain(library::CHARTS) {
        if let Some(svg) = diagram::render(s.lang, s.code, false) {
            std::fs::write(format!("{dir}/{}.svg", s.id), svg.svg).expect("write");
        }
    }
}
