//! Box-and-arrow diagrams on a layered layout: Mermaid flowcharts, class, state and entity
//! relationship diagrams, and Graphviz (DOT) graphs.

use std::collections::HashMap;
use std::fmt::Write as _;

use super::{Svg, esc, header, ink, text_w};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum Shape {
    Rect,
    Round,
    Stadium,
    Cylinder,
    Diamond,
    Circle,
    Hexagon,
    Ellipse,
    /// A box with a title and compartments (classes, entities).
    Table,
    /// The filled dot where a state diagram starts.
    Start,
    /// The ringed dot where it ends.
    End,
    /// Plain text without a border.
    Plain,
}

#[derive(Debug, Clone)]
pub(super) struct Node {
    pub label: String,
    pub shape: Shape,
    /// Compartments under the title (`Shape::Table`).
    pub body: Vec<Vec<String>>,
    pub fill: Option<String>,
}

impl Node {
    pub fn new(label: impl Into<String>, shape: Shape) -> Node {
        Node { label: label.into(), shape, body: Vec::new(), fill: None }
    }
}

/// What an edge's end looks like.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum Mark {
    None,
    Arrow,
    /// Open triangle (inheritance, realisation).
    Triangle,
    Diamond,
    OpenDiamond,
    /// Open arrow head (dependency).
    Open,
}

#[derive(Debug, Clone)]
pub(super) struct Edge {
    pub from: usize,
    pub to: usize,
    pub label: Option<String>,
    pub dotted: bool,
    pub thick: bool,
    pub start: Mark,
    pub end: Mark,
    /// Text next to the start and the end (cardinalities).
    pub tail: Option<String>,
    pub head: Option<String>,
}

impl Edge {
    pub fn new(from: usize, to: usize) -> Edge {
        Edge { from, to, label: None, dotted: false, thick: false, start: Mark::None, end: Mark::Arrow, tail: None, head: None }
    }
}

/// Nodes by id, created on first use.
#[derive(Default)]
pub(super) struct Graph {
    pub nodes: Vec<Node>,
    pub edges: Vec<Edge>,
    index: HashMap<String, usize>,
}

impl Graph {
    pub fn node(&mut self, id: &str, default: Shape) -> usize {
        if let Some(i) = self.index.get(id) {
            return *i;
        }
        self.nodes.push(Node::new(id, default));
        self.index.insert(id.to_string(), self.nodes.len() - 1);
        self.nodes.len() - 1
    }
    pub fn get(&self, id: &str) -> Option<usize> {
        self.index.get(id).copied()
    }
}

// ---- Mermaid flowchart ---------------------------------------------------------------------------

/// A node: its id, its label and shape when given, and the rest of the line.
type NodeParse<'a> = (String, Option<(String, Shape)>, &'a str);

fn parse_node(s: &str) -> Option<NodeParse<'_>> {
    let s = s.trim_start();
    let id_end = s.find(|c: char| !(c.is_alphanumeric() || c == '_')).unwrap_or(s.len());
    if id_end == 0 {
        return None;
    }
    let id = s[..id_end].to_string();
    let rest = &s[id_end..];
    let shapes: [(&str, &str, Shape); 12] = [
        ("([", "])", Shape::Stadium),
        ("[(", ")]", Shape::Cylinder),
        ("(((", ")))", Shape::Circle),
        ("((", "))", Shape::Circle),
        ("{{", "}}", Shape::Hexagon),
        ("[[", "]]", Shape::Rect),
        ("[/", "/]", Shape::Rect),
        ("[\\", "\\]", Shape::Rect),
        ("[", "]", Shape::Rect),
        ("(", ")", Shape::Round),
        ("{", "}", Shape::Diamond),
        (">", "]", Shape::Rect),
    ];
    for (open, close, shape) in shapes {
        if let Some(r) = rest.strip_prefix(open)
            && let Some(e) = r.find(close)
        {
            let label = clean_label(&r[..e]);
            return Some((id, Some((label, shape)), &r[e + close.len()..]));
        }
    }
    Some((id, None, rest))
}

/// Quotes, `<br>` and Markdown markers out of a label.
fn clean_label(s: &str) -> String {
    let s = s.trim().trim_matches('"').trim_matches('`');
    let s = s.replace("<br>", " ").replace("<br/>", " ").replace("<br />", " ");
    s.replace("**", "").replace("&quot;", "\"").trim().to_string()
}

/// The link at the start of `s`: (label, dotted, thick, start, end) and the rest.
type LinkParse<'a> = (Option<String>, bool, bool, Mark, Mark, &'a str);

fn parse_link(s: &str) -> Option<LinkParse<'_>> {
    let s = s.trim_start();
    // A -- text --> B
    for (open, close, dotted, thick) in
        [("-- ", "-->", false, false), ("-. ", ".->", true, false), ("== ", "==>", false, true), ("-- ", "---", false, false)]
    {
        if let Some(r) = s.strip_prefix(open)
            && let Some(e) = r.find(close)
        {
            let end = if close.ends_with('>') { Mark::Arrow } else { Mark::None };
            return Some((Some(clean_label(&r[..e])), dotted, thick, Mark::None, end, &r[e + close.len()..]));
        }
    }
    // the link token: <? then - . = then > o x ?
    let b = s.as_bytes();
    let mut i = 0;
    let start = if b.first() == Some(&b'<') {
        i += 1;
        Mark::Arrow
    } else {
        Mark::None
    };
    let body_start = i;
    while i < b.len() && matches!(b[i], b'-' | b'.' | b'=') {
        i += 1;
    }
    let body = &s[body_start..i];
    if body.len() < 2 && !(body.len() == 1 && b.get(i) == Some(&b'>')) {
        return None;
    }
    if !body.contains('-') && !body.contains('=') {
        return None;
    }
    let mut end = Mark::None;
    if i < b.len() && matches!(b[i], b'>' | b'o' | b'x') {
        // `o` / `x` only count when a space or a node follows
        let next = b.get(i + 1).copied().unwrap_or(b' ');
        if b[i] == b'>' || !next.is_ascii_alphanumeric() {
            end = Mark::Arrow;
            i += 1;
        }
    }
    let dotted = body.contains('.');
    let thick = body.contains('=');
    let r = s[i..].trim_start();
    if let Some(r2) = r.strip_prefix('|')
        && let Some(e) = r2.find('|')
    {
        return Some((Some(clean_label(&r2[..e])), dotted, thick, start, end, &r2[e + 1..]));
    }
    Some((None, dotted, thick, start, end, r))
}

pub(super) fn flowchart(lines: &[&str], dark: bool) -> Option<Svg> {
    let dir = lines[0].split_whitespace().nth(1).unwrap_or("TD").to_uppercase();
    let horizontal = dir == "LR" || dir == "RL";
    let mut g = Graph::default();
    let mut fills: HashMap<String, String> = HashMap::new();
    let mut classes: Vec<(String, String)> = Vec::new();
    for l in &lines[1..] {
        let kw = l.split_whitespace().next().unwrap_or("");
        match kw {
            "classDef" => {
                let mut it = l.split_whitespace().skip(1);
                if let (Some(name), Some(style)) = (it.next(), it.next())
                    && let Some(f) = style.split(',').find_map(|p| p.strip_prefix("fill:"))
                {
                    fills.insert(name.to_string(), f.to_string());
                }
                continue;
            }
            "class" => {
                let mut it = l.split_whitespace().skip(1);
                if let (Some(ids), Some(name)) = (it.next(), it.next()) {
                    for id in ids.split(',') {
                        classes.push((id.to_string(), name.trim_end_matches(';').to_string()));
                    }
                }
                continue;
            }
            "style" | "linkStyle" | "click" | "subgraph" | "end" | "direction" => continue,
            _ => {}
        }
        for stmt in l.split(';') {
            let Some((id, def, mut rest)) = parse_node(stmt) else { continue };
            let (id, rest2) = class_suffix(&id, rest, &mut classes);
            rest = rest2;
            let mut prev = flow_node(&mut g, &id, def);
            while let Some((label, dotted, thick, start, end, r)) = parse_link(rest) {
                // `A --> B & C`
                let mut targets = Vec::new();
                let mut r = r;
                while let Some((id, def, r2)) = parse_node(r) {
                    let (id, r2) = class_suffix(&id, r2, &mut classes);
                    targets.push(flow_node(&mut g, &id, def));
                    r = r2.trim_start();
                    if let Some(r3) = r.strip_prefix('&') {
                        r = r3;
                    } else {
                        break;
                    }
                }
                for t in &targets {
                    g.edges.push(Edge { label: label.clone(), dotted, thick, start, end, ..Edge::new(prev, *t) });
                }
                match targets.last() {
                    Some(t) => prev = *t,
                    None => break,
                }
                rest = r;
            }
        }
    }
    for (id, class) in classes {
        if let (Some(i), Some(f)) = (g.get(&id), fills.get(&class)) {
            g.nodes[i].fill = Some(f.clone());
        }
    }
    if g.nodes.is_empty() {
        return None;
    }
    if dir == "RL" || dir == "BT" {
        for e in &mut g.edges {
            std::mem::swap(&mut e.from, &mut e.to);
            std::mem::swap(&mut e.start, &mut e.end);
        }
    }
    Some(layered(&g, horizontal, dark))
}

/// `A:::class` → the class is remembered.
fn class_suffix<'a>(id: &str, rest: &'a str, classes: &mut Vec<(String, String)>) -> (String, &'a str) {
    if let Some(r) = rest.strip_prefix(":::") {
        let e = r.find(|c: char| !(c.is_alphanumeric() || c == '_' || c == '-')).unwrap_or(r.len());
        classes.push((id.to_string(), r[..e].to_string()));
        return (id.to_string(), &r[e..]);
    }
    (id.to_string(), rest)
}

fn flow_node(g: &mut Graph, id: &str, def: Option<(String, Shape)>) -> usize {
    let i = g.node(id, Shape::Rect);
    if let Some((label, shape)) = def {
        g.nodes[i].label = label;
        g.nodes[i].shape = shape;
    }
    i
}

// ---- Mermaid state diagram ----------------------------------------------------------------------

pub(super) fn state(lines: &[&str], dark: bool) -> Option<Svg> {
    let mut g = Graph::default();
    let mut horizontal = false;
    let mut names: HashMap<String, String> = HashMap::new();
    let mut starts = 0;
    let mut ends = 0;
    // `[*]` is a start when it is the source, an end when it is the target
    let mut endpoint = |g: &mut Graph, id: &str, source: bool, names: &HashMap<String, String>| -> usize {
        if id == "[*]" {
            if source {
                starts += 1;
                let i = g.node(&format!("[*]start{starts}"), Shape::Start);
                g.nodes[i].label.clear();
                i
            } else {
                ends += 1;
                let i = g.node(&format!("[*]end{ends}"), Shape::End);
                g.nodes[i].label.clear();
                i
            }
        } else {
            let i = g.node(id, Shape::Round);
            if let Some(n) = names.get(id) {
                g.nodes[i].label = n.clone();
            }
            i
        }
    };
    for l in &lines[1..] {
        if l.starts_with("direction") {
            horizontal = l.contains("LR") || l.contains("RL");
            continue;
        }
        if l.starts_with("note")
            || l.starts_with("end note")
            || *l == "--"
            || *l == "}"
            || l.starts_with("classDef")
            || l.starts_with("class ")
        {
            continue;
        }
        if let Some(r) = l.strip_prefix("state ") {
            // state "Long name" as id   ·   state id { …   ·   state id <<choice>>
            let r = r.trim().trim_end_matches('{').trim();
            if let Some(q) = r.strip_prefix('"')
                && let Some(e) = q.find('"')
            {
                let label = q[..e].to_string();
                let id = q[e + 1..].trim().strip_prefix("as").map(str::trim).unwrap_or(&label).to_string();
                names.insert(id.clone(), label.clone());
                let i = g.node(&id, Shape::Round);
                g.nodes[i].label = label;
            } else {
                let mut it = r.split_whitespace();
                let id = it.next().unwrap_or("");
                let i = g.node(id, Shape::Round);
                match it.next() {
                    Some("<<choice>>") => {
                        g.nodes[i].shape = Shape::Diamond;
                        g.nodes[i].label.clear();
                    }
                    Some("<<fork>>") | Some("<<join>>") => {
                        g.nodes[i].shape = Shape::Rect;
                        g.nodes[i].label.clear();
                    }
                    _ => {}
                }
            }
            continue;
        }
        if let Some(a) = l.find("-->") {
            let from = l[..a].trim();
            let (to, label) = match l[a + 3..].split_once(':') {
                Some((t, lab)) => (t.trim(), Some(lab.trim().to_string())),
                None => (l[a + 3..].trim(), None),
            };
            let f = endpoint(&mut g, from, true, &names);
            let t = endpoint(&mut g, to, false, &names);
            g.edges.push(Edge { label, ..Edge::new(f, t) });
            continue;
        }
        // `Id : description`
        if let Some((id, desc)) = l.split_once(':') {
            let i = g.node(id.trim(), Shape::Round);
            g.nodes[i].body.push(vec![desc.trim().to_string()]);
        }
    }
    if g.nodes.is_empty() {
        return None;
    }
    // states with a description show it under the name
    for n in &mut g.nodes {
        if !n.body.is_empty() {
            n.shape = Shape::Table;
            let lines: Vec<String> = n.body.concat();
            n.body = vec![lines];
        }
    }
    Some(layered(&g, horizontal, dark))
}

// ---- Mermaid class diagram ----------------------------------------------------------------------

/// Class relation operators: (token, start mark, end mark, dotted).
const CLASS_OPS: [(&str, Mark, Mark, bool); 14] = [
    ("<|--", Mark::Triangle, Mark::None, false),
    ("--|>", Mark::None, Mark::Triangle, false),
    ("<|..", Mark::Triangle, Mark::None, true),
    ("..|>", Mark::None, Mark::Triangle, true),
    ("*--", Mark::Diamond, Mark::None, false),
    ("--*", Mark::None, Mark::Diamond, false),
    ("o--", Mark::OpenDiamond, Mark::None, false),
    ("--o", Mark::None, Mark::OpenDiamond, false),
    ("<--", Mark::Arrow, Mark::None, false),
    ("-->", Mark::None, Mark::Arrow, false),
    ("<..", Mark::Open, Mark::None, true),
    ("..>", Mark::None, Mark::Open, true),
    ("--", Mark::None, Mark::None, false),
    ("..", Mark::None, Mark::None, true),
];

/// A class relation: the two classes, the marks at their ends, dotted, the label and the
/// cardinalities at both ends.
type Relation = (String, String, Mark, Mark, bool, Option<String>, Option<String>, Option<String>);

/// A relation line `A "1" <|-- "*" B : label`.
fn class_relation(l: &str) -> Option<Relation> {
    let (rel, label) = match l.split_once(" : ").or_else(|| l.split_once(':')) {
        Some((r, lab)) => (r, Some(lab.trim().to_string())),
        None => (l, None),
    };
    for (op, start, end, dotted) in CLASS_OPS {
        if let Some(p) = rel.find(op) {
            let left = rel[..p].trim();
            let right = rel[p + op.len()..].trim();
            let (a, tail) = split_card(left, true);
            let (b, head) = split_card(right, false);
            if a.is_empty() || b.is_empty() || a.contains(' ') || b.contains(' ') {
                return None;
            }
            return Some((a, b, start, end, dotted, label, tail, head));
        }
    }
    None
}

/// `Name "1"` / `"1" Name` → (name, cardinality).
fn split_card(s: &str, left: bool) -> (String, Option<String>) {
    let s = s.trim();
    if let (Some(a), Some(b)) = (s.find('"'), s.rfind('"'))
        && a < b
    {
        let card = s[a + 1..b].to_string();
        let name = if left { s[..a].trim() } else { s[b + 1..].trim() };
        return (clean_class(name), Some(card));
    }
    (clean_class(s), None)
}

fn clean_class(s: &str) -> String {
    // generics: List~int~ → List<int>
    s.trim().trim_matches('`').replace('~', "")
}

pub(super) fn class(lines: &[&str], dark: bool, plantuml: bool) -> Option<Svg> {
    let mut g = Graph::default();
    let mut horizontal = false;
    let mut open: Option<usize> = None;
    let member = |g: &mut Graph, i: usize, m: &str| {
        let m = m.trim().trim_end_matches(';');
        if m.is_empty() || m.starts_with("<<") {
            return;
        }
        let n = &mut g.nodes[i];
        n.shape = Shape::Table;
        while n.body.len() < 2 {
            n.body.push(Vec::new());
        }
        let k = if m.contains('(') { 1 } else { 0 };
        n.body[k].push(m.replace('~', ""));
    };
    for l in lines.iter().skip(if plantuml { 0 } else { 1 }) {
        let l = l.trim();
        if l.starts_with('@') || l.starts_with("skinparam") || l.starts_with("hide") || l.starts_with("show") || l.starts_with("note") {
            continue;
        }
        if l.starts_with("direction") {
            horizontal = l.contains("LR") || l.contains("RL");
            continue;
        }
        if let Some(i) = open {
            if l.starts_with('}') {
                open = None;
            } else {
                member(&mut g, i, l);
            }
            continue;
        }
        let decl = ["class ", "interface ", "abstract class ", "abstract ", "enum ", "annotation "]
            .iter()
            .find_map(|k| l.strip_prefix(k).map(|r| (*k, r)));
        if let Some((kind, r)) = decl {
            let r = r.trim();
            let braces = r.ends_with('{');
            let name_part = r.trim_end_matches('{').trim();
            // class A as "Label"  ·  class A["Label"]  ·  class A <<interface>>
            let name = name_part.split(|c: char| c.is_whitespace() || c == '[' || c == '<' || c == '{').next().unwrap_or("");
            let name = clean_class(name);
            let i = g.node(&name, Shape::Table);
            g.nodes[i].shape = Shape::Table;
            if let Some(a) = name_part.find("[\"")
                && let Some(b) = name_part[a + 2..].find("\"]")
            {
                g.nodes[i].label = name_part[a + 2..a + 2 + b].to_string();
            }
            let stereo = if kind.starts_with("interface") || name_part.contains("<<interface>>") {
                Some("«interface»")
            } else if kind.starts_with("enum") || name_part.contains("<<enumeration>>") {
                Some("«enumeration»")
            } else if kind.starts_with("abstract") || name_part.contains("<<abstract>>") {
                Some("«abstract»")
            } else {
                None
            };
            if let Some(s) = stereo {
                g.nodes[i].label = format!("{s} {}", g.nodes[i].label);
            }
            if braces {
                open = Some(i);
            }
            continue;
        }
        if let Some((a, b, start, end, dotted, label, tail, head)) = class_relation(l) {
            let ai = g.node(&a, Shape::Table);
            let bi = g.node(&b, Shape::Table);
            // the parent (triangle, diamond) above its children
            let parent_right = matches!(end, Mark::Triangle | Mark::Diamond | Mark::OpenDiamond) && start == Mark::None;
            let e = if parent_right {
                Edge { label, dotted, start: end, end: start, tail: head, head: tail, ..Edge::new(bi, ai) }
            } else {
                Edge { label, dotted, start, end, tail, head, ..Edge::new(ai, bi) }
            };
            g.edges.push(e);
            continue;
        }
        // `Name : +member`
        if let Some((name, m)) = l.split_once(':')
            && !name.trim().contains(' ')
            && !name.trim().is_empty()
        {
            let i = g.node(&clean_class(name), Shape::Table);
            member(&mut g, i, m);
        }
    }
    if g.nodes.is_empty() {
        return None;
    }
    for n in &mut g.nodes {
        n.shape = Shape::Table;
        if n.body.is_empty() {
            n.body = vec![Vec::new()];
        }
    }
    Some(layered(&g, horizontal, dark))
}

// ---- Mermaid entity relationship diagram --------------------------------------------------------

/// `||`, `|o`, `}|`, `}o` (and mirrored) → a cardinality.
fn er_card(s: &str) -> &'static str {
    match s {
        "||" => "1",
        "|o" | "o|" => "0..1",
        "}|" | "|{" => "1..*",
        "}o" | "o{" => "0..*",
        _ => "",
    }
}

pub(super) fn er(lines: &[&str], dark: bool) -> Option<Svg> {
    let mut g = Graph::default();
    let mut open: Option<usize> = None;
    let mut horizontal = true;
    for l in &lines[1..] {
        if l.starts_with("direction") {
            horizontal = l.contains("LR") || l.contains("RL");
            continue;
        }
        if let Some(i) = open {
            if l.starts_with('}') {
                open = None;
            } else {
                let n = &mut g.nodes[i];
                if n.body.is_empty() {
                    n.body.push(Vec::new());
                }
                // type name PK "comment"
                let row = l.split('"').next().unwrap_or(l).split_whitespace().collect::<Vec<_>>().join("  ");
                n.body[0].push(row);
            }
            continue;
        }
        if let Some(name) = l.strip_suffix('{') {
            let name = name.trim().trim_matches('"');
            let name = name.split('[').next().unwrap_or(name).trim();
            open = Some(g.node(name, Shape::Table));
            continue;
        }
        // A ||--o{ B : label
        let (rel, label) = match l.split_once(':') {
            Some((r, lab)) => (r.trim(), Some(lab.trim().trim_matches('"').to_string())),
            None => (l.trim(), None),
        };
        let mut it = rel.split_whitespace();
        let (Some(a), Some(op), Some(b)) = (it.next(), it.next(), it.next()) else {
            if !rel.is_empty() && !rel.contains(' ') {
                g.node(rel.trim_matches('"'), Shape::Table);
            }
            continue;
        };
        if op.len() < 6 {
            continue;
        }
        let (left, mid, right) = (&op[..2], &op[2..op.len() - 2], &op[op.len() - 2..]);
        let ai = g.node(a.trim_matches('"'), Shape::Table);
        let bi = g.node(b.trim_matches('"'), Shape::Table);
        g.edges.push(Edge {
            label,
            dotted: mid.contains('.'),
            end: Mark::None,
            tail: Some(er_card(left).to_string()).filter(|s| !s.is_empty()),
            head: Some(er_card(right).to_string()).filter(|s| !s.is_empty()),
            ..Edge::new(ai, bi)
        });
    }
    if g.nodes.is_empty() {
        return None;
    }
    for n in &mut g.nodes {
        n.shape = Shape::Table;
    }
    Some(layered(&g, horizontal, dark))
}

// ---- Graphviz -----------------------------------------------------------------------------------

/// `[a=b, c="d e"]` → pairs.
fn dot_attrs(s: &str) -> Vec<(String, String)> {
    let mut out = Vec::new();
    let c: Vec<char> = s.chars().collect();
    let mut i = 0;
    while i < c.len() {
        while i < c.len() && (c[i].is_whitespace() || c[i] == ',' || c[i] == ';') {
            i += 1;
        }
        let k0 = i;
        while i < c.len() && c[i] != '=' && c[i] != ',' && c[i] != ';' && !c[i].is_whitespace() {
            i += 1;
        }
        let key: String = c[k0..i].iter().collect();
        while i < c.len() && c[i].is_whitespace() {
            i += 1;
        }
        if i < c.len() && c[i] == '=' {
            i += 1;
            while i < c.len() && c[i].is_whitespace() {
                i += 1;
            }
            let val = if i < c.len() && c[i] == '"' {
                i += 1;
                let v0 = i;
                while i < c.len() && c[i] != '"' {
                    if c[i] == '\\' {
                        i += 1;
                    }
                    i += 1;
                }
                let v: String = c[v0..i.min(c.len())].iter().collect();
                i += 1;
                v
            } else {
                let v0 = i;
                while i < c.len() && c[i] != ',' && c[i] != ';' && !c[i].is_whitespace() {
                    i += 1;
                }
                c[v0..i].iter().collect()
            };
            if !key.is_empty() {
                out.push((key, val.replace("\\n", " ")));
            }
        } else if !key.is_empty() {
            out.push((key, String::new()));
        } else {
            i += 1;
        }
    }
    out
}

fn dot_shape(s: &str) -> Shape {
    match s {
        "box" | "rect" | "rectangle" | "square" | "record" | "Mrecord" | "component" | "note" | "tab" | "folder" => Shape::Rect,
        "circle" | "doublecircle" | "point" => Shape::Circle,
        "diamond" => Shape::Diamond,
        "cylinder" => Shape::Cylinder,
        "hexagon" => Shape::Hexagon,
        "plaintext" | "plain" | "none" => Shape::Plain,
        _ => Shape::Ellipse,
    }
}

/// Split DOT into statements (on `;`, new lines and braces, outside quotes and brackets).
fn dot_statements(src: &str) -> Vec<String> {
    let mut out = Vec::new();
    let mut cur = String::new();
    let (mut quote, mut bracket) = (false, 0);
    let mut chars = src.chars().peekable();
    while let Some(ch) = chars.next() {
        // comments
        if !quote && ch == '/' && chars.peek() == Some(&'/') {
            for c in chars.by_ref() {
                if c == '\n' {
                    break;
                }
            }
            out.push(std::mem::take(&mut cur));
            continue;
        }
        if !quote && ch == '#' && cur.trim().is_empty() {
            for c in chars.by_ref() {
                if c == '\n' {
                    break;
                }
            }
            continue;
        }
        match ch {
            '"' => {
                quote = !quote;
                cur.push(ch);
            }
            '[' if !quote => {
                bracket += 1;
                cur.push(ch);
            }
            ']' if !quote => {
                bracket -= 1;
                cur.push(ch);
            }
            ';' | '\n' | '{' | '}' if !quote && bracket == 0 => {
                if ch == '{' {
                    cur.push('{');
                }
                out.push(std::mem::take(&mut cur));
            }
            _ => cur.push(ch),
        }
    }
    out.push(cur);
    out.into_iter().map(|s| s.trim().to_string()).filter(|s| !s.is_empty()).collect()
}

fn dot_id(s: &str) -> String {
    let s = s.trim();
    let s = s.split(':').next().unwrap_or(s); // ports
    s.trim_matches('"').to_string()
}

pub(super) fn dot(code: &str, dark: bool) -> Option<Svg> {
    let mut g = Graph::default();
    let mut horizontal = false;
    let mut node_shape = Shape::Ellipse;
    let mut directed = true;
    let mut seen_graph = false;
    for st in dot_statements(code) {
        let (body, attrs) = match st.find('[') {
            Some(p) if !st[..p].contains('"') || st[..p].matches('"').count() % 2 == 0 => {
                (st[..p].trim().to_string(), dot_attrs(st[p + 1..].trim_end().trim_end_matches(']')))
            }
            _ => (st.clone(), Vec::new()),
        };
        let lower = body.to_ascii_lowercase();
        if !seen_graph && body.ends_with('{') {
            seen_graph = true;
            directed = lower.contains("digraph");
            continue;
        }
        if body.ends_with('{') || lower.starts_with("subgraph") {
            continue;
        }
        if body == "graph" || body == "edge" {
            for (k, v) in &attrs {
                if k == "rankdir" {
                    horizontal = v == "LR" || v == "RL";
                }
            }
            continue;
        }
        if body == "node" {
            for (k, v) in &attrs {
                if k == "shape" {
                    node_shape = dot_shape(v);
                }
            }
            continue;
        }
        if let Some((k, v)) = body.split_once('=')
            && !body.contains("->")
            && !body.contains("--")
        {
            if k.trim() == "rankdir" {
                let v = v.trim().trim_matches('"');
                horizontal = v == "LR" || v == "RL";
            }
            continue;
        }
        let op = if body.contains("->") { "->" } else { "--" };
        let parts: Vec<&str> = body.split(op).collect();
        let get = |k: &str| attrs.iter().find(|(a, _)| a == k).map(|(_, v)| v.clone());
        if parts.len() == 1 {
            let id = dot_id(parts[0]);
            if id.is_empty() {
                continue;
            }
            let i = g.node(&id, node_shape);
            if let Some(l) = get("label") {
                g.nodes[i].label = l.replace(['{', '}'], "").replace('|', " · ");
            }
            if let Some(s) = get("shape") {
                g.nodes[i].shape = dot_shape(&s);
            }
            if let Some(c) = get("fillcolor").or_else(|| get("color").filter(|_| get("style").is_some_and(|s| s.contains("filled")))) {
                g.nodes[i].fill = Some(c);
            }
            continue;
        }
        let mut prev: Option<Vec<usize>> = None;
        for p in parts {
            // `{a b}` groups
            let ids: Vec<usize> = p
                .trim()
                .trim_matches(|c| c == '{' || c == '}')
                .split(|c: char| c.is_whitespace() || c == ',')
                .map(dot_id)
                .filter(|s| !s.is_empty())
                .map(|id| g.node(&id, node_shape))
                .collect();
            if let Some(prev) = &prev {
                for a in prev {
                    for b in &ids {
                        let style = get("style").unwrap_or_default();
                        let dir = get("dir").unwrap_or_default();
                        g.edges.push(Edge {
                            label: get("label").or_else(|| get("xlabel")),
                            dotted: style.contains("dashed") || style.contains("dotted"),
                            thick: style.contains("bold"),
                            end: if directed && dir != "none" && dir != "back" { Mark::Arrow } else { Mark::None },
                            start: if dir == "back" || dir == "both" { Mark::Arrow } else { Mark::None },
                            ..Edge::new(*a, *b)
                        });
                    }
                }
            }
            prev = Some(ids);
        }
    }
    if g.nodes.is_empty() {
        return None;
    }
    Some(layered(&g, horizontal, dark))
}

// ---- layout and drawing -------------------------------------------------------------------------

const FS: f32 = 13.0;
const ROW: f32 = 18.0;

fn node_size(v: &Node) -> (f32, f32) {
    let tw = text_w(&v.label, FS).max(if v.label.is_empty() { 0.0 } else { 24.0 });
    match v.shape {
        Shape::Diamond if v.label.is_empty() => (28.0, 28.0),
        Shape::Diamond => (tw + 40.0, 56.0f32.max(tw * 0.45 + 30.0)),
        Shape::Circle => {
            let d = tw + 24.0;
            (d, d)
        }
        Shape::Ellipse => (tw + 40.0, 42.0),
        Shape::Cylinder => (tw + 28.0, 50.0),
        Shape::Start | Shape::End => (20.0, 20.0),
        Shape::Plain => (tw + 8.0, 24.0),
        Shape::Rect if v.label.is_empty() => (60.0, 8.0),
        Shape::Table => {
            let body_w = v.body.iter().flatten().map(|l| text_w(l, 12.0)).fold(0.0, f32::max);
            let w = (text_w(&v.label, FS) + 28.0).max(body_w + 20.0).max(80.0);
            let rows: usize = v.body.iter().map(|c| c.len().max(1)).sum();
            let h = 30.0 + rows as f32 * ROW + v.body.len() as f32 * 6.0;
            (w, if v.body.is_empty() { 34.0 } else { h })
        }
        _ => (tw + 28.0, 40.0),
    }
}

/// Lay the nodes out in layers along the edges and draw them.
pub(super) fn layered(g: &Graph, horizontal: bool, dark: bool) -> Svg {
    let ink = ink(dark);
    let (nodes, edges) = (&g.nodes, &g.edges);
    let n = nodes.len();
    // layers: longest path over the edges that do not close a cycle
    let mut back = vec![false; edges.len()];
    {
        let mut state = vec![0u8; n];
        let mut out: Vec<Vec<usize>> = vec![Vec::new(); n];
        for (k, e) in edges.iter().enumerate() {
            out[e.from].push(k);
        }
        fn dfs(v: usize, edges: &[Edge], out: &[Vec<usize>], state: &mut [u8], back: &mut [bool]) {
            state[v] = 1;
            for &k in &out[v] {
                let e = &edges[k];
                match state[e.to] {
                    0 => dfs(e.to, edges, out, state, back),
                    1 => back[k] = true,
                    _ => {}
                }
            }
            state[v] = 2;
        }
        for v in 0..n {
            if state[v] == 0 {
                dfs(v, edges, &out, &mut state, &mut back);
            }
        }
    }
    let mut layer = vec![0usize; n];
    for _ in 0..n {
        let mut changed = false;
        for (k, e) in edges.iter().enumerate() {
            if !back[k] && e.from != e.to && layer[e.to] < layer[e.from] + 1 {
                layer[e.to] = layer[e.from] + 1;
                changed = true;
            }
        }
        if !changed {
            break;
        }
    }
    // edges that span several layers pass through virtual nodes, one per layer, so they can
    // go around the boxes in between; back edges are routed the same way, reversed
    let mut sizes: Vec<(f32, f32)> = nodes.iter().map(node_size).collect();
    let mut layer_of = layer.clone();
    let mut chains: Vec<Vec<usize>> = Vec::with_capacity(edges.len());
    let mut links: Vec<(usize, usize)> = Vec::new();
    for (k, e) in edges.iter().enumerate() {
        let (a, b) = if back[k] && layer_of[e.to] < layer_of[e.from] { (e.to, e.from) } else { (e.from, e.to) };
        let mut chain = vec![a];
        if a != b && layer_of[b] > layer_of[a] + 1 {
            for l in layer_of[a] + 1..layer_of[b] {
                sizes.push(if horizontal { (0.0, 14.0) } else { (14.0, 0.0) });
                layer_of.push(l);
                chain.push(sizes.len() - 1);
            }
        }
        chain.push(b);
        for w in chain.windows(2) {
            if w[0] != w[1] && layer_of[w[1]] == layer_of[w[0]] + 1 {
                links.push((w[0], w[1]));
            }
        }
        if a != e.from {
            chain.reverse();
        }
        chains.push(chain);
    }
    let total = sizes.len();
    let layers = layer_of.iter().max().copied().unwrap_or(0) + 1;
    let mut rows: Vec<Vec<usize>> = vec![Vec::new(); layers];
    for v in 0..total {
        rows[layer_of[v]].push(v);
    }
    // order by the average position of the neighbours (down and up sweeps)
    let mut ups: Vec<Vec<usize>> = vec![Vec::new(); total];
    let mut downs: Vec<Vec<usize>> = vec![Vec::new(); total];
    for (a, b) in &links {
        downs[*a].push(*b);
        ups[*b].push(*a);
    }
    let sweep = |rows: &mut Vec<Vec<usize>>, li: usize, other: usize, nb: &Vec<Vec<usize>>| {
        let pos: HashMap<usize, f32> = rows[other].iter().enumerate().map(|(i, v)| (*v, i as f32)).collect();
        let mut keyed: Vec<(f32, usize)> = rows[li]
            .iter()
            .enumerate()
            .map(|(i, v)| {
                let ps: Vec<f32> = nb[*v].iter().filter_map(|u| pos.get(u).copied()).collect();
                (if ps.is_empty() { i as f32 } else { ps.iter().sum::<f32>() / ps.len() as f32 }, *v)
            })
            .collect();
        keyed.sort_by(|a, b| a.0.total_cmp(&b.0));
        rows[li] = keyed.into_iter().map(|(_, v)| v).collect();
    };
    for _ in 0..4 {
        for li in 1..layers {
            sweep(&mut rows, li, li - 1, &ups);
        }
        for li in (0..layers.saturating_sub(1)).rev() {
            sweep(&mut rows, li, li + 1, &downs);
        }
    }
    let labelled = edges.iter().any(|e| e.label.is_some() || e.tail.is_some() || e.head.is_some());
    let (gap_main, gap_cross) = match (horizontal, labelled) {
        (true, true) => (110.0, 30.0),
        (true, false) => (70.0, 26.0),
        (false, true) => (74.0, 40.0),
        (false, false) => (56.0, 30.0),
    };
    let layer_main: Vec<f32> =
        rows.iter().map(|r| r.iter().map(|v| if horizontal { sizes[*v].0 } else { sizes[*v].1 }).fold(0.0, f32::max)).collect();
    let layer_cross: Vec<f32> = rows
        .iter()
        .map(|r| {
            r.iter().map(|v| if horizontal { sizes[*v].1 } else { sizes[*v].0 }).sum::<f32>()
                + gap_cross * (r.len().saturating_sub(1)) as f32
        })
        .collect();
    let cross_total = layer_cross.iter().copied().fold(0.0, f32::max);
    let mut center = vec![(0.0f32, 0.0f32); total];
    let pad = 16.0;
    let mut main = pad;
    for (li, r) in rows.iter().enumerate() {
        let mut cross = pad + (cross_total - layer_cross[li]) / 2.0;
        for v in r {
            let (w, h) = sizes[*v];
            center[*v] =
                if horizontal { (main + layer_main[li] / 2.0, cross + h / 2.0) } else { (cross + w / 2.0, main + layer_main[li] / 2.0) };
            cross += if horizontal { h } else { w } + gap_cross;
        }
        main += layer_main[li] + gap_main;
    }
    let main_total = main - gap_main + pad;
    let (w, h) = if horizontal { (main_total, cross_total + pad * 2.0) } else { (cross_total + pad * 2.0, main_total) };
    let mut s = header(w, h, &ink);
    let m = ink.muted;
    let bg = ink.bg;
    let _ = write!(
        s,
        "<defs>\
<marker id=\"ah\" viewBox=\"0 0 10 10\" refX=\"9\" refY=\"5\" markerWidth=\"7\" markerHeight=\"7\" orient=\"auto-start-reverse\"><path d=\"M0,0 L10,5 L0,10 z\" fill=\"{m}\"/></marker>\
<marker id=\"op\" viewBox=\"0 0 10 10\" refX=\"9\" refY=\"5\" markerWidth=\"8\" markerHeight=\"8\" orient=\"auto-start-reverse\"><path d=\"M0,0 L10,5 L0,10\" fill=\"none\" stroke=\"{m}\" stroke-width=\"1.5\"/></marker>\
<marker id=\"tri\" viewBox=\"0 0 12 12\" refX=\"11\" refY=\"6\" markerWidth=\"11\" markerHeight=\"11\" orient=\"auto-start-reverse\"><path d=\"M1,1 L11,6 L1,11 z\" fill=\"{bg}\" stroke=\"{m}\" stroke-width=\"1.2\"/></marker>\
<marker id=\"dia\" viewBox=\"0 0 16 10\" refX=\"15\" refY=\"5\" markerWidth=\"13\" markerHeight=\"9\" orient=\"auto-start-reverse\"><path d=\"M1,5 L8,1 L15,5 L8,9 z\" fill=\"{m}\"/></marker>\
<marker id=\"odia\" viewBox=\"0 0 16 10\" refX=\"15\" refY=\"5\" markerWidth=\"13\" markerHeight=\"9\" orient=\"auto-start-reverse\"><path d=\"M1,5 L8,1 L15,5 L8,9 z\" fill=\"{bg}\" stroke=\"{m}\" stroke-width=\"1.2\"/></marker>\
</defs>"
    );
    // where a line from the centre towards (tx, ty) leaves a node's box
    let border = |v: usize, tx: f32, ty: f32| -> (f32, f32) {
        let (cx, cy) = center[v];
        let (w, h) = sizes[v];
        let (dx, dy) = (tx - cx, ty - cy);
        if dx == 0.0 && dy == 0.0 {
            return (cx, cy);
        }
        if matches!(nodes[v].shape, Shape::Circle | Shape::Start | Shape::End | Shape::Ellipse | Shape::Diamond) {
            // ellipse (close enough for a diamond too)
            let (a, b) = (w / 2.0, h / 2.0);
            let k = 1.0 / ((dx * dx) / (a * a) + (dy * dy) / (b * b)).sqrt();
            return (cx + dx * k, cy + dy * k);
        }
        let sx = if dx != 0.0 { (w / 2.0) / dx.abs() } else { f32::MAX };
        let sy = if dy != 0.0 { (h / 2.0) / dy.abs() } else { f32::MAX };
        let k = sx.min(sy);
        (cx + dx * k, cy + dy * k)
    };
    let marker = |m: Mark| match m {
        Mark::None => None,
        Mark::Arrow => Some("ah"),
        Mark::Open => Some("op"),
        Mark::Triangle => Some("tri"),
        Mark::Diamond => Some("dia"),
        Mark::OpenDiamond => Some("odia"),
    };
    // edges between the same two nodes are spread apart
    let mut pair_count: HashMap<(usize, usize), usize> = HashMap::new();
    let mut pair_total: HashMap<(usize, usize), usize> = HashMap::new();
    for e in edges {
        *pair_total.entry((e.from.min(e.to), e.from.max(e.to))).or_insert(0) += 1;
    }
    let mut labels = String::new();
    let mut ext = [0.0f32, 0.0, w, h];
    for (k, e) in edges.iter().enumerate() {
        let chain = &chains[k];
        let key = (e.from.min(e.to), e.from.max(e.to));
        let nth = *pair_count.entry(key).and_modify(|c| *c += 1).or_insert(0);
        let dash = if e.dotted { " stroke-dasharray=\"5 4\"" } else { "" };
        let sw = if e.thick { 2.6 } else { 1.5 };
        let mk = marker(e.end).map(|id| format!(" marker-end=\"url(#{id})\"")).unwrap_or_default();
        let ms = marker(e.start).map(|id| format!(" marker-start=\"url(#{id})\"")).unwrap_or_default();
        if e.from == e.to {
            // a loop on the node's right side
            let (cx, cy) = center[e.from];
            let (nw, nh) = sizes[e.from];
            let x = cx + nw / 2.0;
            let _ = write!(
                s,
                "<path d=\"M{x:.1},{:.1} C{:.1},{:.1} {:.1},{:.1} {x:.1},{:.1}\" fill=\"none\" stroke=\"{m}\" stroke-width=\"{sw}\"{dash}{mk}/>",
                cy - nh * 0.25,
                x + 40.0,
                cy - nh * 0.6,
                x + 40.0,
                cy + nh * 0.6,
                cy + nh * 0.25
            );
            if let Some(l) = &e.label {
                edge_label(&mut labels, l, x + 34.0, cy, &ink, false, &mut ext);
            }
            continue;
        }
        // the points the edge passes: its virtual nodes
        let via: Vec<(f32, f32)> = chain[1..chain.len() - 1].iter().map(|v| center[*v]).collect();
        let (tx, ty) = via.first().copied().unwrap_or(center[e.to]);
        let (fx, fy) = via.last().copied().unwrap_or(center[e.from]);
        let (ax, ay) = border(e.from, tx, ty);
        let (bx, by) = border(e.to, fx, fy);
        // a parallel edge bends to one side
        let siblings = pair_total.get(&key).copied().unwrap_or(1);
        // edges between the same nodes bend apart, the same way whichever way they point
        let flip = if e.from < e.to { 1.0 } else { -1.0 };
        let bend = if siblings < 2 || !via.is_empty() { 0.0 } else { (nth as f32 - (siblings - 1) as f32 / 2.0) * 34.0 * flip };
        let (mut mx, mut my) = ((ax + bx) / 2.0, (ay + by) / 2.0);
        let len = ((bx - ax).powi(2) + (by - ay).powi(2)).sqrt().max(1.0);
        let (nx, ny) = (-(by - ay) / len, (bx - ax) / len);
        let (qx, qy) = (mx + nx * bend * 2.0, my + ny * bend * 2.0);
        if !via.is_empty() {
            // a smooth path through the virtual nodes
            let mut pts = vec![(ax, ay)];
            pts.extend(via.iter().copied());
            pts.push((bx, by));
            let mut d = format!("M{ax:.1},{ay:.1}");
            for i in 1..pts.len() - 1 {
                let (px, py) = pts[i];
                let (nx2, ny2) = ((px + pts[i + 1].0) / 2.0, (py + pts[i + 1].1) / 2.0);
                if i == pts.len() - 2 {
                    let _ = write!(d, " Q{px:.1},{py:.1} {bx:.1},{by:.1}");
                } else {
                    let _ = write!(d, " Q{px:.1},{py:.1} {nx2:.1},{ny2:.1}");
                }
            }
            let _ = write!(s, "<path d=\"{d}\" fill=\"none\" stroke=\"{m}\" stroke-width=\"{sw}\"{dash}{mk}{ms}/>");
            (mx, my) = via[via.len() / 2];
        } else if bend == 0.0 {
            let _ = write!(
                s,
                "<line x1=\"{ax:.1}\" y1=\"{ay:.1}\" x2=\"{bx:.1}\" y2=\"{by:.1}\" stroke=\"{m}\" stroke-width=\"{sw}\"{dash}{mk}{ms}/>"
            );
        } else {
            let _ = write!(
                s,
                "<path d=\"M{ax:.1},{ay:.1} Q{qx:.1},{qy:.1} {bx:.1},{by:.1}\" fill=\"none\" stroke=\"{m}\" stroke-width=\"{sw}\"{dash}{mk}{ms}/>"
            );
        }
        if let Some(l) = &e.label
            && !l.is_empty()
        {
            if bend == 0.0 {
                edge_label(&mut labels, l, mx, my, &ink, true, &mut ext);
            } else {
                // beside the curve's outer side
                let (lx, ly) = (mx + nx * bend, my + ny * bend);
                let lw = text_w(l, 11.0) + 10.0;
                let side = |v: f32| if v * bend >= 0.0 { 1.0 } else { -1.0 };
                if nx.abs() > ny.abs() {
                    let x0 = if side(nx) > 0.0 { lx + 4.0 } else { lx - 4.0 - lw };
                    edge_label(&mut labels, l, x0, ly, &ink, false, &mut ext);
                } else {
                    edge_label(&mut labels, l, lx, ly + side(ny) * 12.0, &ink, true, &mut ext);
                }
            }
        }
        // cardinalities near the ends, off to the side of the line
        let card = |out: &mut String, t: &str, px: f32, py: f32, toward: (f32, f32), ext: &mut [f32; 4]| {
            let (dx, dy) = (toward.0 - px, toward.1 - py);
            let d = (dx * dx + dy * dy).sqrt().max(1.0);
            let (ux, uy) = (dx / d, dy / d);
            let (x, y) = (px + ux * 18.0 - uy * 12.0, py + uy * 18.0 + ux * 12.0);
            grow(ext, x - text_w(t, 11.0) / 2.0, y - 8.0, x + text_w(t, 11.0) / 2.0, y + 8.0);
            let _ = write!(
                out,
                "<text x=\"{x:.1}\" y=\"{:.1}\" text-anchor=\"middle\" font-size=\"11\" fill=\"{m}\">{}</text>",
                y + 4.0,
                esc(t)
            );
        };
        if let Some(t) = &e.tail {
            card(&mut labels, t, ax, ay, (tx, ty), &mut ext);
        }
        if let Some(t) = &e.head {
            card(&mut labels, t, bx, by, (fx, fy), &mut ext);
        }
    }
    for (v, nd) in nodes.iter().enumerate() {
        let (cx, cy) = center[v];
        let (w, h) = sizes[v];
        let (x, y) = (cx - w / 2.0, cy - h / 2.0);
        let fill = nd.fill.as_deref().unwrap_or(ink.node);
        let st = format!("fill=\"{fill}\" stroke=\"{}\" stroke-width=\"1.5\"", ink.node_border);
        match nd.shape {
            Shape::Rect if nd.label.is_empty() => {
                let _ = write!(s, "<rect x=\"{x:.1}\" y=\"{y:.1}\" width=\"{w:.1}\" height=\"{h:.1}\" rx=\"2\" fill=\"{}\"/>", ink.fg);
            }
            Shape::Rect => {
                let _ = write!(s, "<rect x=\"{x:.1}\" y=\"{y:.1}\" width=\"{w:.1}\" height=\"{h:.1}\" rx=\"3\" {st}/>");
            }
            Shape::Round => {
                let _ = write!(s, "<rect x=\"{x:.1}\" y=\"{y:.1}\" width=\"{w:.1}\" height=\"{h:.1}\" rx=\"10\" {st}/>");
            }
            Shape::Stadium => {
                let _ = write!(s, "<rect x=\"{x:.1}\" y=\"{y:.1}\" width=\"{w:.1}\" height=\"{h:.1}\" rx=\"{:.1}\" {st}/>", h / 2.0);
            }
            Shape::Circle => {
                let _ = write!(s, "<circle cx=\"{cx:.1}\" cy=\"{cy:.1}\" r=\"{:.1}\" {st}/>", w / 2.0);
            }
            Shape::Ellipse => {
                let _ = write!(s, "<ellipse cx=\"{cx:.1}\" cy=\"{cy:.1}\" rx=\"{:.1}\" ry=\"{:.1}\" {st}/>", w / 2.0, h / 2.0);
            }
            Shape::Start => {
                let _ = write!(s, "<circle cx=\"{cx:.1}\" cy=\"{cy:.1}\" r=\"8\" fill=\"{}\"/>", ink.fg);
            }
            Shape::End => {
                let _ = write!(
                    s,
                    "<circle cx=\"{cx:.1}\" cy=\"{cy:.1}\" r=\"9\" fill=\"none\" stroke=\"{}\" stroke-width=\"1.5\"/><circle cx=\"{cx:.1}\" cy=\"{cy:.1}\" r=\"5.5\" fill=\"{}\"/>",
                    ink.fg, ink.fg
                );
            }
            Shape::Plain => {}
            Shape::Diamond => {
                let _ = write!(s, "<polygon points=\"{cx:.1},{y:.1} {:.1},{cy:.1} {cx:.1},{:.1} {x:.1},{cy:.1}\" {st}/>", x + w, y + h);
            }
            Shape::Hexagon => {
                let k = 12.0;
                let _ = write!(
                    s,
                    "<polygon points=\"{:.1},{y:.1} {:.1},{y:.1} {:.1},{cy:.1} {:.1},{:.1} {:.1},{:.1} {x:.1},{cy:.1}\" {st}/>",
                    x + k,
                    x + w - k,
                    x + w,
                    x + w - k,
                    y + h,
                    x + k,
                    y + h
                );
            }
            Shape::Cylinder => {
                let ry = 6.0;
                let _ = write!(
                    s,
                    "<path d=\"M{x:.1},{:.1} L{x:.1},{:.1} A{:.1},{ry} 0 0 0 {:.1},{:.1} L{:.1},{:.1} A{:.1},{ry} 0 0 0 {x:.1},{:.1} Z\" {st}/><path d=\"M{x:.1},{:.1} A{:.1},{ry} 0 0 0 {:.1},{:.1}\" fill=\"none\" stroke=\"{}\" stroke-width=\"1.5\"/>",
                    y + ry,
                    y + h - ry,
                    w / 2.0,
                    x + w,
                    y + h - ry,
                    x + w,
                    y + ry,
                    w / 2.0,
                    y + ry,
                    y + ry,
                    w / 2.0,
                    x + w,
                    y + ry,
                    ink.node_border
                );
            }
            Shape::Table => {
                let _ = write!(
                    s,
                    "<rect x=\"{x:.1}\" y=\"{y:.1}\" width=\"{w:.1}\" height=\"{h:.1}\" rx=\"3\" fill=\"{}\" stroke=\"{}\" stroke-width=\"1.5\"/>",
                    ink.bg, ink.node_border
                );
                let head_h = if nd.body.is_empty() { h } else { 30.0 };
                let _ = write!(s, "<rect x=\"{x:.1}\" y=\"{y:.1}\" width=\"{w:.1}\" height=\"{head_h:.1}\" rx=\"3\" {st}/>");
                let _ = write!(
                    s,
                    "<text x=\"{cx:.1}\" y=\"{:.1}\" text-anchor=\"middle\" font-size=\"{FS}\" font-weight=\"600\">{}</text>",
                    if nd.body.is_empty() { cy + 4.5 } else { y + 20.0 },
                    esc(&nd.label)
                );
                let mut yy = y + 30.0;
                for (ci, comp) in nd.body.iter().enumerate() {
                    if ci > 0 {
                        let _ = write!(
                            s,
                            "<line x1=\"{x:.1}\" y1=\"{yy:.1}\" x2=\"{:.1}\" y2=\"{yy:.1}\" stroke=\"{}\" stroke-width=\"1\"/>",
                            x + w,
                            ink.node_border
                        );
                    }
                    yy += 3.0;
                    for row in comp {
                        yy += ROW;
                        let _ = write!(s, "<text x=\"{:.1}\" y=\"{:.1}\" font-size=\"12\">{}</text>", x + 10.0, yy - 5.0, esc(row));
                    }
                    if comp.is_empty() {
                        yy += ROW;
                    }
                    yy += 3.0;
                }
                continue;
            }
        }
        if !nd.label.is_empty() && !matches!(nd.shape, Shape::Start | Shape::End) {
            let _ =
                write!(s, "<text x=\"{cx:.1}\" y=\"{:.1}\" text-anchor=\"middle\" font-size=\"{FS}\">{}</text>", cy + 4.5, esc(&nd.label));
        }
    }
    s.push_str(&labels);
    s.push_str("</svg>");
    // labels that stick out widen the picture
    if ext != [0.0, 0.0, w, h] {
        let (vw, vh) = (ext[2] - ext[0], ext[3] - ext[1]);
        let old = format!("width=\"{w}\" height=\"{h}\" viewBox=\"0 0 {w} {h}\"");
        let new = format!("width=\"{vw}\" height=\"{vh}\" viewBox=\"{} {} {vw} {vh}\"", ext[0], ext[1]);
        s = s.replacen(&old, &new, 1);
        return Svg { svg: s, width: vw, height: vh };
    }
    Svg { svg: s, width: w, height: h }
}

/// Widen the drawing's extent to take in a box.
fn grow(ext: &mut [f32; 4], x0: f32, y0: f32, x1: f32, y1: f32) {
    ext[0] = ext[0].min(x0 - 4.0);
    ext[1] = ext[1].min(y0 - 4.0);
    ext[2] = ext[2].max(x1 + 4.0);
    ext[3] = ext[3].max(y1 + 4.0);
}

fn edge_label(s: &mut String, l: &str, x: f32, y: f32, ink: &super::Ink, centred: bool, ext: &mut [f32; 4]) {
    let lw = text_w(l, 11.0) + 10.0;
    let x0 = if centred { x - lw / 2.0 } else { x };
    grow(ext, x0, y - 9.0, x0 + lw, y + 9.0);
    let _ = write!(
        s,
        "<rect x=\"{x0:.1}\" y=\"{:.1}\" width=\"{lw:.1}\" height=\"18\" rx=\"4\" fill=\"{}\" opacity=\"0.92\"/>",
        y - 9.0,
        ink.bg
    );
    let _ = write!(
        s,
        "<text x=\"{:.1}\" y=\"{:.1}\" text-anchor=\"middle\" font-size=\"11\" fill=\"{}\">{}</text>",
        x0 + lw / 2.0,
        y + 4.0,
        ink.muted,
        esc(l)
    );
}
