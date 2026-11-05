//! MdCraft's content libraries: what the ribbon's menus insert (diagrams, charts, callouts,
//! code languages, equations, slide layouts) and the document and slide themes.

/// A ready-made code block: ```` ```lang ```` with `code`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Snippet {
    pub id: &'static str,
    pub name: &'static str,
    pub lang: &'static str,
    pub code: &'static str,
}

pub const DIAGRAMS: &[Snippet] = &[
    Snippet {
        id: "flowchart",
        name: "Flowchart",
        lang: "mermaid",
        code: "flowchart LR\n    A[Start] --> B{Decision?}\n    B -- Yes --> C[Do the thing]\n    B -- No --> D[Skip it]\n    C --> E((Done))\n    D --> E",
    },
    Snippet {
        id: "sequence",
        name: "Sequence",
        lang: "mermaid",
        code: "sequenceDiagram\n    autonumber\n    participant U as User\n    participant A as App\n    participant S as Server\n    U->>A: Click \"Save\"\n    A->>S: POST /documents\n    S-->>A: 201 Created\n    A-->>U: Saved ✓",
    },
    Snippet {
        id: "class",
        name: "Class",
        lang: "mermaid",
        code: "classDiagram\n    class Document {\n      +String title\n      +String body\n      +save()\n    }\n    class Template {\n      +String id\n      +instantiate() Document\n    }\n    Template ..> Document : creates",
    },
    Snippet {
        id: "state",
        name: "State",
        lang: "mermaid",
        code: "stateDiagram-v2\n    [*] --> Draft\n    Draft --> Review : submit\n    Review --> Draft : changes requested\n    Review --> Published : approve\n    Published --> [*]",
    },
    Snippet {
        id: "er",
        name: "Entity relationship",
        lang: "mermaid",
        code: "erDiagram\n    CUSTOMER ||--o{ ORDER : places\n    ORDER ||--|{ LINE_ITEM : contains\n    PRODUCT ||--o{ LINE_ITEM : \"ordered in\"",
    },
    Snippet {
        id: "gantt",
        name: "Gantt (timeline)",
        lang: "mermaid",
        code: "gantt\n    title Project plan\n    dateFormat YYYY-MM-DD\n    section Discovery\n    Interviews        :a1, 2026-11-02, 10d\n    Synthesis         :a2, after a1, 5d\n    section Build\n    Prototype         :b1, after a2, 15d\n    Pilot             :b2, after b1, 10d",
    },
    Snippet {
        id: "pie",
        name: "Pie",
        lang: "mermaid",
        code: "pie title Time spent\n    \"Writing\" : 45\n    \"Editing\" : 30\n    \"Meetings\" : 15\n    \"Coffee\" : 10",
    },
    Snippet {
        id: "mindmap",
        name: "Mind map",
        lang: "mermaid",
        code: "mindmap\n  root((Product))\n    Users\n      Writers\n      Teams\n    Features\n      Editor\n      Templates\n      Slides\n    Platforms\n      macOS\n      Windows\n      Linux",
    },
    Snippet {
        id: "timeline",
        name: "Timeline",
        lang: "mermaid",
        code: "timeline\n    title Roadmap\n    Q1 : Research : Hiring\n    Q2 : Beta release\n    Q3 : General availability\n    Q4 : Enterprise features",
    },
    Snippet {
        id: "journey",
        name: "User journey",
        lang: "mermaid",
        code: "journey\n    title Writing a report\n    section Draft\n      Pick a template: 5: Writer\n      Write the content: 3: Writer\n    section Share\n      Export to PDF: 4: Writer\n      Present to the team: 5: Writer, Team",
    },
    Snippet {
        id: "quadrant",
        name: "Quadrant",
        lang: "mermaid",
        code: "quadrantChart\n    title Impact vs effort\n    x-axis Low effort --> High effort\n    y-axis Low impact --> High impact\n    quadrant-1 Plan carefully\n    quadrant-2 Quick wins\n    quadrant-3 Fill-ins\n    quadrant-4 Avoid\n    Templates: [0.3, 0.8]\n    Plugins: [0.75, 0.6]\n    Dark mode: [0.2, 0.35]",
    },
    Snippet {
        id: "git",
        name: "Git graph",
        lang: "mermaid",
        code: "gitGraph\n    commit\n    branch feature\n    checkout feature\n    commit\n    commit\n    checkout main\n    merge feature\n    commit",
    },
    Snippet {
        id: "graphviz",
        name: "Graphviz (DOT)",
        lang: "dot",
        code: "digraph G {\n    rankdir=LR;\n    node [shape=box, style=rounded];\n    Idea -> Draft -> Review -> Publish;\n    Review -> Draft [label=\"edits\"];\n}",
    },
    Snippet {
        id: "plantuml",
        name: "PlantUML",
        lang: "plantuml",
        code: "@startuml\nactor Writer\nWriter -> mdcraft : open template\nmdcraft --> Writer : document\n@enduml",
    },
    Snippet {
        id: "wavedrom",
        name: "Timing (WaveDrom)",
        lang: "wavedrom",
        code: "{ signal: [\n  { name: \"clk\",  wave: \"p......\" },\n  { name: \"req\",  wave: \"0.1..0.\" },\n  { name: \"ack\",  wave: \"0..1.0.\" }\n]}",
    },
];

pub const CHARTS: &[Snippet] = &[
    Snippet {
        id: "bar",
        name: "Bar chart",
        lang: "chart",
        code: "{\n  type: \"bar\",\n  data: {\n    labels: [\"Q1\", \"Q2\", \"Q3\", \"Q4\"],\n    datasets: [\n      { label: \"2025\", data: [12, 19, 15, 22] },\n      { label: \"2026\", data: [16, 23, 21, 30] }\n    ]\n  },\n  options: { plugins: { title: { display: true, text: \"Revenue (k€)\" } } }\n}",
    },
    Snippet {
        id: "line",
        name: "Line chart",
        lang: "chart",
        code: "{\n  type: \"line\",\n  data: {\n    labels: [\"Jan\", \"Feb\", \"Mar\", \"Apr\", \"May\", \"Jun\"],\n    datasets: [{ label: \"Active users\", data: [120, 190, 260, 310, 420, 515], tension: 0.3, fill: true }]\n  },\n  options: { plugins: { title: { display: true, text: \"Growth\" } } }\n}",
    },
    Snippet {
        id: "doughnut",
        name: "Doughnut chart",
        lang: "chart",
        code: "{\n  type: \"doughnut\",\n  data: {\n    labels: [\"Product\", \"Services\", \"Licences\"],\n    datasets: [{ data: [55, 30, 15] }]\n  },\n  options: { plugins: { title: { display: true, text: \"Revenue mix\" } } }\n}",
    },
    Snippet {
        id: "radar",
        name: "Radar chart",
        lang: "chart",
        code: "{\n  type: \"radar\",\n  data: {\n    labels: [\"Speed\", \"Quality\", \"Cost\", \"Scope\", \"Risk\"],\n    datasets: [{ label: \"Plan A\", data: [4, 5, 3, 4, 2] }, { label: \"Plan B\", data: [3, 4, 5, 3, 4] }]\n  }\n}",
    },
    Snippet {
        id: "vega",
        name: "Vega-Lite chart",
        lang: "vega-lite",
        code: "{\n  \"$schema\": \"https://vega.github.io/schema/vega-lite/v5.json\",\n  \"data\": { \"values\": [\n    {\"team\": \"North\", \"sales\": 28}, {\"team\": \"South\", \"sales\": 55},\n    {\"team\": \"East\", \"sales\": 43}, {\"team\": \"West\", \"sales\": 91}\n  ]},\n  \"mark\": \"bar\",\n  \"encoding\": {\n    \"x\": {\"field\": \"team\", \"type\": \"nominal\"},\n    \"y\": {\"field\": \"sales\", \"type\": \"quantitative\"}\n  }\n}",
    },
    Snippet {
        id: "csv",
        name: "Chart from a CSV table",
        lang: "csv {chart=\"bar\" title=\"Units sold\"}",
        code: "Month,Units\nJan,120\nFeb,150\nMar,170\nApr,160",
    },
];

/// `(id, name, text)`: GitHub alerts.
pub const CALLOUTS: &[(&str, &str, &str)] = &[
    ("NOTE", "Note", "Useful information that readers should know."),
    ("TIP", "Tip", "A helpful suggestion for doing things better."),
    ("IMPORTANT", "Important", "Key information readers need to succeed."),
    ("WARNING", "Warning", "Urgent information that needs attention."),
    ("CAUTION", "Caution", "The risks or negative outcomes of an action."),
];

pub const CODE_LANGUAGES: &[(&str, &str)] = &[
    ("python", "Python"),
    ("javascript", "JavaScript"),
    ("typescript", "TypeScript"),
    ("rust", "Rust"),
    ("bash", "Shell (bash)"),
    ("json", "JSON"),
    ("yaml", "YAML"),
    ("sql", "SQL"),
    ("html", "HTML"),
    ("css", "CSS"),
    ("go", "Go"),
    ("java", "Java"),
    ("csharp", "C#"),
    ("cpp", "C++"),
    ("text", "Plain text"),
];

/// `(name, tex)`; `None` is inline math.
pub const EQUATIONS: &[(&str, Option<&str>)] = &[
    ("Inline math", None),
    ("Display equation", Some("\u{1}E = mc^2\u{1}")),
    ("Quadratic formula", Some("x = \\frac{-b \\pm \\sqrt{b^2 - 4ac}}{2a}")),
    ("Sum", Some("\\sum_{i=1}^{n} i = \\frac{n(n+1)}{2}")),
    ("Integral", Some("\\int_0^\\infty e^{-x^2}\\,dx = \\frac{\\sqrt{\\pi}}{2}")),
    ("Matrix", Some("A = \\begin{pmatrix} a & b \\\\ c & d \\end{pmatrix}")),
    ("Aligned equations", Some("\\begin{aligned}\nf(x) &= (x+1)^2 \\\\\n     &= x^2 + 2x + 1\n\\end{aligned}")),
    ("Cases", Some("|x| = \\begin{cases} x & x \\ge 0 \\\\ -x & x < 0 \\end{cases}")),
    ("Chemical equation", Some("\\ce{2H2 + O2 -> 2H2O}")),
];

/// A slide layout: its class and starting text.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Layout {
    pub id: &'static str,
    pub name: &'static str,
    pub cls: &'static str,
    pub body: &'static str,
}

pub const LAYOUTS: &[Layout] = &[
    Layout {
        id: "title",
        name: "Title slide",
        cls: "title",
        body: "# \u{1}Presentation title\u{1}\n\nSubtitle or tagline\n\n**{{author}}** · {{date}}",
    },
    Layout {
        id: "content",
        name: "Title and content",
        cls: "",
        body: "## \u{1}Slide title\u{1}\n\n- First point\n- Second point\n- Third point",
    },
    Layout { id: "section", name: "Section header", cls: "section", body: "# \u{1}Section title\u{1}\n\nWhat this part is about" },
    Layout {
        id: "two",
        name: "Two content",
        cls: "two-col",
        body: "## \u{1}Comparison\u{1}\n\n<div class=\"cols\">\n<div>\n\n### Before\n\n- Point\n- Point\n\n</div>\n<div>\n\n### After\n\n- Point\n- Point\n\n</div>\n</div>",
    },
    Layout {
        id: "agenda",
        name: "Agenda",
        cls: "agenda",
        body: "## \u{1}Agenda\u{1}\n\n1. Where we are\n2. What we learned\n3. What comes next\n4. Questions",
    },
    Layout { id: "quote", name: "Quote", cls: "quote", body: "> \u{1}A sentence worth remembering.\u{1}\n>\n> — Someone wise" },
    Layout { id: "stat", name: "Big number", cls: "stat", body: "# \u{1}87%\u{1}\n\nof readers finish a one-page summary" },
    Layout {
        id: "image",
        name: "Picture with caption",
        cls: "image",
        body: "## \u{1}Title\u{1}\n\n![A description of the picture](images/picture.jpg \"Caption\")",
    },
    Layout { id: "end", name: "Closing", cls: "end", body: "# \u{1}Thank you\u{1}\n\nQuestions? · name@example.com" },
    Layout { id: "blank", name: "Blank", cls: "blank", body: "\u{1}\u{1}" },
];

/// A document or slide theme: colours (dark, accent, background) and its font.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ThemeInfo {
    pub id: &'static str,
    pub name: &'static str,
    pub deck: bool,
    pub colors: [u32; 3],
    pub font: &'static str,
}

pub const THEMES: &[ThemeInfo] = &[
    ThemeInfo { id: "office", name: "Office", deck: false, colors: [0x1f3864, 0x2e74b5, 0xf2f2f2], font: "Source Sans 3" },
    ThemeInfo { id: "report", name: "Report", deck: false, colors: [0x14324a, 0xc0392b, 0xf5f1e8], font: "Source Serif 4" },
    ThemeInfo { id: "classic", name: "Classic", deck: false, colors: [0x111111, 0x555555, 0xffffff], font: "Source Serif 4" },
    ThemeInfo { id: "modern", name: "Modern", deck: false, colors: [0x0f766e, 0x111827, 0xecfdf5], font: "Inter" },
    ThemeInfo { id: "letter", name: "Letter", deck: false, colors: [0x2d3a4a, 0x8a6d3b, 0xfbfaf7], font: "Source Serif 4" },
    ThemeInfo { id: "newsletter", name: "Newsletter", deck: false, colors: [0x7c2d12, 0xea580c, 0xfff7ed], font: "Source Sans 3" },
    ThemeInfo { id: "github", name: "GitHub", deck: false, colors: [0x1f2328, 0x0969da, 0xf6f8fa], font: "system-ui" },
    ThemeInfo { id: "paper", name: "Paper", deck: true, colors: [0xffffff, 0x1d4ed8, 0x111827], font: "Inter" },
    ThemeInfo { id: "aurora", name: "Aurora", deck: true, colors: [0x120c2e, 0x7c3aed, 0x22d3ee], font: "Inter" },
    ThemeInfo { id: "midnight", name: "Midnight", deck: true, colors: [0x0b1220, 0x38bdf8, 0xe2e8f0], font: "Inter" },
    ThemeInfo { id: "sunrise", name: "Sunrise", deck: true, colors: [0xfff7ed, 0xf97316, 0x7c2d12], font: "Source Sans 3" },
    ThemeInfo { id: "corporate", name: "Corporate", deck: true, colors: [0xffffff, 0x1e3a8a, 0x0ea5e9], font: "Source Sans 3" },
    ThemeInfo { id: "chalk", name: "Chalkboard", deck: true, colors: [0x1f3a2e, 0xf1f5d8, 0xfacc15], font: "Source Serif 4" },
    ThemeInfo { id: "ocean", name: "Ocean", deck: true, colors: [0x063a4f, 0x14b8a6, 0xecfeff], font: "Inter" },
];

pub fn theme(id: &str) -> Option<&'static ThemeInfo> {
    THEMES.iter().find(|t| t.id == id)
}

pub const SYMBOLS: &str = "© ® ™ § ¶ † ‡ • · … — – ± × ÷ ≈ ≠ ≤ ≥ ∞ → ← ↑ ↓ ⇒ ⇔ ✓ ✗ ★ ☆ € £ ¥ ¢ ° ‰ µ π Ω α β γ δ ε θ λ σ φ ω Σ Δ √ ∫ ∂ ∇ ∈ ∉ ⊂ ∪ ∩ ∀ ∃ ¼ ½ ¾ ² ³ « » „ “ ” ‘ ’";

/// The text a "New" document starts with.
pub const BLANK: &str = "# \u{1}Untitled\u{1}\n\n";

#[cfg(test)]
mod tests {
    use super::*;
    use crate::md::edit::SEL;

    #[test]
    fn libraries_are_complete() {
        assert_eq!(DIAGRAMS.len(), 15);
        assert_eq!(CHARTS.len(), 6);
        assert!(DIAGRAMS[0].code.starts_with("flowchart LR\n"));
        assert!(LAYOUTS.iter().all(|l| l.body.matches(SEL).count() == 2));
        assert_eq!(THEMES.iter().filter(|t| t.deck).count(), 7);
    }
}
