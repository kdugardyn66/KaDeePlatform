//! Page setup from the front matter, as Pandoc writes it:
//!
//! ```yaml
//! geometry: a4paper, margin=2cm, landscape
//! geometry: letterpaper, top=2cm, bottom=2.5cm, left=3cm, right=2cm
//! ```
//!
//! The older keys `paper` (a4, letter, legal, a5), `orientation` and `margins` (narrow,
//! normal, wide) still count when there is no `geometry`.

/// The paper sizes offered: (Pandoc name, label, width × height in points, portrait).
pub const PAPERS: [(&str, &str, f32, f32); 6] = [
    ("a4paper", "A4 (210 × 297 mm)", 595.3, 841.9),
    ("letterpaper", "Letter (8.5 × 11 in)", 612.0, 792.0),
    ("legalpaper", "Legal (8.5 × 14 in)", 612.0, 1008.0),
    ("a5paper", "A5 (148 × 210 mm)", 419.5, 595.3),
    ("b5paper", "B5 (176 × 250 mm)", 498.9, 708.7),
    ("executivepaper", "Executive (7.25 × 10.5 in)", 522.0, 756.0),
];

/// Margins in points.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Margins {
    pub top: f32,
    pub bottom: f32,
    pub left: f32,
    pub right: f32,
}

impl Margins {
    pub fn all(v: f32) -> Margins {
        Margins { top: v, bottom: v, left: v, right: v }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct PageSetup {
    /// Pandoc's name (`a4paper` …).
    pub paper: String,
    pub landscape: bool,
    pub margins: Margins,
}

pub const DEFAULT_MARGIN: f32 = 70.9; // 2.5 cm

impl Default for PageSetup {
    fn default() -> Self {
        PageSetup { paper: "a4paper".into(), landscape: false, margins: Margins::all(DEFAULT_MARGIN) }
    }
}

/// A length (`2cm`, `20mm`, `1in`, `72pt`) in points.
pub fn points(v: &str) -> Option<f32> {
    super::para::css_cm(v).map(|cm| cm / 2.54 * 72.0)
}

fn pt_to_cm(pt: f32) -> String {
    let cm = (pt / 72.0 * 2.54 * 100.0).round() / 100.0;
    format!("{cm}cm")
}

impl PageSetup {
    /// From the front matter (`get` reads a key).
    pub fn from_front(get: &dyn Fn(&str) -> Option<String>) -> PageSetup {
        let mut p = PageSetup::default();
        match get("geometry") {
            Some(g) => {
                for part in g.trim_matches(|c| c == '[' || c == ']' || c == '"').split(',') {
                    let part = part.trim().trim_matches('"');
                    match part.split_once('=') {
                        Some((k, v)) => {
                            let Some(v) = points(v.trim()) else { continue };
                            match k.trim() {
                                "margin" => p.margins = Margins::all(v),
                                "top" => p.margins.top = v,
                                "bottom" => p.margins.bottom = v,
                                "left" | "inner" => p.margins.left = v,
                                "right" | "outer" => p.margins.right = v,
                                "hmargin" => {
                                    p.margins.left = v;
                                    p.margins.right = v;
                                }
                                "vmargin" => {
                                    p.margins.top = v;
                                    p.margins.bottom = v;
                                }
                                _ => {}
                            }
                        }
                        None => match part {
                            "landscape" => p.landscape = true,
                            "portrait" => p.landscape = false,
                            name if PAPERS.iter().any(|(n, ..)| *n == name) => p.paper = name.to_string(),
                            _ => {}
                        },
                    }
                }
            }
            None => {
                if let Some(paper) = get("paper") {
                    p.paper = match paper.as_str() {
                        "letter" => "letterpaper",
                        "legal" => "legalpaper",
                        "a5" => "a5paper",
                        "b5" => "b5paper",
                        "executive" => "executivepaper",
                        _ => "a4paper",
                    }
                    .into();
                }
                p.landscape = get("orientation").as_deref() == Some("landscape");
                p.margins = Margins::all(match get("margins").as_deref() {
                    Some("narrow") => 36.0,
                    Some("wide") => 99.2,
                    _ => DEFAULT_MARGIN,
                });
            }
        }
        p
    }

    /// Width and height in points (turned for landscape).
    pub fn size(&self) -> (f32, f32) {
        let (w, h) = PAPERS.iter().find(|(n, ..)| *n == self.paper).map(|(_, _, w, h)| (*w, *h)).unwrap_or((595.3, 841.9));
        if self.landscape { (h, w) } else { (w, h) }
    }

    /// As Pandoc's `geometry` value.
    pub fn to_geometry(&self) -> String {
        let mut parts = vec![self.paper.clone()];
        let m = self.margins;
        if m.top == m.bottom && m.bottom == m.left && m.left == m.right {
            parts.push(format!("margin={}", pt_to_cm(m.top)));
        } else {
            parts.push(format!("top={}", pt_to_cm(m.top)));
            parts.push(format!("bottom={}", pt_to_cm(m.bottom)));
            parts.push(format!("left={}", pt_to_cm(m.left)));
            parts.push(format!("right={}", pt_to_cm(m.right)));
        }
        if self.landscape {
            parts.push("landscape".into());
        }
        parts.join(", ")
    }

    /// The CSS `@page` size (`A4 landscape`, `176mm 250mm` …).
    pub fn css_size(&self) -> String {
        let (w, h) = self.size();
        format!("{:.1}mm {:.1}mm", w / 72.0 * 25.4, h / 72.0 * 25.4)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn front<'a>(pairs: &'a [(&'a str, &'a str)]) -> impl Fn(&str) -> Option<String> + 'a {
        move |k| pairs.iter().find(|(a, _)| *a == k).map(|(_, v)| v.to_string())
    }

    #[test]
    fn reads_geometry_and_old_keys() {
        let p = PageSetup::from_front(&front(&[("geometry", "letterpaper, margin=2cm, landscape")]));
        assert_eq!(p.paper, "letterpaper");
        assert!(p.landscape);
        assert!((p.margins.left - 56.69).abs() < 0.1);
        assert_eq!(p.size(), (792.0, 612.0));
        assert_eq!(p.to_geometry(), "letterpaper, margin=2cm, landscape");
        let p = PageSetup::from_front(&front(&[("geometry", "b5paper, top=1in, bottom=2cm, left=3cm, right=2cm")]));
        assert!((p.margins.top - 72.0).abs() < 0.01);
        assert_eq!(p.to_geometry(), "b5paper, top=2.54cm, bottom=2cm, left=3cm, right=2cm");
        let old = PageSetup::from_front(&front(&[("paper", "legal"), ("margins", "narrow")]));
        assert_eq!(old.paper, "legalpaper");
        assert_eq!(old.margins, Margins::all(36.0));
        assert_eq!(PageSetup::from_front(&front(&[])), PageSetup::default());
    }
}
