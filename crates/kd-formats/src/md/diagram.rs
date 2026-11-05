//! Diagrams and charts drawn natively, as SVG: Chart.js charts (bar, line, pie, doughnut,
//! radar), charts from CSV tables, simple Vega-Lite charts; Mermaid flowcharts, pie charts,
//! sequence, class, state and entity relationship diagrams, Gantt charts, mind maps, timelines,
//! user journeys, quadrant charts and git graphs; Graphviz (DOT); PlantUML sequence, class and
//! mind map diagrams; WaveDrom timing diagrams. Other kinds return `None` (the preview then
//! shows their source).

mod graph;
mod kinds;

use std::collections::HashMap;
use std::fmt::Write as _;

/// A drawn diagram.
#[derive(Debug, Clone, PartialEq)]
pub struct Svg {
    pub svg: String,
    pub width: f32,
    pub height: f32,
}

/// Colours for light or dark pages.
#[derive(Debug, Clone, Copy)]
struct Ink {
    fg: &'static str,
    muted: &'static str,
    grid: &'static str,
    bg: &'static str,
    node: &'static str,
    node_border: &'static str,
}

fn ink(dark: bool) -> Ink {
    if dark {
        Ink { fg: "#e6edf3", muted: "#9198a1", grid: "#30363d", bg: "#0d1117", node: "#1c2a44", node_border: "#6e93f2" }
    } else {
        Ink { fg: "#1f2328", muted: "#59636e", grid: "#e1e4e8", bg: "#ffffff", node: "#eef3fd", node_border: "#2f5dd6" }
    }
}

const PALETTE: [&str; 10] = ["#2f5dd6", "#e8743b", "#19a979", "#ed4a7b", "#945ecf", "#13a4b4", "#525df4", "#bf399e", "#6c8893", "#ee6868"];

/// Draw a fenced block, when its kind is one that is drawn natively.
pub fn render(lang: &str, code: &str, dark: bool) -> Option<Svg> {
    let lang = lang.trim();
    let first = lang.split(|c: char| c.is_whitespace() || c == '{').next().unwrap_or("").to_ascii_lowercase();
    let chart = match first.as_str() {
        "chart" | "chartjs" => chartjs(code),
        "vega-lite" | "vega" => vega_lite(code),
        "csv" if lang.contains("chart=") => csv_chart(lang, code),
        "mermaid" => return mermaid(code, dark),
        "dot" | "graphviz" => return graph::dot(code, dark),
        "plantuml" | "puml" => return kinds::plantuml(code, dark),
        "wavedrom" => return kinds::wavedrom(code, dark),
        _ => None,
    }?;
    Some(draw_chart(&chart, dark))
}

/// Whether this kind of chart or diagram is drawn natively (it may still fail on bad syntax).
pub fn supported(lang: &str, code: &str) -> bool {
    let lang = lang.trim();
    let first = lang.split(|c: char| c.is_whitespace() || c == '{').next().unwrap_or("").to_ascii_lowercase();
    match first.as_str() {
        "chart" | "chartjs" | "vega-lite" | "vega" => true,
        "csv" => lang.contains("chart="),
        "dot" | "graphviz" | "plantuml" | "puml" | "wavedrom" => true,
        "mermaid" => MERMAID_KINDS.contains(&mermaid_kind(code).as_str()),
        _ => false,
    }
}

/// The Mermaid diagram kinds that are drawn.
const MERMAID_KINDS: [&str; 15] = [
    "flowchart",
    "graph",
    "pie",
    "sequenceDiagram",
    "gantt",
    "classDiagram",
    "classDiagram-v2",
    "stateDiagram",
    "stateDiagram-v2",
    "erDiagram",
    "mindmap",
    "timeline",
    "journey",
    "quadrantChart",
    "gitGraph",
];

/// A Mermaid block's lines without front matter (`---` … `---`), comments and blank lines.
fn mermaid_lines(code: &str) -> Vec<&str> {
    let mut out = Vec::new();
    let mut front = false;
    for (i, l) in code.lines().map(str::trim).enumerate() {
        if l == "---" && (i == 0 || front) {
            front = !front;
            continue;
        }
        if front || l.is_empty() || l.starts_with("%%") {
            continue;
        }
        out.push(l);
    }
    out
}

/// The diagram keyword on a Mermaid block's first line (`classDiagram`, `flowchart` …).
pub fn mermaid_kind(code: &str) -> String {
    mermaid_lines(code).first().and_then(|l| l.split_whitespace().next()).unwrap_or("").to_string()
}

// ---- lenient JSON (Chart.js specs are JavaScript objects) ------------------------------------

/// JavaScript object syntax to JSON: unquoted keys, single quotes, trailing commas, comments.
pub fn js_to_json(src: &str) -> String {
    let c: Vec<char> = src.chars().collect();
    let mut out = String::with_capacity(src.len() + 32);
    let mut i = 0;
    while i < c.len() {
        let ch = c[i];
        if ch == '/' && c.get(i + 1) == Some(&'/') {
            while i < c.len() && c[i] != '\n' {
                i += 1;
            }
            continue;
        }
        if ch == '/' && c.get(i + 1) == Some(&'*') {
            i += 2;
            while i + 1 < c.len() && !(c[i] == '*' && c[i + 1] == '/') {
                i += 1;
            }
            i += 2;
            continue;
        }
        if ch == '"' || ch == '\'' {
            let q = ch;
            out.push('"');
            i += 1;
            while i < c.len() && c[i] != q {
                if c[i] == '\\' && i + 1 < c.len() {
                    if c[i + 1] == '\'' {
                        out.push('\'');
                    } else {
                        out.push('\\');
                        out.push(c[i + 1]);
                    }
                    i += 2;
                    continue;
                }
                if c[i] == '"' {
                    out.push_str("\\\"");
                } else {
                    out.push(c[i]);
                }
                i += 1;
            }
            out.push('"');
            i += 1;
            continue;
        }
        if ch.is_alphabetic() || ch == '_' || ch == '$' {
            let start = i;
            while i < c.len() && (c[i].is_alphanumeric() || c[i] == '_' || c[i] == '$') {
                i += 1;
            }
            let word: String = c[start..i].iter().collect();
            let mut j = i;
            while j < c.len() && c[j].is_whitespace() {
                j += 1;
            }
            if c.get(j) == Some(&':') {
                let _ = write!(out, "\"{word}\"");
            } else {
                out.push_str(&word);
            }
            continue;
        }
        out.push(ch);
        i += 1;
    }
    drop_trailing_commas(&out)
}

/// Remove commas that come right before `}` or `]` (outside strings).
fn drop_trailing_commas(s: &str) -> String {
    let c: Vec<char> = s.chars().collect();
    let mut out = String::with_capacity(s.len());
    let mut in_str = false;
    let mut i = 0;
    while i < c.len() {
        let ch = c[i];
        if in_str {
            out.push(ch);
            if ch == '\\' && i + 1 < c.len() {
                out.push(c[i + 1]);
                i += 2;
                continue;
            }
            if ch == '"' {
                in_str = false;
            }
        } else if ch == '"' {
            in_str = true;
            out.push(ch);
        } else if ch == ',' {
            let mut j = i + 1;
            while j < c.len() && c[j].is_whitespace() {
                j += 1;
            }
            if !matches!(c.get(j), Some('}') | Some(']')) {
                out.push(ch);
            }
        } else {
            out.push(ch);
        }
        i += 1;
    }
    out
}

// ---- charts ------------------------------------------------------------------------------------

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ChartKind {
    Bar,
    Line,
    Pie,
    Doughnut,
    Radar,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Dataset {
    pub label: String,
    pub data: Vec<f64>,
    pub color: Option<String>,
    pub colors: Vec<String>,
    pub dashed: bool,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Chart {
    pub kind: ChartKind,
    pub labels: Vec<String>,
    pub datasets: Vec<Dataset>,
    pub title: Option<String>,
    pub horizontal: bool,
    pub stacked: bool,
}

fn value_text(v: &serde_json::Value) -> String {
    match v {
        serde_json::Value::String(s) => s.clone(),
        serde_json::Value::Number(n) => {
            let f = n.as_f64().unwrap_or(0.0);
            if f.fract() == 0.0 { format!("{}", f as i64) } else { format!("{f}") }
        }
        other => other.to_string(),
    }
}

fn color_of(v: Option<&serde_json::Value>) -> (Option<String>, Vec<String>) {
    match v {
        Some(serde_json::Value::String(s)) => (Some(s.clone()), Vec::new()),
        Some(serde_json::Value::Array(a)) => (None, a.iter().filter_map(|x| x.as_str().map(str::to_string)).collect()),
        _ => (None, Vec::new()),
    }
}

/// A Chart.js spec.
pub fn chartjs(code: &str) -> Option<Chart> {
    let v: serde_json::Value = serde_json::from_str(&js_to_json(code)).ok()?;
    let kind = match v.get("type")?.as_str()? {
        "bar" => ChartKind::Bar,
        "line" | "scatter" => ChartKind::Line,
        "pie" => ChartKind::Pie,
        "doughnut" | "polarArea" => ChartKind::Doughnut,
        "radar" => ChartKind::Radar,
        _ => return None,
    };
    let data = v.get("data")?;
    let labels = data.get("labels").and_then(|l| l.as_array()).map(|a| a.iter().map(value_text).collect()).unwrap_or_default();
    let datasets = data
        .get("datasets")?
        .as_array()?
        .iter()
        .map(|d| {
            let fill = d.get("backgroundColor");
            let border = d.get("borderColor");
            let (c1, colors) = color_of(fill);
            let (c2, _) = color_of(border);
            let color = if kind == ChartKind::Line || kind == ChartKind::Radar { c2.or(c1) } else { c1.or(c2) };
            Dataset {
                label: d.get("label").map(value_text).unwrap_or_default(),
                data: d
                    .get("data")
                    .and_then(|a| a.as_array())
                    .map(|a| a.iter().map(|x| x.as_f64().or_else(|| x.get("y").and_then(|y| y.as_f64())).unwrap_or(0.0)).collect())
                    .unwrap_or_default(),
                color,
                colors,
                dashed: d.get("borderDash").is_some(),
            }
        })
        .collect();
    let opts = v.get("options");
    let title =
        opts.and_then(|o| o.pointer("/plugins/title/text").or_else(|| o.pointer("/title/text"))).map(value_text).filter(|t| !t.is_empty());
    let horizontal = opts.and_then(|o| o.get("indexAxis")).and_then(|a| a.as_str()) == Some("y");
    let stacked = opts.and_then(|o| o.pointer("/scales/x/stacked")).and_then(|s| s.as_bool()).unwrap_or(false);
    Some(Chart { kind, labels, datasets, title, horizontal, stacked })
}

/// A ```` ```csv {chart="bar" title="…"} ```` block: the first column labels, the others series.
pub fn csv_chart(info: &str, code: &str) -> Option<Chart> {
    let attr = |k: &str| -> Option<String> {
        let i = info.find(&format!("{k}="))? + k.len() + 1;
        let rest = &info[i..];
        let rest = rest.strip_prefix('"').unwrap_or(rest);
        Some(rest.split(['"', ' ', '}']).next().unwrap_or("").to_string())
    };
    let kind = match attr("chart")?.as_str() {
        "line" => ChartKind::Line,
        "pie" => ChartKind::Pie,
        "doughnut" => ChartKind::Doughnut,
        "radar" => ChartKind::Radar,
        _ => ChartKind::Bar,
    };
    let sep = if code.contains('\t') {
        '\t'
    } else if code.lines().next().unwrap_or("").contains(';') {
        ';'
    } else {
        ','
    };
    let mut rows = code
        .lines()
        .filter(|l| !l.trim().is_empty())
        .map(|l| l.split(sep).map(|c| c.trim().trim_matches('"').to_string()).collect::<Vec<_>>());
    let header = rows.next()?;
    let rows: Vec<Vec<String>> = rows.collect();
    let labels = rows.iter().map(|r| r.first().cloned().unwrap_or_default()).collect();
    let datasets = (1..header.len())
        .map(|c| Dataset {
            label: header[c].clone(),
            data: rows.iter().map(|r| r.get(c).and_then(|x| x.replace(' ', "").parse().ok()).unwrap_or(0.0)).collect(),
            color: None,
            colors: Vec::new(),
            dashed: false,
        })
        .collect();
    Some(Chart { kind, labels, datasets, title: attr("title"), horizontal: false, stacked: false })
}

/// A simple Vega-Lite spec: `mark` bar / line / arc with inline `data.values` and x / y (or
/// theta / color) fields.
pub fn vega_lite(code: &str) -> Option<Chart> {
    let v: serde_json::Value = serde_json::from_str(&js_to_json(code)).ok()?;
    let mark = match v.get("mark")? {
        serde_json::Value::String(s) => s.clone(),
        m => m.get("type")?.as_str()?.to_string(),
    };
    let values = v.pointer("/data/values")?.as_array()?;
    let enc = v.get("encoding")?;
    let (kind, lab, val) = match mark.as_str() {
        "bar" => (ChartKind::Bar, enc.pointer("/x/field")?, enc.pointer("/y/field")?),
        "line" | "area" | "point" => (ChartKind::Line, enc.pointer("/x/field")?, enc.pointer("/y/field")?),
        "arc" => (ChartKind::Doughnut, enc.pointer("/color/field")?, enc.pointer("/theta/field")?),
        _ => return None,
    };
    let (lab, val) = (lab.as_str()?, val.as_str()?);
    let series = if kind == ChartKind::Doughnut { None } else { enc.pointer("/color/field").and_then(|f| f.as_str()) };
    let mut labels: Vec<String> = Vec::new();
    for r in values {
        let l = r.get(lab).map(value_text).unwrap_or_default();
        if !labels.contains(&l) {
            labels.push(l);
        }
    }
    let mut names: Vec<String> = Vec::new();
    if let Some(s) = series {
        for r in values {
            let n = r.get(s).map(value_text).unwrap_or_default();
            if !names.contains(&n) {
                names.push(n);
            }
        }
    } else {
        names.push(val.to_string());
    }
    let datasets = names
        .iter()
        .map(|n| {
            let mut data = vec![0.0; labels.len()];
            for r in values {
                if series.is_some_and(|s| r.get(s).map(value_text).as_deref() != Some(n)) {
                    continue;
                }
                let l = r.get(lab).map(value_text).unwrap_or_default();
                if let Some(i) = labels.iter().position(|x| *x == l) {
                    data[i] += r.get(val).and_then(|x| x.as_f64()).unwrap_or(0.0);
                }
            }
            Dataset { label: n.clone(), data, color: None, colors: Vec::new(), dashed: false }
        })
        .collect();
    let title = v.get("title").map(|t| t.get("text").map(value_text).unwrap_or_else(|| value_text(t)));
    Some(Chart { kind, labels, datasets, title, horizontal: false, stacked: false })
}

fn esc(s: &str) -> String {
    s.replace('&', "&amp;").replace('<', "&lt;").replace('>', "&gt;").replace('"', "&quot;")
}

/// Rough width of `s` in a sans-serif font of `size` px.
fn text_w(s: &str, size: f32) -> f32 {
    s.chars()
        .map(|c| {
            if c.is_uppercase() || c == 'm' || c == 'w' {
                0.68
            } else if " il.,:;|'".contains(c) {
                0.3
            } else {
                0.55
            }
        })
        .sum::<f32>()
        * size
}

/// Tick values for 0..max (or min..max).
fn nice_ticks(min: f64, max: f64) -> Vec<f64> {
    let (lo, hi) = (min.min(0.0), if max <= min.min(0.0) { min.min(0.0) + 1.0 } else { max });
    let span = hi - lo;
    let raw = span / 5.0;
    let mag = 10f64.powf(raw.log10().floor());
    let step = [1.0, 2.0, 2.5, 5.0, 10.0].iter().map(|m| m * mag).find(|s| span / s <= 6.0).unwrap_or(mag * 10.0);
    let start = (lo / step).floor() * step;
    let mut out = Vec::new();
    let mut t = start;
    while t <= hi + step * 0.999 {
        out.push((t * 1e9).round() / 1e9);
        t += step;
    }
    out
}

fn num(v: f64) -> String {
    if v.fract().abs() < 1e-9 {
        format!("{}", v as i64)
    } else {
        let s = format!("{v:.2}");
        s.trim_end_matches('0').trim_end_matches('.').to_string()
    }
}

fn header(w: f32, h: f32, ink: &Ink) -> String {
    format!(
        "<svg xmlns=\"http://www.w3.org/2000/svg\" width=\"{w}\" height=\"{h}\" viewBox=\"0 0 {w} {h}\" font-family=\"Inter, Noto Sans, DejaVu Sans, Helvetica, Arial, sans-serif\" font-size=\"12\" fill=\"{}\">",
        ink.fg
    )
}

/// Draw a chart.
pub fn draw_chart(c: &Chart, dark: bool) -> Svg {
    let ink = ink(dark);
    let (w, h) = (680.0f32, 380.0f32);
    let mut s = header(w, h, &ink);
    let mut top = 14.0;
    if let Some(t) = &c.title {
        let _ = write!(s, "<text x=\"{}\" y=\"22\" text-anchor=\"middle\" font-size=\"15\" font-weight=\"600\">{}</text>", w / 2.0, esc(t));
        top = 36.0;
    }
    let color = |i: usize, d: &Dataset| d.color.clone().unwrap_or_else(|| PALETTE[i % PALETTE.len()].to_string());
    match c.kind {
        ChartKind::Pie | ChartKind::Doughnut => {
            let d = c.datasets.first();
            let vals: Vec<f64> = d.map(|d| d.data.clone()).unwrap_or_default();
            let total: f64 = vals.iter().filter(|v| **v > 0.0).sum();
            let (cx, cy, r) = (220.0f32, top + (h - top) / 2.0, ((h - top) / 2.0 - 16.0).min(160.0));
            let mut a0 = -std::f64::consts::FRAC_PI_2;
            for (i, v) in vals.iter().enumerate() {
                if *v <= 0.0 || total <= 0.0 {
                    continue;
                }
                let frac = v / total;
                let a1 = a0 + frac * std::f64::consts::TAU;
                let col = d.and_then(|d| d.colors.get(i).cloned()).unwrap_or_else(|| PALETTE[i % PALETTE.len()].to_string());
                let (x0, y0) = (cx as f64 + r as f64 * a0.cos(), cy as f64 + r as f64 * a0.sin());
                let (x1, y1) = (cx as f64 + r as f64 * a1.cos(), cy as f64 + r as f64 * a1.sin());
                if frac >= 0.9999 {
                    let _ = write!(s, "<circle cx=\"{cx}\" cy=\"{cy}\" r=\"{r}\" fill=\"{col}\"/>");
                } else {
                    let large = if frac > 0.5 { 1 } else { 0 };
                    let _ = write!(
                        s,
                        "<path d=\"M{cx},{cy} L{x0:.2},{y0:.2} A{r},{r} 0 {large} 1 {x1:.2},{y1:.2} Z\" fill=\"{col}\" stroke=\"{}\" stroke-width=\"2\"/>",
                        ink.bg
                    );
                }
                a0 = a1;
            }
            if c.kind == ChartKind::Doughnut {
                let _ = write!(s, "<circle cx=\"{cx}\" cy=\"{cy}\" r=\"{}\" fill=\"{}\"/>", r * 0.55, ink.bg);
            }
            // legend
            let mut y = cy - (vals.len() as f32 * 22.0) / 2.0 + 8.0;
            for (i, l) in c.labels.iter().enumerate() {
                let col = d.and_then(|d| d.colors.get(i).cloned()).unwrap_or_else(|| PALETTE[i % PALETTE.len()].to_string());
                let pct = vals.get(i).filter(|_| total > 0.0).map(|v| format!(" · {:.0} %", v / total * 100.0)).unwrap_or_default();
                let _ = write!(s, "<rect x=\"430\" y=\"{}\" width=\"12\" height=\"12\" rx=\"2\" fill=\"{col}\"/>", y - 10.0);
                let _ = write!(s, "<text x=\"450\" y=\"{y}\">{}{}</text>", esc(l), esc(&pct));
                y += 22.0;
            }
        }
        ChartKind::Radar => {
            let n = c.labels.len().max(3);
            let (cx, cy) = (w / 2.0, top + (h - top) / 2.0 + 6.0);
            let r = ((h - top) / 2.0 - 30.0).min(150.0);
            let max = c.datasets.iter().flat_map(|d| d.data.iter().copied()).fold(0.0f64, f64::max);
            let ticks = nice_ticks(0.0, max);
            let top_v = ticks.last().copied().unwrap_or(1.0).max(1e-9);
            let pt = |i: usize, v: f64| {
                let a = -std::f64::consts::FRAC_PI_2 + i as f64 * std::f64::consts::TAU / n as f64;
                let rr = r as f64 * v / top_v;
                (cx as f64 + rr * a.cos(), cy as f64 + rr * a.sin())
            };
            for t in &ticks[1..] {
                let p: Vec<String> = (0..n).map(|i| pt(i, *t)).map(|(x, y)| format!("{x:.1},{y:.1}")).collect();
                let _ = write!(s, "<polygon points=\"{}\" fill=\"none\" stroke=\"{}\"/>", p.join(" "), ink.grid);
            }
            for (i, l) in c.labels.iter().enumerate() {
                let (x, y) = pt(i, top_v);
                let _ = write!(s, "<line x1=\"{cx}\" y1=\"{cy}\" x2=\"{x:.1}\" y2=\"{y:.1}\" stroke=\"{}\"/>", ink.grid);
                let (lx, ly) = pt(i, top_v * 1.12);
                let _ =
                    write!(s, "<text x=\"{lx:.1}\" y=\"{:.1}\" text-anchor=\"middle\" fill=\"{}\">{}</text>", ly + 4.0, ink.muted, esc(l));
            }
            for (k, d) in c.datasets.iter().enumerate() {
                let col = color(k, d);
                let p: Vec<String> = d.data.iter().enumerate().map(|(i, v)| pt(i, *v)).map(|(x, y)| format!("{x:.1},{y:.1}")).collect();
                let _ = write!(
                    s,
                    "<polygon points=\"{}\" fill=\"{col}\" fill-opacity=\"0.2\" stroke=\"{col}\" stroke-width=\"2\"/>",
                    p.join(" ")
                );
            }
            legend(&mut s, c, w, h - 10.0, &ink);
        }
        ChartKind::Bar | ChartKind::Line => {
            let legend_h = if c.datasets.len() > 1 || c.datasets.first().is_some_and(|d| !d.label.is_empty()) { 24.0 } else { 0.0 };
            let all: Vec<f64> = if c.stacked {
                (0..c.labels.len()).map(|i| c.datasets.iter().map(|d| d.data.get(i).copied().unwrap_or(0.0)).sum()).collect()
            } else {
                c.datasets.iter().flat_map(|d| d.data.iter().copied()).collect()
            };
            let (mn, mx) = all.iter().fold((f64::MAX, f64::MIN), |(a, b), v| (a.min(*v), b.max(*v)));
            let (mn, mx) = if all.is_empty() { (0.0, 1.0) } else { (mn, mx) };
            let lo_in = if c.kind == ChartKind::Line && mn > 0.0 && (mx - mn) < mx * 0.5 { mn } else { mn.min(0.0) };
            let ticks = if lo_in > 0.0 {
                let mut t = nice_ticks(0.0, mx - lo_in);
                let base = (lo_in / (t.get(1).copied().unwrap_or(1.0))).floor() * t.get(1).copied().unwrap_or(1.0);
                for v in t.iter_mut() {
                    *v = ((*v + base) * 1e9).round() / 1e9;
                }
                t
            } else {
                nice_ticks(mn, mx)
            };
            let lo = ticks.first().copied().unwrap_or(0.0);
            let hi = ticks.last().copied().unwrap_or(1.0).max(lo + 1e-9);
            let label_w = ticks.iter().map(|t| text_w(&num(*t), 11.0)).fold(0.0, f32::max);
            let (x0, x1, y0, y1) = (label_w + 18.0, w - 16.0, top + legend_h + 8.0, h - 34.0);
            let n = c.labels.len().max(c.datasets.iter().map(|d| d.data.len()).max().unwrap_or(0)).max(1);
            if c.horizontal && c.kind == ChartKind::Bar {
                // categories down, values across
                let lw = c.labels.iter().map(|l| text_w(l, 11.0)).fold(0.0, f32::max) + 14.0;
                let (x0, x1) = (lw, w - 24.0);
                let sx = |v: f64| x0 + ((v - lo) / (hi - lo)) as f32 * (x1 - x0);
                for t in &ticks {
                    let x = sx(*t);
                    let _ = write!(s, "<line x1=\"{x:.1}\" y1=\"{y0}\" x2=\"{x:.1}\" y2=\"{y1}\" stroke=\"{}\"/>", ink.grid);
                    let _ = write!(
                        s,
                        "<text x=\"{x:.1}\" y=\"{}\" text-anchor=\"middle\" font-size=\"11\" fill=\"{}\">{}</text>",
                        y1 + 16.0,
                        ink.muted,
                        num(*t)
                    );
                }
                let band = (y1 - y0) / n as f32;
                let k = c.datasets.len().max(1) as f32;
                for (i, l) in c.labels.iter().enumerate() {
                    let yc = y0 + band * (i as f32 + 0.5);
                    let _ = write!(
                        s,
                        "<text x=\"{}\" y=\"{:.1}\" text-anchor=\"end\" font-size=\"11\" fill=\"{}\">{}</text>",
                        lw - 8.0,
                        yc + 4.0,
                        ink.muted,
                        esc(l)
                    );
                }
                for (di, d) in c.datasets.iter().enumerate() {
                    let col = color(di, d);
                    for (i, v) in d.data.iter().enumerate() {
                        let bh = band * 0.7 / k;
                        let y = y0 + band * i as f32 + band * 0.15 + bh * di as f32;
                        let (a, b) = (sx(lo.max(0.0).min(*v)), sx(v.max(lo.max(0.0))));
                        let fill = d.colors.get(i).cloned().unwrap_or_else(|| col.clone());
                        let _ = write!(
                            s,
                            "<rect x=\"{a:.1}\" y=\"{y:.1}\" width=\"{:.1}\" height=\"{bh:.1}\" rx=\"2\" fill=\"{fill}\"/>",
                            (b - a).max(0.5)
                        );
                    }
                }
            } else {
                let sy = |v: f64| y1 - ((v - lo) / (hi - lo)) as f32 * (y1 - y0);
                for t in &ticks {
                    let y = sy(*t);
                    let _ = write!(s, "<line x1=\"{x0}\" y1=\"{y:.1}\" x2=\"{x1}\" y2=\"{y:.1}\" stroke=\"{}\"/>", ink.grid);
                    let _ = write!(
                        s,
                        "<text x=\"{}\" y=\"{:.1}\" text-anchor=\"end\" font-size=\"11\" fill=\"{}\">{}</text>",
                        x0 - 6.0,
                        y + 4.0,
                        ink.muted,
                        num(*t)
                    );
                }
                let band = (x1 - x0) / n as f32;
                let every = ((c.labels.iter().map(|l| text_w(l, 11.0)).fold(0.0, f32::max) + 8.0) / band).ceil().max(1.0) as usize;
                for (i, l) in c.labels.iter().enumerate() {
                    if i % every != 0 {
                        continue;
                    }
                    let x = x0 + band * (i as f32 + 0.5);
                    let _ = write!(
                        s,
                        "<text x=\"{x:.1}\" y=\"{}\" text-anchor=\"middle\" font-size=\"11\" fill=\"{}\">{}</text>",
                        y1 + 16.0,
                        ink.muted,
                        esc(l)
                    );
                }
                let k = if c.stacked { 1.0 } else { c.datasets.len().max(1) as f32 };
                let mut stack = vec![0.0f64; n];
                for (di, d) in c.datasets.iter().enumerate() {
                    let col = color(di, d);
                    if c.kind == ChartKind::Bar {
                        for (i, v) in d.data.iter().enumerate() {
                            let bw = band * 0.72 / k;
                            let x = x0 + band * i as f32 + band * 0.14 + if c.stacked { 0.0 } else { bw * di as f32 };
                            let base = if c.stacked { stack.get(i).copied().unwrap_or(0.0) } else { 0.0 };
                            let (a, b) = (sy((base + v).max(base.max(lo))), sy(base.max(lo).min(base + v)));
                            if let Some(st) = stack.get_mut(i) {
                                *st += v;
                            }
                            let fill = d.colors.get(i).cloned().unwrap_or_else(|| col.clone());
                            let _ = write!(
                                s,
                                "<rect x=\"{x:.1}\" y=\"{a:.1}\" width=\"{bw:.1}\" height=\"{:.1}\" rx=\"2\" fill=\"{fill}\"/>",
                                (b - a).max(0.5)
                            );
                        }
                    } else {
                        let pts: Vec<String> =
                            d.data.iter().enumerate().map(|(i, v)| format!("{:.1},{:.1}", x0 + band * (i as f32 + 0.5), sy(*v))).collect();
                        let dash = if d.dashed { " stroke-dasharray=\"6 4\"" } else { "" };
                        let _ = write!(
                            s,
                            "<polyline points=\"{}\" fill=\"none\" stroke=\"{col}\" stroke-width=\"2.5\"{dash} stroke-linejoin=\"round\"/>",
                            pts.join(" ")
                        );
                        for p in &pts {
                            let (x, y) = p.split_once(',').unwrap_or(("0", "0"));
                            let _ = write!(s, "<circle cx=\"{x}\" cy=\"{y}\" r=\"3\" fill=\"{col}\"/>");
                        }
                    }
                }
            }
            let _ = (x0, x1);
            if legend_h > 0.0 {
                legend(&mut s, c, w, top + 12.0, &ink);
            }
        }
    }
    s.push_str("</svg>");
    Svg { svg: s, width: w, height: h }
}

fn legend(s: &mut String, c: &Chart, w: f32, y: f32, ink: &Ink) {
    let items: Vec<(String, String)> = c
        .datasets
        .iter()
        .enumerate()
        .map(|(i, d)| (d.label.clone(), d.color.clone().unwrap_or_else(|| PALETTE[i % PALETTE.len()].to_string())))
        .collect();
    let total: f32 = items.iter().map(|(l, _)| text_w(l, 12.0) + 32.0).sum();
    let mut x = (w - total) / 2.0;
    for (l, col) in items {
        let _ = write!(s, "<rect x=\"{x:.1}\" y=\"{}\" width=\"12\" height=\"12\" rx=\"2\" fill=\"{col}\"/>", y - 10.0);
        let _ = write!(s, "<text x=\"{:.1}\" y=\"{y}\" fill=\"{}\">{}</text>", x + 17.0, ink.fg, esc(&l));
        x += text_w(&l, 12.0) + 32.0;
    }
}

// ---- Mermaid -----------------------------------------------------------------------------------

fn mermaid(code: &str, dark: bool) -> Option<Svg> {
    let lines = mermaid_lines(code);
    let first = *lines.first()?;
    let kw = first.split_whitespace().next().unwrap_or("");
    match kw {
        "flowchart" | "graph" => graph::flowchart(&lines, dark),
        "stateDiagram" | "stateDiagram-v2" => graph::state(&lines, dark),
        "classDiagram" | "classDiagram-v2" => graph::class(&lines, dark, false),
        "erDiagram" => graph::er(&lines, dark),
        "mindmap" => kinds::mindmap_mermaid(&lines, code, dark),
        "timeline" => kinds::timeline(&lines, dark),
        "journey" => kinds::journey(&lines, dark),
        "quadrantChart" => kinds::quadrant(&lines, dark),
        "gitGraph" => kinds::git(&lines, dark),
        "pie" => {
            let head = first;
            let title = head.find("title").map(|i| head[i + 5..].trim().to_string());
            let mut labels = Vec::new();
            let mut data = Vec::new();
            for l in &lines[1..] {
                if let Some(t) = l.strip_prefix("title") {
                    let _ = t;
                    continue;
                }
                if let Some((a, b)) = l.rsplit_once(':') {
                    labels.push(a.trim().trim_matches('"').to_string());
                    data.push(b.trim().parse().unwrap_or(0.0));
                }
            }
            let title = title.or_else(|| lines.iter().find_map(|l| l.strip_prefix("title ").map(str::to_string)));
            let chart = Chart {
                kind: ChartKind::Pie,
                labels,
                datasets: vec![Dataset { label: String::new(), data, color: None, colors: Vec::new(), dashed: false }],
                title,
                horizontal: false,
                stacked: false,
            };
            Some(draw_chart(&chart, dark))
        }
        "sequenceDiagram" => sequence(&lines, dark),
        "gantt" => gantt(&lines, dark),
        _ => None,
    }
}

fn sequence(lines: &[&str], dark: bool) -> Option<Svg> {
    let ink = ink(dark);
    let mut actors: Vec<(String, String)> = Vec::new();
    let add = |id: &str, label: &str, actors: &mut Vec<(String, String)>| -> usize {
        if let Some(i) = actors.iter().position(|(a, _)| a == id) {
            return i;
        }
        actors.push((id.to_string(), label.to_string()));
        actors.len() - 1
    };
    enum Step {
        Msg { from: usize, to: usize, text: String, dashed: bool, n: usize },
        Note { at: usize, to: usize, text: String },
    }
    let mut steps = Vec::new();
    let mut autonumber = false;
    let mut count = 0;
    for l in &lines[1..] {
        if *l == "autonumber" {
            autonumber = true;
            continue;
        }
        if let Some(r) = l.strip_prefix("participant ").or_else(|| l.strip_prefix("actor ")) {
            let (id, label) = match r.split_once(" as ") {
                Some((a, b)) => (a.trim(), b.trim()),
                None => (r.trim(), r.trim()),
            };
            add(id, label, &mut actors);
            continue;
        }
        if let Some(r) = l.strip_prefix("Note ").or_else(|| l.strip_prefix("note ")) {
            if let Some((place, text)) = r.split_once(':') {
                let names = place.split_whitespace().skip(1).collect::<Vec<_>>().join(" ");
                let who: Vec<&str> = names.split(',').map(str::trim).collect();
                let a = who.first().map(|w| add(w, w, &mut actors)).unwrap_or(0);
                let b = who.get(1).map(|w| add(w, w, &mut actors)).unwrap_or(a);
                steps.push(Step::Note { at: a.min(b), to: a.max(b), text: text.trim().to_string() });
            }
            continue;
        }
        let Some((head, text)) = l.split_once(':') else { continue };
        for arrow in ["-->>", "->>", "-->", "->", "--x", "-x", "--)", "-)"] {
            if let Some((a, b)) = head.split_once(arrow) {
                let (a, b) = (a.trim(), b.trim().trim_start_matches(['+', '-']));
                let from = add(a, a, &mut actors);
                let to = add(b, b, &mut actors);
                count += 1;
                steps.push(Step::Msg { from, to, text: text.trim().to_string(), dashed: arrow.starts_with("--"), n: count });
                break;
            }
        }
    }
    if actors.is_empty() {
        return None;
    }
    let col_w = actors.iter().map(|(_, l)| text_w(l, 13.0) + 40.0).fold(150.0, f32::max);
    let step_h = 46.0;
    let w = col_w * actors.len() as f32 + 20.0;
    let h = 70.0 + steps.len() as f32 * step_h + 60.0;
    let x_of = |i: usize| 10.0 + col_w * (i as f32 + 0.5);
    let mut s = header(w, h, &ink);
    let _ = write!(
        s,
        "<defs><marker id=\"sa\" viewBox=\"0 0 10 10\" refX=\"9\" refY=\"5\" markerWidth=\"7\" markerHeight=\"7\" orient=\"auto\"><path d=\"M0,0 L10,5 L0,10 z\" fill=\"{}\"/></marker></defs>",
        ink.fg
    );
    for (i, (_, label)) in actors.iter().enumerate() {
        let x = x_of(i);
        let bw = text_w(label, 13.0) + 28.0;
        let _ = write!(s, "<line x1=\"{x}\" y1=\"46\" x2=\"{x}\" y2=\"{}\" stroke=\"{}\" stroke-dasharray=\"4 4\"/>", h - 46.0, ink.grid);
        for y in [10.0, h - 46.0] {
            let _ = write!(
                s,
                "<rect x=\"{:.1}\" y=\"{y}\" width=\"{bw:.1}\" height=\"36\" rx=\"4\" fill=\"{}\" stroke=\"{}\"/><text x=\"{x}\" y=\"{}\" text-anchor=\"middle\" font-size=\"13\">{}</text>",
                x - bw / 2.0,
                ink.node,
                ink.node_border,
                y + 23.0,
                esc(label)
            );
        }
    }
    for (k, st) in steps.iter().enumerate() {
        let y = 80.0 + k as f32 * step_h;
        match st {
            Step::Msg { from, to, text, dashed, n } => {
                let (x1, x2) = (x_of(*from), x_of(*to));
                let dash = if *dashed { " stroke-dasharray=\"5 4\"" } else { "" };
                let label = if autonumber { format!("{n}. {text}") } else { text.clone() };
                if from == to {
                    let _ =
                        write!(s, "<path d=\"M{x1},{y} h40 v18 h-40\" fill=\"none\" stroke=\"{}\"{dash} marker-end=\"url(#sa)\"/>", ink.fg);
                    let _ = write!(s, "<text x=\"{}\" y=\"{}\" font-size=\"12\">{}</text>", x1 + 46.0, y + 4.0, esc(&label));
                } else {
                    let _ = write!(
                        s,
                        "<line x1=\"{x1}\" y1=\"{}\" x2=\"{x2}\" y2=\"{}\" stroke=\"{}\" stroke-width=\"1.5\"{dash} marker-end=\"url(#sa)\"/>",
                        y + 10.0,
                        y + 10.0,
                        ink.fg
                    );
                    let _ = write!(
                        s,
                        "<text x=\"{}\" y=\"{y}\" text-anchor=\"middle\" font-size=\"12\">{}</text>",
                        (x1 + x2) / 2.0,
                        esc(&label)
                    );
                }
            }
            Step::Note { at, to, text } => {
                let (x1, x2) = (x_of(*at) - 60.0, x_of(*to) + 60.0);
                let _ = write!(
                    s,
                    "<rect x=\"{x1}\" y=\"{}\" width=\"{}\" height=\"28\" rx=\"3\" fill=\"#fff5c2\" stroke=\"#d4b106\"/><text x=\"{}\" y=\"{}\" text-anchor=\"middle\" font-size=\"12\" fill=\"#3d3200\">{}</text>",
                    y - 6.0,
                    x2 - x1,
                    (x1 + x2) / 2.0,
                    y + 12.0,
                    esc(text)
                );
            }
        }
    }
    s.push_str("</svg>");
    Some(Svg { svg: s, width: w, height: h })
}

fn parse_date(s: &str) -> Option<i64> {
    let mut p = s.trim().splitn(3, '-');
    let (y, m, d) = (p.next()?.parse::<i32>().ok()?, p.next()?.parse::<u32>().ok()?, p.next()?.parse::<u32>().ok()?);
    let date = chrono::NaiveDate::from_ymd_opt(y, m, d)?;
    Some(date.signed_duration_since(chrono::NaiveDate::from_ymd_opt(1970, 1, 1)?).num_days())
}

fn duration_days(s: &str) -> Option<i64> {
    let s = s.trim();
    let (n, unit) = s.split_at(s.find(|c: char| !c.is_ascii_digit() && c != '.')?);
    let n: f64 = n.parse().ok()?;
    Some(
        match unit {
            "d" => n,
            "w" => n * 7.0,
            "h" => n / 24.0,
            _ => return None,
        }
        .ceil() as i64,
    )
}

fn gantt(lines: &[&str], dark: bool) -> Option<Svg> {
    let ink = ink(dark);
    let mut title = None;
    let mut section = String::new();
    struct Task {
        name: String,
        section: String,
        start: i64,
        end: i64,
    }
    let mut tasks: Vec<Task> = Vec::new();
    let mut ids: HashMap<String, i64> = HashMap::new();
    let mut last_end: Option<i64> = None;
    for l in &lines[1..] {
        if let Some(t) = l.strip_prefix("title ") {
            title = Some(t.trim().to_string());
            continue;
        }
        if let Some(t) = l.strip_prefix("section ") {
            section = t.trim().to_string();
            continue;
        }
        if l.starts_with("dateFormat") || l.starts_with("axisFormat") || l.starts_with("excludes") || l.starts_with("todayMarker") {
            continue;
        }
        let Some((name, spec)) = l.split_once(':') else { continue };
        let parts: Vec<&str> = spec.split(',').map(str::trim).filter(|p| !matches!(*p, "done" | "active" | "crit" | "milestone")).collect();
        let mut id = None;
        let mut start = None;
        let mut end = None;
        for p in &parts {
            if let Some(a) = p.strip_prefix("after ") {
                start = a.split_whitespace().filter_map(|x| ids.get(x).copied()).max();
            } else if let Some(d) = parse_date(p) {
                if start.is_none() { start = Some(d) } else { end = Some(d) }
            } else if let Some(d) = duration_days(p) {
                end = start.or(last_end).map(|s| s + d);
                if start.is_none() {
                    start = last_end;
                }
            } else if id.is_none() {
                id = Some(p.to_string());
            }
        }
        let Some(start) = start.or(last_end) else { continue };
        let end = end.unwrap_or(start + 1).max(start);
        if let Some(id) = id {
            ids.insert(id, end);
        }
        last_end = Some(end);
        tasks.push(Task { name: name.trim().to_string(), section: section.clone(), start, end });
    }
    if tasks.is_empty() {
        return None;
    }
    let lo = tasks.iter().map(|t| t.start).min()?;
    let hi = tasks.iter().map(|t| t.end).max()?.max(lo + 1);
    let name_w = tasks.iter().map(|t| text_w(&t.name, 12.0)).fold(0.0, f32::max) + 24.0;
    let (w, row) = (760.0f32, 28.0f32);
    let top = if title.is_some() { 40.0 } else { 14.0 };
    let h = top + 24.0 + tasks.len() as f32 * row + 16.0;
    let (x0, x1) = (name_w, w - 16.0);
    let sx = |d: i64| x0 + (d - lo) as f32 / (hi - lo) as f32 * (x1 - x0);
    let mut s = header(w, h, &ink);
    if let Some(t) = &title {
        let _ = write!(s, "<text x=\"{}\" y=\"24\" text-anchor=\"middle\" font-size=\"15\" font-weight=\"600\">{}</text>", w / 2.0, esc(t));
    }
    // weeks on the axis
    let day0 = chrono::NaiveDate::from_ymd_opt(1970, 1, 1)?;
    let span = hi - lo;
    let step = if span > 120 {
        30
    } else if span > 40 {
        14
    } else {
        7
    };
    let mut d = lo;
    while d <= hi {
        let x = sx(d);
        let date = day0 + chrono::Duration::days(d);
        let _ = write!(s, "<line x1=\"{x:.1}\" y1=\"{}\" x2=\"{x:.1}\" y2=\"{}\" stroke=\"{}\"/>", top + 18.0, h - 10.0, ink.grid);
        let _ = write!(
            s,
            "<text x=\"{x:.1}\" y=\"{}\" text-anchor=\"middle\" font-size=\"10.5\" fill=\"{}\">{}</text>",
            top + 12.0,
            ink.muted,
            date.format("%-d %b")
        );
        d += step;
    }
    let mut sections: Vec<&str> = Vec::new();
    for (i, t) in tasks.iter().enumerate() {
        let y = top + 24.0 + i as f32 * row;
        let si = match sections.iter().position(|x| *x == t.section) {
            Some(p) => p,
            None => {
                sections.push(&t.section);
                sections.len() - 1
            }
        };
        let col = PALETTE[si % PALETTE.len()];
        let _ = write!(
            s,
            "<text x=\"{}\" y=\"{:.1}\" text-anchor=\"end\" font-size=\"12\">{}</text>",
            x0 - 10.0,
            y + row / 2.0 + 4.0,
            esc(&t.name)
        );
        let (a, b) = (sx(t.start), sx(t.end));
        let _ = write!(
            s,
            "<rect x=\"{a:.1}\" y=\"{:.1}\" width=\"{:.1}\" height=\"{:.1}\" rx=\"4\" fill=\"{col}\"/>",
            y + 5.0,
            (b - a).max(3.0),
            row - 10.0
        );
    }
    s.push_str("</svg>");
    Some(Svg { svg: s, width: w, height: h })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::md::library;

    #[test]
    fn js_objects_become_json() {
        let j = js_to_json("{ type: 'bar', data: { labels: [\"a\", 'b',], }, // c\n }");
        let v: serde_json::Value = serde_json::from_str(&j).expect("json");
        assert_eq!(v["type"], "bar");
        assert_eq!(v["data"]["labels"][1], "b");
    }

    #[test]
    fn library_charts_draw() {
        for c in library::CHARTS {
            let svg = render(c.lang, c.code, false);
            assert!(svg.is_some(), "{} does not draw", c.id);
            assert!(svg.expect("svg").svg.ends_with("</svg>"));
        }
    }

    #[test]
    fn mermaid_kinds() {
        for d in library::DIAGRAMS {
            let id = d.id;
            let d = library::DIAGRAMS.iter().find(|d| d.id == id).expect("snippet");
            let svg = render(d.lang, d.code, true);
            assert!(svg.is_some(), "{id} does not draw");
        }
        assert!(render("mermaid", "requirementDiagram\n  requirement A", false).is_none());
        assert!(!supported("mermaid", "requirementDiagram\n  requirement A"));
        assert!(supported("mermaid", "classDiagram\n  class A"));
        assert!(supported("mermaid", "flowchart LR\n  A --> B"));
        assert!(supported("plantuml", "@startuml"));
        assert!(render("rust", "fn main() {}", false).is_none());
    }

    #[test]
    fn flowchart_parses_shapes_labels_and_chains() {
        let svg = render("mermaid", "flowchart LR\n  A([Start]) -->|go| B{OK?}\n  B -- Yes --> C[(DB)] --> D((End))\n  B -.-> A", false)
            .expect("svg");
        assert!(svg.svg.contains(">Start<") && svg.svg.contains(">OK?<") && svg.svg.contains(">go<") && svg.svg.contains(">Yes<"));
        assert!(svg.svg.contains("stroke-dasharray"));
    }

    #[test]
    fn templates_draw_their_charts() {
        for t in crate::md::templates::BUILT_IN {
            for b in crate::md::parse(t.source).blocks {
                if let crate::md::Block::Code { lang, info, text, .. } = b
                    && (lang == "chart" || lang == "csv" && info.contains("chart="))
                {
                    assert!(render(&info, &text, false).is_some(), "{}: {info}", t.id);
                }
            }
        }
    }
}
