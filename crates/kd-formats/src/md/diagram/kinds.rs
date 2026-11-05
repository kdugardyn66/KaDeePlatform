//! Mermaid mind maps, timelines, user journeys, quadrant charts and git graphs; WaveDrom timing
//! diagrams; PlantUML (sequence, class and mind map diagrams).

use std::fmt::Write as _;

use super::{PALETTE, Svg, esc, header, ink, text_w};

/// Text wrapped to lines of at most `width` px.
fn wrap(s: &str, size: f32, width: f32) -> Vec<String> {
    let mut lines = Vec::new();
    let mut cur = String::new();
    for word in s.split_whitespace() {
        let next = if cur.is_empty() { word.to_string() } else { format!("{cur} {word}") };
        if text_w(&next, size) > width && !cur.is_empty() {
            lines.push(std::mem::take(&mut cur));
            cur = word.to_string();
        } else {
            cur = next;
        }
    }
    if !cur.is_empty() {
        lines.push(cur);
    }
    if lines.is_empty() {
        lines.push(String::new());
    }
    lines
}

/// A light tint of a palette colour for boxes.
fn tint(hex: &str, dark: bool) -> String {
    let v = u32::from_str_radix(hex.trim_start_matches('#'), 16).unwrap_or(0x2f5dd6);
    let (r, g, b) = ((v >> 16) & 0xff, (v >> 8) & 0xff, v & 0xff);
    let mix = |c: u32| if dark { (c as f32 * 0.35 + 13.0 * 0.65) as u32 } else { (c as f32 * 0.18 + 255.0 * 0.82) as u32 };
    format!("#{:02x}{:02x}{:02x}", mix(r), mix(g), mix(b))
}

// ---- mind map -----------------------------------------------------------------------------------

struct MNode {
    text: String,
    round: bool,
    children: Vec<usize>,
}

/// Mermaid `mindmap` (indentation) or PlantUML `* / **` lines → nodes, the first is the root.
fn mind_tree(lines: &[(usize, String)]) -> Vec<MNode> {
    let mut nodes: Vec<MNode> = Vec::new();
    let mut stack: Vec<(usize, usize)> = Vec::new(); // (depth, node)
    for (depth, raw) in lines {
        let (text, round) = mind_label(raw);
        if text.is_empty() {
            continue;
        }
        let i = nodes.len();
        nodes.push(MNode { text, round, children: Vec::new() });
        while let Some((d, _)) = stack.last() {
            if *d >= *depth {
                stack.pop();
            } else {
                break;
            }
        }
        if let Some((_, p)) = stack.last() {
            let p = *p;
            nodes[p].children.push(i);
        } else if i > 0 {
            nodes[0].children.push(i);
        }
        stack.push((*depth, i));
    }
    nodes
}

/// `id((text))`, `id[text]`, `id(text)`, `::icon(…)` → the text and whether it is round.
fn mind_label(s: &str) -> (String, bool) {
    let s = s.trim();
    if s.starts_with("::") {
        return (String::new(), false);
    }
    for (open, close, round) in
        [("((", "))", true), ("))", "((", true), ("{{", "}}", false), ("[", "]", false), ("(", ")", true), (")", "(", true)]
    {
        if let Some(a) = s.find(open)
            && let Some(b) = s.rfind(close)
            && b > a + open.len()
        {
            return (s[a + open.len()..b].trim().trim_matches('"').trim_matches('`').to_string(), round);
        }
    }
    (s.trim_matches('"').trim_matches('`').to_string(), false)
}

pub(super) fn mindmap_mermaid(lines: &[&str], code: &str, dark: bool) -> Option<Svg> {
    let _ = lines;
    let mut items = Vec::new();
    let mut started = false;
    for l in code.lines() {
        let t = l.trim();
        if t.is_empty() || t.starts_with("%%") {
            continue;
        }
        if !started {
            started = t == "mindmap";
            continue;
        }
        let depth = l.len() - l.trim_start().len();
        items.push((depth, t.to_string()));
    }
    mindmap(&mind_tree(&items), dark)
}

fn mindmap(nodes: &[MNode], dark: bool) -> Option<Svg> {
    if nodes.is_empty() {
        return None;
    }
    let ink = ink(dark);
    // a tidy tree from left to right on both sides of the root
    let size = |n: &MNode, depth: usize| -> (f32, f32) {
        let fs = if depth == 0 {
            15.0
        } else if depth == 1 {
            13.0
        } else {
            12.0
        };
        (text_w(&n.text, fs) + if depth == 0 { 36.0 } else { 22.0 }, if depth == 0 { 44.0 } else { 28.0 })
    };
    fn height(nodes: &[MNode], i: usize, depth: usize) -> f32 {
        let own = if depth == 0 { 44.0 } else { 28.0 } + 10.0;
        let kids: f32 = nodes[i].children.iter().map(|c| height(nodes, *c, depth + 1)).sum();
        f32::max(own, kids)
    }
    let root = &nodes[0];
    let kids = &root.children;
    // split the branches into right and left, balancing their heights
    let mut right = Vec::new();
    let mut left = Vec::new();
    let (mut hr, mut hl) = (0.0, 0.0);
    for c in kids {
        let h = height(nodes, *c, 1);
        if hr <= hl {
            right.push(*c);
            hr += h;
        } else {
            left.push(*c);
            hl += h;
        }
    }
    struct Placed {
        x: f32,
        y: f32,
        w: f32,
        h: f32,
        depth: usize,
        branch: usize,
        parent: Option<usize>,
        node: usize,
    }
    let mut placed: Vec<Placed> = Vec::new();
    let gap = 46.0;
    #[allow(clippy::too_many_arguments)]
    fn place(
        nodes: &[MNode],
        i: usize,
        depth: usize,
        x_edge: f32,
        top: f32,
        dir: f32,
        branch: usize,
        parent: Option<usize>,
        out: &mut Vec<Placed>,
        size: &dyn Fn(&MNode, usize) -> (f32, f32),
        gap: f32,
    ) {
        let h_all = height(nodes, i, depth);
        let (w, h) = size(&nodes[i], depth);
        let cx = x_edge + dir * w / 2.0;
        let cy = top + h_all / 2.0;
        let me = out.len();
        out.push(Placed { x: cx, y: cy, w, h, depth, branch, parent, node: i });
        let kids_h: f32 = nodes[i].children.iter().map(|c| height(nodes, *c, depth + 1)).sum();
        let mut t = top + (h_all - kids_h) / 2.0;
        for c in &nodes[i].children {
            place(nodes, *c, depth + 1, x_edge + dir * (w + gap), t, dir, branch, Some(me), out, size, gap);
            t += height(nodes, *c, depth + 1);
        }
    }
    let (rw, rh) = size(root, 0);
    let total_h = hr.max(hl).max(rh + 20.0);
    placed.push(Placed { x: 0.0, y: total_h / 2.0, w: rw, h: rh, depth: 0, branch: 0, parent: None, node: 0 });
    let mut t = (total_h - hr) / 2.0;
    for (b, c) in right.iter().enumerate() {
        place(nodes, *c, 1, rw / 2.0 + gap, t, 1.0, b, Some(0), &mut placed, &size, gap);
        t += height(nodes, *c, 1);
    }
    let mut t = (total_h - hl) / 2.0;
    for (b, c) in left.iter().enumerate() {
        place(nodes, *c, 1, -(rw / 2.0 + gap), t, -1.0, b + right.len(), Some(0), &mut placed, &size, gap);
        t += height(nodes, *c, 1);
    }
    let min_x = placed.iter().map(|p| p.x - p.w / 2.0).fold(f32::MAX, f32::min) - 16.0;
    let max_x = placed.iter().map(|p| p.x + p.w / 2.0).fold(f32::MIN, f32::max) + 16.0;
    let (w, h) = (max_x - min_x, total_h + 24.0);
    let (ox, oy) = (-min_x, 12.0);
    let mut s = header(w, h, &ink);
    for p in &placed {
        if let Some(par) = p.parent {
            let q = &placed[par];
            let col = PALETTE[p.branch % PALETTE.len()];
            let dir = if p.x >= q.x { 1.0 } else { -1.0 };
            let (x1, y1) = (q.x + ox + dir * q.w / 2.0 * if q.depth == 0 { 0.6 } else { 1.0 }, q.y + oy);
            let (x2, y2) = (p.x + ox - dir * p.w / 2.0, p.y + oy);
            let mx = (x1 + x2) / 2.0;
            let sw = if p.depth == 1 { 3.0 } else { 2.0 };
            let _ = write!(
                s,
                "<path d=\"M{x1:.1},{y1:.1} C{mx:.1},{y1:.1} {mx:.1},{y2:.1} {x2:.1},{y2:.1}\" fill=\"none\" stroke=\"{col}\" stroke-width=\"{sw}\"/>"
            );
        }
    }
    for p in &placed {
        let n = &nodes[p.node];
        let (x, y) = (p.x + ox - p.w / 2.0, p.y + oy - p.h / 2.0);
        if p.depth == 0 {
            let _ = write!(
                s,
                "<rect x=\"{x:.1}\" y=\"{y:.1}\" width=\"{:.1}\" height=\"{:.1}\" rx=\"{:.1}\" fill=\"{}\" stroke=\"{}\" stroke-width=\"2\"/>",
                p.w,
                p.h,
                if n.round { p.h / 2.0 } else { 8.0 },
                ink.node,
                ink.node_border
            );
            let _ = write!(
                s,
                "<text x=\"{:.1}\" y=\"{:.1}\" text-anchor=\"middle\" font-size=\"15\" font-weight=\"600\">{}</text>",
                p.x + ox,
                p.y + oy + 5.0,
                esc(&n.text)
            );
        } else {
            let col = PALETTE[p.branch % PALETTE.len()];
            let fs = if p.depth == 1 { 13.0 } else { 12.0 };
            let _ = write!(
                s,
                "<rect x=\"{x:.1}\" y=\"{y:.1}\" width=\"{:.1}\" height=\"{:.1}\" rx=\"{:.1}\" fill=\"{}\" stroke=\"{col}\" stroke-width=\"1.3\"/>",
                p.w,
                p.h,
                if n.round { p.h / 2.0 } else { 6.0 },
                tint(col, dark)
            );
            let _ = write!(
                s,
                "<text x=\"{:.1}\" y=\"{:.1}\" text-anchor=\"middle\" font-size=\"{fs}\"{}>{}</text>",
                p.x + ox,
                p.y + oy + 4.5,
                if p.depth == 1 { " font-weight=\"600\"" } else { "" },
                esc(&n.text)
            );
        }
    }
    s.push_str("</svg>");
    Some(Svg { svg: s, width: w, height: h })
}

// ---- timeline -----------------------------------------------------------------------------------

pub(super) fn timeline(lines: &[&str], dark: bool) -> Option<Svg> {
    let ink = ink(dark);
    let mut title = None;
    struct Period {
        name: String,
        events: Vec<String>,
        section: usize,
    }
    let mut periods: Vec<Period> = Vec::new();
    let mut sections: Vec<String> = Vec::new();
    for l in &lines[1..] {
        if let Some(t) = l.strip_prefix("title ") {
            title = Some(t.trim().to_string());
            continue;
        }
        if let Some(t) = l.strip_prefix("section ") {
            sections.push(t.trim().to_string());
            continue;
        }
        let mut parts = l.split(" : ").map(str::trim);
        let first = parts.next().unwrap_or("");
        if let Some(ev) = first.strip_prefix(':') {
            // continuation: `: event` under the previous period
            if let Some(p) = periods.last_mut() {
                p.events.push(ev.trim().to_string());
                p.events.extend(parts.map(str::to_string));
            }
            continue;
        }
        let (name, rest): (String, Vec<String>) = if l.contains(" : ") {
            (first.to_string(), parts.map(str::to_string).collect())
        } else if let Some((a, b)) = l.split_once(':') {
            (a.trim().to_string(), b.split(':').map(|x| x.trim().to_string()).collect())
        } else {
            (first.to_string(), Vec::new())
        };
        periods.push(Period {
            name,
            events: rest.into_iter().filter(|e| !e.is_empty()).collect(),
            section: sections.len().saturating_sub(1),
        });
    }
    if periods.is_empty() {
        return None;
    }
    let col_w = 150.0f32;
    let gap = 14.0;
    let n = periods.len() as f32;
    let w = 24.0 + n * col_w + (n - 1.0) * gap + 24.0;
    let mut top = 16.0;
    if title.is_some() {
        top += 30.0;
    }
    let has_sections = !sections.is_empty();
    if has_sections {
        top += 30.0;
    }
    let ev_lines: Vec<Vec<Vec<String>>> = periods.iter().map(|p| p.events.iter().map(|e| wrap(e, 12.0, col_w - 20.0)).collect()).collect();
    let ev_h = |ls: &Vec<String>| ls.len() as f32 * 15.0 + 14.0;
    let col_h = ev_lines.iter().map(|evs| evs.iter().map(ev_h).sum::<f32>() + evs.len() as f32 * 8.0).fold(0.0, f32::max);
    let h = top + 40.0 + 30.0 + col_h + 16.0;
    let mut s = header(w, h, &ink);
    if let Some(t) = &title {
        let _ = write!(s, "<text x=\"{}\" y=\"28\" text-anchor=\"middle\" font-size=\"16\" font-weight=\"600\">{}</text>", w / 2.0, esc(t));
    }
    let x_of = |i: usize| 24.0 + i as f32 * (col_w + gap);
    if has_sections {
        for (si, name) in sections.iter().enumerate() {
            let cols: Vec<usize> = periods.iter().enumerate().filter(|(_, p)| p.section == si).map(|(i, _)| i).collect();
            if let (Some(a), Some(b)) = (cols.first(), cols.last()) {
                let (x0, x1) = (x_of(*a), x_of(*b) + col_w);
                let col = PALETTE[si % PALETTE.len()];
                let _ = write!(
                    s,
                    "<rect x=\"{x0:.1}\" y=\"{:.1}\" width=\"{:.1}\" height=\"24\" rx=\"4\" fill=\"{col}\"/>",
                    top - 30.0,
                    x1 - x0
                );
                let _ = write!(
                    s,
                    "<text x=\"{:.1}\" y=\"{:.1}\" text-anchor=\"middle\" font-size=\"12\" font-weight=\"600\" fill=\"#ffffff\">{}</text>",
                    (x0 + x1) / 2.0,
                    top - 13.5,
                    esc(name)
                );
            }
        }
    }
    // the line through the periods
    let line_y = top + 40.0 + 14.0;
    let _ = write!(
        s,
        "<line x1=\"16\" y1=\"{line_y:.1}\" x2=\"{:.1}\" y2=\"{line_y:.1}\" stroke=\"{}\" stroke-width=\"2\"/>",
        w - 16.0,
        ink.muted
    );
    for (i, p) in periods.iter().enumerate() {
        let x = x_of(i);
        let col = PALETTE[if has_sections { p.section } else { i } % PALETTE.len()];
        let _ = write!(s, "<rect x=\"{x:.1}\" y=\"{top:.1}\" width=\"{col_w}\" height=\"34\" rx=\"6\" fill=\"{col}\"/>");
        let _ = write!(
            s,
            "<text x=\"{:.1}\" y=\"{:.1}\" text-anchor=\"middle\" font-size=\"13\" font-weight=\"600\" fill=\"#ffffff\">{}</text>",
            x + col_w / 2.0,
            top + 22.0,
            esc(&p.name)
        );
        let cx = x + col_w / 2.0;
        let _ = write!(s, "<circle cx=\"{cx:.1}\" cy=\"{line_y:.1}\" r=\"5\" fill=\"{col}\"/>");
        let mut y = line_y + 16.0;
        for ev in &ev_lines[i] {
            let eh = ev_h(ev);
            let _ = write!(
                s,
                "<line x1=\"{cx:.1}\" y1=\"{:.1}\" x2=\"{cx:.1}\" y2=\"{y:.1}\" stroke=\"{col}\" stroke-width=\"1\" stroke-dasharray=\"3 3\"/>",
                y - 8.0
            );
            let _ = write!(
                s,
                "<rect x=\"{:.1}\" y=\"{y:.1}\" width=\"{:.1}\" height=\"{eh:.1}\" rx=\"5\" fill=\"{}\" stroke=\"{col}\" stroke-width=\"1\"/>",
                x + 4.0,
                col_w - 8.0,
                tint(col, dark)
            );
            for (k, l) in ev.iter().enumerate() {
                let _ = write!(
                    s,
                    "<text x=\"{cx:.1}\" y=\"{:.1}\" text-anchor=\"middle\" font-size=\"12\">{}</text>",
                    y + 18.0 + k as f32 * 15.0,
                    esc(l)
                );
            }
            y += eh + 8.0;
        }
    }
    s.push_str("</svg>");
    Some(Svg { svg: s, width: w, height: h })
}

// ---- user journey -------------------------------------------------------------------------------

pub(super) fn journey(lines: &[&str], dark: bool) -> Option<Svg> {
    let ink = ink(dark);
    let mut title = None;
    let mut sections: Vec<String> = Vec::new();
    struct Task {
        name: String,
        score: f32,
        actors: Vec<String>,
        section: usize,
    }
    let mut tasks: Vec<Task> = Vec::new();
    for l in &lines[1..] {
        if let Some(t) = l.strip_prefix("title ") {
            title = Some(t.trim().to_string());
            continue;
        }
        if let Some(t) = l.strip_prefix("section ") {
            sections.push(t.trim().to_string());
            continue;
        }
        let parts: Vec<&str> = l.split(':').map(str::trim).collect();
        if parts.len() >= 2 {
            let score = parts[1].parse::<f32>().unwrap_or(3.0).clamp(0.0, 5.0);
            let actors =
                parts.get(2).map(|a| a.split(',').map(|x| x.trim().to_string()).filter(|x| !x.is_empty()).collect()).unwrap_or_default();
            tasks.push(Task { name: parts[0].to_string(), score, actors, section: sections.len().saturating_sub(1) });
        }
    }
    if tasks.is_empty() {
        return None;
    }
    let mut all_actors: Vec<String> = Vec::new();
    for t in &tasks {
        for a in &t.actors {
            if !all_actors.contains(a) {
                all_actors.push(a.clone());
            }
        }
    }
    let col_w = 130.0f32;
    let gap = 10.0;
    let n = tasks.len() as f32;
    let left = 24.0;
    let w = left + n * col_w + (n - 1.0) * gap + 24.0;
    let mut top = 16.0;
    if title.is_some() {
        top += 30.0;
    }
    let legend_h = if all_actors.is_empty() { 0.0 } else { 26.0 };
    top += legend_h;
    let sec_y = top;
    let task_y = sec_y + if sections.is_empty() { 0.0 } else { 32.0 };
    let names: Vec<Vec<String>> = tasks.iter().map(|t| wrap(&t.name, 12.0, col_w - 16.0)).collect();
    let task_h = names.iter().map(|l| l.len()).max().unwrap_or(1) as f32 * 15.0 + 16.0;
    let chart_top = task_y + task_h + 20.0;
    let chart_h = 150.0;
    let h = chart_top + chart_h + 24.0;
    let mut s = header(w, h, &ink);
    if let Some(t) = &title {
        let _ = write!(s, "<text x=\"{}\" y=\"28\" text-anchor=\"middle\" font-size=\"16\" font-weight=\"600\">{}</text>", w / 2.0, esc(t));
    }
    // actors legend
    let mut lx = left;
    for (i, a) in all_actors.iter().enumerate() {
        let col = PALETTE[(i + 4) % PALETTE.len()];
        let ly = top - legend_h + 10.0;
        let _ = write!(s, "<circle cx=\"{:.1}\" cy=\"{ly:.1}\" r=\"6\" fill=\"{col}\"/>", lx + 6.0);
        let _ = write!(s, "<text x=\"{:.1}\" y=\"{:.1}\" font-size=\"12\">{}</text>", lx + 16.0, ly + 4.0, esc(a));
        lx += 28.0 + text_w(a, 12.0);
    }
    let x_of = |i: usize| left + i as f32 * (col_w + gap);
    for (si, name) in sections.iter().enumerate() {
        let cols: Vec<usize> = tasks.iter().enumerate().filter(|(_, t)| t.section == si).map(|(i, _)| i).collect();
        if let (Some(a), Some(b)) = (cols.first(), cols.last()) {
            let (x0, x1) = (x_of(*a), x_of(*b) + col_w);
            let col = PALETTE[si % PALETTE.len()];
            let _ = write!(s, "<rect x=\"{x0:.1}\" y=\"{sec_y:.1}\" width=\"{:.1}\" height=\"24\" rx=\"4\" fill=\"{col}\"/>", x1 - x0);
            let _ = write!(
                s,
                "<text x=\"{:.1}\" y=\"{:.1}\" text-anchor=\"middle\" font-size=\"12\" font-weight=\"600\" fill=\"#ffffff\">{}</text>",
                (x0 + x1) / 2.0,
                sec_y + 16.5,
                esc(name)
            );
        }
    }
    // dashed guide lines for the scores 1–5
    let score_y = |sc: f32| chart_top + chart_h - (sc - 1.0).max(0.0) / 4.0 * (chart_h - 20.0) - 10.0;
    let mut pts = Vec::new();
    for (i, t) in tasks.iter().enumerate() {
        let x = x_of(i);
        let col = PALETTE[if sections.is_empty() { 0 } else { t.section } % PALETTE.len()];
        let _ = write!(
            s,
            "<rect x=\"{x:.1}\" y=\"{task_y:.1}\" width=\"{col_w}\" height=\"{task_h:.1}\" rx=\"5\" fill=\"{}\" stroke=\"{col}\"/>",
            tint(col, dark)
        );
        for (k, l) in names[i].iter().enumerate() {
            let _ = write!(
                s,
                "<text x=\"{:.1}\" y=\"{:.1}\" text-anchor=\"middle\" font-size=\"12\">{}</text>",
                x + col_w / 2.0,
                task_y + 20.0 + k as f32 * 15.0,
                esc(l)
            );
        }
        // actors on the task
        for (k, a) in t.actors.iter().enumerate() {
            let ai = all_actors.iter().position(|x| x == a).unwrap_or(0);
            let _ = write!(
                s,
                "<circle cx=\"{:.1}\" cy=\"{:.1}\" r=\"4\" fill=\"{}\"/>",
                x + 10.0 + k as f32 * 11.0,
                task_y + task_h - 8.0,
                PALETTE[(ai + 4) % PALETTE.len()]
            );
        }
        let cx = x + col_w / 2.0;
        let _ = write!(
            s,
            "<line x1=\"{cx:.1}\" y1=\"{:.1}\" x2=\"{cx:.1}\" y2=\"{:.1}\" stroke=\"{}\" stroke-dasharray=\"3 4\"/>",
            task_y + task_h,
            chart_top + chart_h,
            ink.grid
        );
        pts.push((cx, score_y(t.score), t.score));
    }
    let path: Vec<String> = pts.iter().map(|(x, y, _)| format!("{x:.1},{y:.1}")).collect();
    let _ = write!(s, "<polyline points=\"{}\" fill=\"none\" stroke=\"{}\" stroke-width=\"1.5\"/>", path.join(" "), ink.muted);
    for (x, y, sc) in pts {
        let col = match sc.round() as i32 {
            5 => "#19a979",
            4 => "#7cc36a",
            3 => "#f2b134",
            2 => "#ee8a3c",
            _ => "#e5484d",
        };
        let _ = write!(s, "<circle cx=\"{x:.1}\" cy=\"{y:.1}\" r=\"14\" fill=\"{col}\"/>");
        // a face: eyes and a mouth that smiles with the score
        let _ = write!(
            s,
            "<circle cx=\"{:.1}\" cy=\"{:.1}\" r=\"1.8\" fill=\"#1f2328\"/><circle cx=\"{:.1}\" cy=\"{:.1}\" r=\"1.8\" fill=\"#1f2328\"/>",
            x - 5.0,
            y - 3.5,
            x + 5.0,
            y - 3.5
        );
        let bend = (sc - 3.0) * 2.5;
        let _ = write!(
            s,
            "<path d=\"M{:.1},{:.1} Q{x:.1},{:.1} {:.1},{:.1}\" fill=\"none\" stroke=\"#1f2328\" stroke-width=\"1.6\"/>",
            x - 6.0,
            y + 5.0,
            y + 5.0 + bend,
            x + 6.0,
            y + 5.0
        );
    }
    s.push_str("</svg>");
    Some(Svg { svg: s, width: w, height: h })
}

// ---- quadrant chart -----------------------------------------------------------------------------

pub(super) fn quadrant(lines: &[&str], dark: bool) -> Option<Svg> {
    let ink = ink(dark);
    let mut title = None;
    let (mut x_lo, mut x_hi, mut y_lo, mut y_hi) = (String::new(), String::new(), String::new(), String::new());
    let mut quads = [String::new(), String::new(), String::new(), String::new()];
    let mut points: Vec<(String, f32, f32)> = Vec::new();
    let axis = |r: &str| -> (String, String) {
        match r.split_once("-->") {
            Some((a, b)) => (a.trim().to_string(), b.trim().to_string()),
            None => (r.trim().to_string(), String::new()),
        }
    };
    for l in &lines[1..] {
        if let Some(t) = l.strip_prefix("title ") {
            title = Some(t.trim().to_string());
        } else if let Some(r) = l.strip_prefix("x-axis ") {
            (x_lo, x_hi) = axis(r);
        } else if let Some(r) = l.strip_prefix("y-axis ") {
            (y_lo, y_hi) = axis(r);
        } else if let Some(r) = l.strip_prefix("quadrant-") {
            if let Some((n, t)) = r.split_once(' ')
                && let Ok(k) = n.parse::<usize>()
                && (1..=4).contains(&k)
            {
                quads[k - 1] = t.trim().to_string();
            }
        } else if let Some((name, rest)) = l.split_once(':')
            && let (Some(a), Some(b)) = (rest.find('['), rest.find(']'))
        {
            let nums: Vec<f32> = rest[a + 1..b].split(',').filter_map(|v| v.trim().parse().ok()).collect();
            if nums.len() == 2 {
                points.push((name.trim().to_string(), nums[0].clamp(0.0, 1.0), nums[1].clamp(0.0, 1.0)));
            }
        }
    }
    if points.is_empty() && quads.iter().all(String::is_empty) {
        return None;
    }
    let size = 420.0f32;
    let left = 40.0;
    let top = if title.is_some() { 46.0 } else { 16.0 };
    let (w, h) = (left + size + 24.0, top + size + 44.0);
    let mut s = header(w, h, &ink);
    if let Some(t) = &title {
        let _ = write!(s, "<text x=\"{}\" y=\"28\" text-anchor=\"middle\" font-size=\"16\" font-weight=\"600\">{}</text>", w / 2.0, esc(t));
    }
    let half = size / 2.0;
    // quadrant 1 top right, 2 top left, 3 bottom left, 4 bottom right
    let cells = [(left + half, top, 0usize), (left, top, 1), (left, top + half, 2), (left + half, top + half, 3)];
    let fills = ["#2f5dd6", "#19a979", "#945ecf", "#e8743b"];
    for (x, y, k) in cells {
        let _ = write!(
            s,
            "<rect x=\"{x:.1}\" y=\"{y:.1}\" width=\"{half:.1}\" height=\"{half:.1}\" fill=\"{}\" stroke=\"{}\"/>",
            tint(fills[k], dark),
            ink.bg
        );
        if !quads[k].is_empty() {
            let _ = write!(
                s,
                "<text x=\"{:.1}\" y=\"{:.1}\" text-anchor=\"middle\" font-size=\"13\" font-weight=\"600\" fill=\"{}\">{}</text>",
                x + half / 2.0,
                y + 24.0,
                ink.muted,
                esc(&quads[k])
            );
        }
    }
    let _ = write!(s, "<rect x=\"{left}\" y=\"{top}\" width=\"{size}\" height=\"{size}\" fill=\"none\" stroke=\"{}\"/>", ink.grid);
    // axes labels
    let by = top + size + 20.0;
    let _ = write!(
        s,
        "<text x=\"{:.1}\" y=\"{by:.1}\" text-anchor=\"middle\" font-size=\"12\" fill=\"{}\">{}</text>",
        left + half / 2.0,
        ink.muted,
        esc(&x_lo)
    );
    let _ = write!(
        s,
        "<text x=\"{:.1}\" y=\"{by:.1}\" text-anchor=\"middle\" font-size=\"12\" fill=\"{}\">{}</text>",
        left + half * 1.5,
        ink.muted,
        esc(&x_hi)
    );
    for (label, cy) in [(&y_lo, top + half * 1.5), (&y_hi, top + half / 2.0)] {
        let _ = write!(
            s,
            "<text x=\"24\" y=\"{cy:.1}\" text-anchor=\"middle\" font-size=\"12\" fill=\"{}\" transform=\"rotate(-90 24 {cy:.1})\">{}</text>",
            ink.muted,
            esc(label)
        );
    }
    for (name, x, y) in &points {
        let (px, py) = (left + x * size, top + (1.0 - y) * size);
        let _ = write!(
            s,
            "<circle cx=\"{px:.1}\" cy=\"{py:.1}\" r=\"6\" fill=\"{}\" stroke=\"{}\" stroke-width=\"1.5\"/>",
            ink.node_border, ink.bg
        );
        let _ = write!(s, "<text x=\"{px:.1}\" y=\"{:.1}\" text-anchor=\"middle\" font-size=\"12\">{}</text>", py + 20.0, esc(name));
    }
    s.push_str("</svg>");
    Some(Svg { svg: s, width: w, height: h })
}

// ---- git graph ----------------------------------------------------------------------------------

pub(super) fn git(lines: &[&str], dark: bool) -> Option<Svg> {
    let ink = ink(dark);
    struct Commit {
        lane: usize,
        parents: Vec<usize>,
        id: String,
        tag: Option<String>,
        merge: bool,
    }
    let mut branches: Vec<String> = vec!["main".into()];
    let mut heads: Vec<Option<usize>> = vec![None];
    let mut cur = 0usize;
    let mut commits: Vec<Commit> = Vec::new();
    let attr = |l: &str, key: &str| -> Option<String> {
        let p = l.find(&format!("{key}:"))?;
        let r = l[p + key.len() + 1..].trim_start();
        let r = r.strip_prefix('"')?;
        Some(r[..r.find('"')?].to_string())
    };
    for l in &lines[1..] {
        let kw = l.split_whitespace().next().unwrap_or("");
        let arg = l.split_whitespace().nth(1).unwrap_or("").trim_matches('"').to_string();
        match kw {
            "commit" => {
                let parents = heads[cur].into_iter().collect();
                commits.push(Commit { lane: cur, parents, id: attr(l, "id").unwrap_or_default(), tag: attr(l, "tag"), merge: false });
                heads[cur] = Some(commits.len() - 1);
            }
            "branch" => {
                if let Some(i) = branches.iter().position(|b| *b == arg) {
                    cur = i;
                } else {
                    branches.push(arg);
                    heads.push(heads[cur]);
                    cur = branches.len() - 1;
                }
            }
            "checkout" | "switch" => {
                if let Some(i) = branches.iter().position(|b| *b == arg) {
                    cur = i;
                }
            }
            "merge" => {
                if let Some(i) = branches.iter().position(|b| *b == arg) {
                    let parents = [heads[cur], heads[i]].into_iter().flatten().collect();
                    commits.push(Commit { lane: cur, parents, id: attr(l, "id").unwrap_or_default(), tag: attr(l, "tag"), merge: true });
                    heads[cur] = Some(commits.len() - 1);
                }
            }
            "cherry-pick" => {
                let parents = heads[cur].into_iter().collect();
                commits.push(Commit { lane: cur, parents, id: attr(l, "id").unwrap_or_default(), tag: attr(l, "tag"), merge: false });
                heads[cur] = Some(commits.len() - 1);
            }
            _ => {}
        }
    }
    if commits.is_empty() {
        return None;
    }
    let label_w = branches.iter().map(|b| text_w(b, 12.0)).fold(0.0, f32::max) + 34.0;
    let (step, lane_h) = (54.0f32, 46.0f32);
    let w = label_w + commits.len() as f32 * step + 30.0;
    let h = 24.0 + branches.len() as f32 * lane_h + 20.0;
    let mut s = header(w, h, &ink);
    let pos = |i: usize| (label_w + 20.0 + i as f32 * step, 30.0 + commits[i].lane as f32 * lane_h);
    for (b, name) in branches.iter().enumerate() {
        let col = PALETTE[b % PALETTE.len()];
        let y = 30.0 + b as f32 * lane_h;
        let on: Vec<usize> = commits.iter().enumerate().filter(|(_, c)| c.lane == b).map(|(i, _)| i).collect();
        if let (Some(a), Some(z)) = (on.first(), on.last()) {
            let _ = write!(
                s,
                "<line x1=\"{:.1}\" y1=\"{y:.1}\" x2=\"{:.1}\" y2=\"{y:.1}\" stroke=\"{col}\" stroke-width=\"3\" opacity=\"0.5\"/>",
                pos(*a).0,
                pos(*z).0
            );
        }
        let _ = write!(s, "<rect x=\"6\" y=\"{:.1}\" width=\"{:.1}\" height=\"22\" rx=\"11\" fill=\"{col}\"/>", y - 11.0, label_w - 16.0);
        let _ = write!(
            s,
            "<text x=\"{:.1}\" y=\"{:.1}\" text-anchor=\"middle\" font-size=\"12\" fill=\"#ffffff\" font-weight=\"600\">{}</text>",
            6.0 + (label_w - 16.0) / 2.0,
            y + 4.0,
            esc(name)
        );
    }
    for (i, c) in commits.iter().enumerate() {
        let (x, y) = pos(i);
        for p in &c.parents {
            let (px, py) = pos(*p);
            if (py - y).abs() > 0.1 {
                let col = PALETTE[if py > y { commits[*p].lane } else { c.lane } % PALETTE.len()];
                let mx = (px + x) / 2.0;
                let _ = write!(
                    s,
                    "<path d=\"M{px:.1},{py:.1} C{mx:.1},{py:.1} {mx:.1},{y:.1} {x:.1},{y:.1}\" fill=\"none\" stroke=\"{col}\" stroke-width=\"3\" opacity=\"0.7\"/>"
                );
            }
        }
    }
    for (i, c) in commits.iter().enumerate() {
        let (x, y) = pos(i);
        let col = PALETTE[c.lane % PALETTE.len()];
        if c.merge {
            let _ = write!(s, "<circle cx=\"{x:.1}\" cy=\"{y:.1}\" r=\"9\" fill=\"{}\" stroke=\"{col}\" stroke-width=\"4\"/>", ink.bg);
        } else {
            let _ = write!(s, "<circle cx=\"{x:.1}\" cy=\"{y:.1}\" r=\"9\" fill=\"{col}\"/>");
        }
        if !c.id.is_empty() {
            let _ = write!(
                s,
                "<text x=\"{x:.1}\" y=\"{:.1}\" text-anchor=\"middle\" font-size=\"10\" fill=\"{}\">{}</text>",
                y + 24.0,
                ink.muted,
                esc(&c.id)
            );
        }
        if let Some(t) = &c.tag {
            let tw = text_w(t, 10.0) + 10.0;
            let _ = write!(
                s,
                "<rect x=\"{:.1}\" y=\"{:.1}\" width=\"{tw:.1}\" height=\"16\" rx=\"3\" fill=\"{}\" stroke=\"{}\"/>",
                x - tw / 2.0,
                y - 30.0,
                ink.node,
                ink.node_border
            );
            let _ = write!(s, "<text x=\"{x:.1}\" y=\"{:.1}\" text-anchor=\"middle\" font-size=\"10\">{}</text>", y - 18.5, esc(t));
        }
    }
    s.push_str("</svg>");
    Some(Svg { svg: s, width: w, height: h })
}

// ---- WaveDrom -----------------------------------------------------------------------------------

pub(super) fn wavedrom(code: &str, dark: bool) -> Option<Svg> {
    let ink = ink(dark);
    let v: serde_json::Value = serde_json::from_str(&super::js_to_json(code)).ok()?;
    let hscale = v.pointer("/config/hscale").and_then(|x| x.as_f64()).unwrap_or(1.0) as f32;
    let mut lanes: Vec<(String, String, Vec<String>, usize)> = Vec::new(); // name, wave, data, group depth
    fn collect(v: &serde_json::Value, depth: usize, out: &mut Vec<(String, String, Vec<String>, usize)>) {
        match v {
            serde_json::Value::Array(a) => {
                // ["group name", lanes…]
                let mut it = a.iter();
                if let Some(serde_json::Value::String(g)) = a.first() {
                    out.push((g.clone(), String::new(), Vec::new(), depth));
                    it.next();
                }
                for x in it {
                    collect(x, depth + 1, out);
                }
            }
            serde_json::Value::Object(o) => {
                let name = o.get("name").and_then(|x| x.as_str()).unwrap_or("").to_string();
                let wave = o.get("wave").and_then(|x| x.as_str()).unwrap_or("").to_string();
                let data = match o.get("data") {
                    Some(serde_json::Value::Array(d)) => d.iter().map(super::value_text).collect(),
                    Some(serde_json::Value::String(d)) => d.split_whitespace().map(str::to_string).collect(),
                    _ => Vec::new(),
                };
                let period = o.get("period").and_then(|x| x.as_u64()).unwrap_or(1).max(1) as usize;
                // a period stretches every step
                let wave = if period > 1 {
                    wave.chars().flat_map(|c| std::iter::once(c).chain(std::iter::repeat_n('.', period - 1))).collect()
                } else {
                    wave
                };
                out.push((name, wave, data, depth));
            }
            _ => out.push((String::new(), String::new(), Vec::new(), depth)),
        }
    }
    for x in v.get("signal")?.as_array()? {
        collect(x, 0, &mut lanes);
    }
    if lanes.is_empty() {
        return None;
    }
    let steps = lanes.iter().map(|l| l.1.chars().count()).max().unwrap_or(0).max(1);
    let cell = 40.0 * hscale;
    let name_w = lanes.iter().map(|l| text_w(&l.0, 12.0) + l.3 as f32 * 10.0).fold(0.0, f32::max) + 20.0;
    let lane_h = 30.0;
    let (amp, top) = (16.0, 20.0);
    let w = name_w + steps as f32 * cell + 20.0;
    let h = top + lanes.len() as f32 * lane_h + 10.0;
    let mut s = header(w, h, &ink);
    // grid
    for i in 0..=steps {
        let x = name_w + i as f32 * cell;
        let _ = write!(
            s,
            "<line x1=\"{x:.1}\" y1=\"{:.1}\" x2=\"{x:.1}\" y2=\"{:.1}\" stroke=\"{}\" stroke-dasharray=\"2 3\"/>",
            top - 10.0,
            h - 6.0,
            ink.grid
        );
    }
    let stroke = ink.node_border;
    for (li, (name, wave, data, depth)) in lanes.iter().enumerate() {
        let base = top + li as f32 * lane_h;
        let (hi, lo, mid) = (base + 4.0, base + 4.0 + amp, base + 4.0 + amp / 2.0);
        if wave.is_empty() {
            // a group heading
            let _ = write!(
                s,
                "<text x=\"{:.1}\" y=\"{:.1}\" font-size=\"12\" font-weight=\"600\" fill=\"{}\">{}</text>",
                8.0 + *depth as f32 * 10.0,
                mid + 4.0,
                ink.muted,
                esc(name)
            );
            continue;
        }
        let _ = write!(s, "<text x=\"{:.1}\" y=\"{:.1}\" font-size=\"12\">{}</text>", 8.0 + *depth as f32 * 10.0, mid + 4.0, esc(name));
        let chars: Vec<char> = wave.chars().collect();
        let mut prev = 'x';
        let mut data_i = 0;
        // bus segments: (start step, end step, kind, label)
        let mut i = 0;
        let mut path = String::new();
        let mut level: Option<f32> = None; // the y of the last drawn point
        while i < chars.len() {
            let mut c = chars[i];
            let x0 = name_w + i as f32 * cell;
            if c == '.' {
                c = prev;
                if matches!(c, 'p' | 'n' | 'P' | 'N') {
                    // clocks repeat
                } else {
                    // extend the previous level
                    if let Some(y) = level
                        && !matches!(c, '=' | '2'..='9' | 'x')
                    {
                        let _ = write!(path, " L{:.1},{y:.1}", x0 + cell);
                    }
                    if matches!(c, '=' | '2'..='9' | 'x') {
                        // buses are drawn as whole segments below
                    }
                    i += 1;
                    continue;
                }
            }
            if c == '|' {
                c = prev;
                i += 1;
                if let Some(y) = level {
                    let _ = write!(path, " L{:.1},{y:.1}", x0 + cell);
                }
                prev = c;
                continue;
            }
            match c {
                'p' | 'P' | 'n' | 'N' => {
                    let (first, second) = if matches!(c, 'p' | 'P') { (hi, lo) } else { (lo, hi) };
                    if level.is_none() {
                        let _ = write!(path, " M{x0:.1},{second:.1}");
                    }
                    let _ = write!(
                        path,
                        " L{x0:.1},{first:.1} L{:.1},{first:.1} L{:.1},{second:.1} L{:.1},{second:.1}",
                        x0 + cell / 2.0,
                        x0 + cell / 2.0,
                        x0 + cell
                    );
                    level = Some(second);
                }
                '0' | 'l' | 'L' | '1' | 'h' | 'H' | 'z' | 'u' | 'd' => {
                    let y = match c {
                        '0' | 'l' | 'L' | 'd' => lo,
                        'z' => mid,
                        _ => hi,
                    };
                    match level {
                        None => {
                            let _ = write!(path, " M{x0:.1},{y:.1}");
                        }
                        Some(py) if (py - y).abs() > 0.1 => {
                            // a short slope between levels
                            let _ = write!(path, " L{:.1},{y:.1}", x0 + if matches!(c, 'L' | 'H') { 0.0 } else { 3.0 });
                            let _ = py;
                        }
                        _ => {}
                    }
                    let _ = write!(path, " L{:.1},{y:.1}", x0 + cell);
                    level = Some(y);
                }
                'x' | '=' | '2'..='9' => {
                    // a bus value until the next non-'.' character
                    let mut j = i + 1;
                    while j < chars.len() && chars[j] == '.' {
                        j += 1;
                    }
                    let x1 = name_w + j as f32 * cell;
                    if !path.is_empty() {
                        let _ = write!(s, "<path d=\"{path}\" fill=\"none\" stroke=\"{stroke}\" stroke-width=\"1.5\"/>");
                        path.clear();
                    }
                    let k = 4.0;
                    let fill = match c {
                        'x' => format!("url(#hatch{})", if dark { "d" } else { "l" }),
                        '=' | '2' => ink.bg.to_string(),
                        _ => tint(PALETTE[(c as usize - '3' as usize + 1) % PALETTE.len()], dark),
                    };
                    let _ = write!(
                        s,
                        "<path d=\"M{x0:.1},{mid:.1} L{:.1},{hi:.1} L{:.1},{hi:.1} L{x1:.1},{mid:.1} L{:.1},{lo:.1} L{:.1},{lo:.1} Z\" fill=\"{fill}\" stroke=\"{stroke}\" stroke-width=\"1.3\"/>",
                        x0 + k,
                        x1 - k,
                        x1 - k,
                        x0 + k
                    );
                    if c != 'x' {
                        if let Some(d) = data.get(data_i) {
                            let _ = write!(
                                s,
                                "<text x=\"{:.1}\" y=\"{:.1}\" text-anchor=\"middle\" font-size=\"11\">{}</text>",
                                (x0 + x1) / 2.0,
                                mid + 4.0,
                                esc(d)
                            );
                        }
                        data_i += 1;
                    }
                    level = None;
                    prev = c;
                    i = j;
                    continue;
                }
                _ => {}
            }
            prev = c;
            i += 1;
        }
        if !path.is_empty() {
            let _ = write!(s, "<path d=\"{path}\" fill=\"none\" stroke=\"{stroke}\" stroke-width=\"1.5\"/>");
        }
    }
    // the hatch for unknown values
    let defs = format!(
        "<defs><pattern id=\"hatch{}\" width=\"6\" height=\"6\" patternUnits=\"userSpaceOnUse\" patternTransform=\"rotate(45)\"><line x1=\"0\" y1=\"0\" x2=\"0\" y2=\"6\" stroke=\"{}\" stroke-width=\"2\"/></pattern></defs>",
        if dark { "d" } else { "l" },
        ink.grid
    );
    let p = s.find('>').map(|p| p + 1).unwrap_or(0);
    s.insert_str(p, &defs);
    s.push_str("</svg>");
    Some(Svg { svg: s, width: w, height: h })
}

// ---- PlantUML -----------------------------------------------------------------------------------

pub(super) fn plantuml(code: &str, dark: bool) -> Option<Svg> {
    let lines: Vec<&str> = code.lines().map(str::trim).filter(|l| !l.is_empty() && !l.starts_with('\'')).collect();
    let start = lines.first().copied().unwrap_or("");
    if start.starts_with("@startmindmap") {
        let items: Vec<(usize, String)> = lines
            .iter()
            .filter(|l| l.starts_with('*') || l.starts_with('+') || l.starts_with('-'))
            .map(|l| {
                let depth = l.chars().take_while(|c| matches!(c, '*' | '+' | '-')).count();
                (depth, l[depth..].trim_start_matches('_').trim().to_string())
            })
            .collect();
        return mindmap(&mind_tree(&items), dark);
    }
    let body: Vec<&str> = lines.iter().copied().filter(|l| !l.starts_with('@')).collect();
    let is_class = body.iter().any(|l| {
        ["class ", "interface ", "abstract ", "enum "].iter().any(|k| l.starts_with(k))
            || ["<|--", "--|>", "<|..", "..|>", "*--", "o--"].iter().any(|op| l.contains(op))
    });
    if is_class {
        return super::graph::class(&body, dark, true);
    }
    // sequence: participants and messages use Mermaid's syntax closely enough
    let mut seq = vec!["sequenceDiagram"];
    seq.extend(body.iter().copied().filter(|l| !l.starts_with("skinparam") && !l.starts_with("title")));
    super::sequence(&seq, dark)
}
