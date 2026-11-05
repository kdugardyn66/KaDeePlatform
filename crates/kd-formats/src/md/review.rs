//! Review › Check accessibility, and small helpers for tables from CSV files.

use super::parse::{Block, Inline, Parsed, inline_text};

/// One accessibility issue: where it is (1-based line) and what to do.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Issue {
    pub line: usize,
    pub message: String,
}

fn walk<'a>(blocks: &'a [Block], out: &mut Vec<&'a Block>) {
    for b in blocks {
        out.push(b);
        match b {
            Block::List { items, .. } => {
                for it in items {
                    walk(&it.blocks, out);
                }
            }
            Block::Quote { blocks, .. } | Block::Callout { blocks, .. } | Block::Footnote { blocks, .. } => walk(blocks, out),
            _ => {}
        }
    }
}

fn inlines_of(b: &Block) -> Vec<&[Inline]> {
    match b {
        Block::Heading { inlines, .. } | Block::Paragraph { inlines, .. } => vec![inlines.as_slice()],
        Block::Table { header, rows, .. } => header.iter().chain(rows.iter().flatten()).map(Vec::as_slice).collect(),
        _ => Vec::new(),
    }
}

/// The problems a screen-reader user would meet.
pub fn accessibility(p: &Parsed) -> Vec<Issue> {
    let mut all = Vec::new();
    walk(&p.blocks, &mut all);
    let mut issues = Vec::new();
    let mut last_level = 0u8;
    let mut h1 = 0;
    for b in &all {
        let line = b.line();
        if let Block::Heading { level, inlines, .. } = b {
            if *level == 1 {
                h1 += 1;
            }
            if last_level > 0 && *level > last_level + 1 {
                issues.push(Issue { line, message: format!("Heading level skipped: heading {} after heading {last_level}", level) });
            }
            if inline_text(inlines).trim().is_empty() {
                issues.push(Issue { line, message: "Empty heading".into() });
            }
            last_level = *level;
        }
        if let Block::Table { header, .. } = b
            && header.iter().all(|h| inline_text(h).trim().is_empty())
        {
            issues.push(Issue { line, message: "Table without column headers".into() });
        }
        for inl in inlines_of(b) {
            let mut link_text: Option<(String, String)> = None;
            for i in inl {
                match i {
                    Inline::Image { alt, url, .. } if alt.trim().is_empty() => {
                        issues.push(Issue { line, message: format!("Picture without alternative text: {url}") });
                    }
                    Inline::Text(t, st) => {
                        if let Some(u) = &st.link {
                            match &mut link_text {
                                Some((lu, lt)) if lu == u => lt.push_str(t),
                                _ => link_text = Some((u.clone(), t.clone())),
                            }
                        }
                    }
                    _ => {}
                }
            }
            if let Some((_, t)) = link_text {
                let t = t.trim().to_lowercase();
                if matches!(t.as_str(), "here" | "click here" | "link" | "this" | "more" | "read more") {
                    issues.push(Issue { line, message: format!("Link text “{t}” does not say where the link goes") });
                }
            }
        }
    }
    if h1 > 1 {
        issues.push(Issue { line: 1, message: format!("{h1} level-1 headings: a document usually has one title") });
    }
    issues.sort_by_key(|i| i.line);
    issues
}

/// A CSV (or TSV / semicolon) file as a Markdown table.
pub fn csv_to_table(csv: &str) -> String {
    let first = csv.lines().next().unwrap_or("");
    let sep = if first.contains('\t') {
        '\t'
    } else if first.matches(';').count() > first.matches(',').count() {
        ';'
    } else {
        ','
    };
    let rows: Vec<Vec<String>> = csv.lines().filter(|l| !l.trim().is_empty()).map(|l| split_csv(l, sep)).collect();
    let Some(header) = rows.first() else { return String::new() };
    let n = rows.iter().map(Vec::len).max().unwrap_or(0).max(1);
    let cell = |r: &Vec<String>, i: usize| r.get(i).cloned().unwrap_or_default().replace('|', "\\|");
    let line = |r: &Vec<String>| format!("| {} |", (0..n).map(|i| cell(r, i)).collect::<Vec<_>>().join(" | "));
    let numeric = |i: usize| rows.iter().skip(1).all(|r| r.get(i).is_none_or(|c| c.trim().replace([' ', ','], ".").parse::<f64>().is_ok()));
    let mut out =
        vec![line(header), format!("|{}|", (0..n).map(|i| if numeric(i) { " ---: " } else { " --- " }).collect::<Vec<_>>().join("|"))];
    out.extend(rows.iter().skip(1).map(line));
    out.join("\n")
}

/// One CSV line (quoted fields with `""` escapes).
fn split_csv(line: &str, sep: char) -> Vec<String> {
    let mut out = Vec::new();
    let mut cur = String::new();
    let mut quoted = false;
    let mut chars = line.chars().peekable();
    while let Some(c) = chars.next() {
        match c {
            '"' if quoted && chars.peek() == Some(&'"') => {
                cur.push('"');
                chars.next();
            }
            '"' => quoted = !quoted,
            c if c == sep && !quoted => out.push(std::mem::take(&mut cur).trim().to_string()),
            c => cur.push(c),
        }
    }
    out.push(cur.trim().to_string());
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::md::parse;

    #[test]
    fn finds_issues() {
        let p = parse("# A\n\n### C\n\n![](x.png)\n\nSee [here](https://x).\n\n# B\n");
        let msgs: Vec<String> = accessibility(&p).into_iter().map(|i| i.message).collect();
        assert!(msgs.iter().any(|m| m.contains("skipped")));
        assert!(msgs.iter().any(|m| m.contains("alternative text")));
        assert!(msgs.iter().any(|m| m.contains("“here”")));
        assert!(msgs.iter().any(|m| m.contains("2 level-1")));
        assert!(accessibility(&parse("# Fine\n\n## Good\n\n![A cat](c.png)\n")).is_empty());
    }

    #[test]
    fn csv_tables() {
        assert_eq!(
            csv_to_table("Name,Amount\n\"Smith, J\",12\nDoe,3.5"),
            "| Name | Amount |\n| --- | ---: |\n| Smith, J | 12 |\n| Doe | 3.5 |"
        );
    }
}
