//! Lists every chart or diagram block in the Markdown files given and whether it is drawn
//! (and writes them to `$SVG_DIR` when it is set).
use kd_formats::md::{self, Block, diagram};

fn walk(blocks: &[Block], file: &str) {
    for b in blocks {
        match b {
            Block::Code { info, text, line, .. } => {
                let first = info.split_whitespace().next().unwrap_or("");
                if ["mermaid", "chart", "chartjs", "vega-lite", "vega", "dot", "graphviz", "plantuml", "wavedrom", "csv"].contains(&first) {
                    let svg = diagram::render(info, text, false);
                    let ok = svg.is_some();
                    if let (Some(svg), Ok(dir)) = (svg, std::env::var("SVG_DIR")) {
                        std::fs::write(format!("{dir}/block-{line}.svg"), svg.svg).expect("write");
                    }
                    let kind = if first == "mermaid" { diagram::mermaid_kind(text) } else { first.to_string() };
                    println!("{file}:{line} {kind:<16} {}", if ok { "drawn" } else { "NOT DRAWN" });
                }
            }
            Block::Quote { blocks, .. } | Block::Callout { blocks, .. } => walk(blocks, file),
            _ => {}
        }
    }
}

fn main() {
    for f in std::env::args().skip(1) {
        let text = std::fs::read_to_string(&f).expect("read");
        walk(&md::parse(&text).blocks, &f);
    }
}
