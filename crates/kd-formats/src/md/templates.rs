//! Templates: MdCraft's standard ones (built in) and the user's own `.mdd` (document) and
//! `.mds` (slide deck) files. A new document from a template has its placeholders filled in:
//! `{{author}}`, `{{date}}`, `{{date_long}}`, `{{month_year}}`, `{{year}}`, `{{next_year}}`,
//! `{{quarter}}`.

use std::path::{Path, PathBuf};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Kind {
    Document,
    Presentation,
}

impl Kind {
    /// The extension of a user template of this kind.
    pub fn extension(self) -> &'static str {
        match self {
            Kind::Document => "mdd",
            Kind::Presentation => "mds",
        }
    }

    pub fn from_extension(ext: &str) -> Option<Kind> {
        match ext.to_ascii_lowercase().as_str() {
            "mdd" => Some(Kind::Document),
            "mds" | "mdp" => Some(Kind::Presentation),
            _ => None,
        }
    }

    /// What a template of this kind is called.
    pub fn label(self) -> &'static str {
        match self {
            Kind::Document => "Markdown document template",
            Kind::Presentation => "Markdown slide deck template",
        }
    }
}

/// A built-in template.
#[derive(Clone, Debug)]
pub struct Template {
    pub id: &'static str,
    pub name: &'static str,
    pub kind: Kind,
    pub description: &'static str,
    pub theme: &'static str,
    pub source: &'static str,
}

macro_rules! t {
    ($id:literal, $name:literal, $kind:ident, $theme:literal, $file:literal, $desc:literal) => {
        Template {
            id: $id,
            name: $name,
            kind: Kind::$kind,
            description: $desc,
            theme: $theme,
            source: include_str!(concat!("../../templates/", $file)),
        }
    };
}

/// MdCraft's standard templates.
pub static BUILT_IN: &[Template] = &[
    t!("blank", "Blank document", Document, "office", "documents/blank.md", "An empty page with a title, ready to write."),
    t!(
        "welcome",
        "Welcome to Kadee Markdown",
        Document,
        "office",
        "documents/welcome.md",
        "A tour of what Kadee Markdown renders: styles, tables, diagrams, math, charts."
    ),
    t!(
        "report",
        "Business report",
        Document,
        "report",
        "documents/report.md",
        "Cover page, contents, executive summary, findings with charts and tables."
    ),
    t!(
        "proposal",
        "Project proposal",
        Document,
        "office",
        "documents/proposal.md",
        "Problem, solution, scope, timeline (Gantt), budget and sign-off."
    ),
    t!(
        "letter",
        "Formal letter",
        Document,
        "letter",
        "documents/letter.md",
        "A business letter with letterhead, address block and signature."
    ),
    t!("memo", "Memo", Document, "office", "documents/memo.md", "A one-page internal memo with To / From / Subject block."),
    t!(
        "minutes",
        "Meeting minutes",
        Document,
        "office",
        "documents/minutes.md",
        "Attendees, agenda, decisions and an action-item checklist."
    ),
    t!("resume", "Résumé", Document, "modern", "documents/resume.md", "A clean one-page CV: profile, experience, skills, education."),
    t!(
        "spec",
        "Technical specification",
        Document,
        "modern",
        "documents/spec.md",
        "Design doc with architecture diagram, API table, sequence diagram, risks."
    ),
    t!(
        "paper",
        "Research paper",
        Document,
        "classic",
        "documents/paper.md",
        "Academic paper with abstract, equations, figures, footnotes and references."
    ),
    t!(
        "newsletter",
        "Newsletter",
        Document,
        "newsletter",
        "documents/newsletter.md",
        "A team newsletter with a masthead, highlights and two-column stories."
    ),
    t!("blank-deck", "Blank presentation", Presentation, "paper", "slides/blank.md", "A title slide and a content slide."),
    t!(
        "pitch",
        "Pitch deck",
        Presentation,
        "aurora",
        "slides/pitch.md",
        "Problem, solution, market, traction, business model, team, the ask."
    ),
    t!(
        "status",
        "Project status update",
        Presentation,
        "corporate",
        "slides/status.md",
        "RAG status, milestones, risks and next steps for a steering meeting."
    ),
    t!(
        "qbr",
        "Quarterly business review",
        Presentation,
        "paper",
        "slides/qbr.md",
        "KPIs, revenue chart, wins and misses, priorities for next quarter."
    ),
    t!(
        "training",
        "Training workshop",
        Presentation,
        "sunrise",
        "slides/training.md",
        "Agenda, learning objectives, exercises, quiz and wrap-up."
    ),
    t!(
        "tech-talk",
        "Tech talk",
        Presentation,
        "midnight",
        "slides/tech-talk.md",
        "A conference talk with code, architecture diagrams and benchmarks."
    ),
    t!(
        "lecture",
        "Lecture",
        Presentation,
        "chalk",
        "slides/lecture.md",
        "A university lecture with definitions, equations and worked examples."
    ),
    t!(
        "launch",
        "Product launch",
        Presentation,
        "ocean",
        "slides/launch.md",
        "Vision, features, roadmap timeline, pricing and launch plan."
    ),
];

pub fn built_in(id: &str) -> Option<&'static Template> {
    BUILT_IN.iter().find(|t| t.id == id)
}

/// A user template on disk.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct UserTemplate {
    pub path: PathBuf,
    pub name: String,
    pub kind: Kind,
}

/// Whether `path` is a template (`.mdd` or `.mds`).
pub fn kind_of(path: &Path) -> Option<Kind> {
    Kind::from_extension(path.extension()?.to_str()?)
}

/// The templates in `dirs` (each folder's `.mdd` and `.mds` files), sorted by name; the first
/// folder wins when two have a template of the same name.
pub fn user_templates(dirs: &[PathBuf]) -> Vec<UserTemplate> {
    let mut out: Vec<UserTemplate> = Vec::new();
    for dir in dirs {
        let Ok(rd) = std::fs::read_dir(dir) else { continue };
        for e in rd.flatten() {
            let path = e.path();
            let Some(kind) = kind_of(&path) else { continue };
            if !path.is_file() {
                continue;
            }
            let name = path.file_stem().map(|s| s.to_string_lossy().to_string()).unwrap_or_default();
            if !out.iter().any(|t| t.name == name && t.kind == kind) {
                out.push(UserTemplate { path, name, kind });
            }
        }
    }
    out.sort_by_key(|t| t.name.to_lowercase());
    out
}

/// The values of the placeholders.
#[derive(Clone, Debug)]
pub struct Vars {
    pub author: String,
    pub year: i32,
    pub month: u32,
    pub day: u32,
}

const MONTHS: [&str; 12] =
    ["January", "February", "March", "April", "May", "June", "July", "August", "September", "October", "November", "December"];

impl Vars {
    /// Today's values.
    pub fn today(author: &str) -> Vars {
        use chrono::Datelike;
        let d = chrono::Local::now().date_naive();
        Vars { author: author.to_string(), year: d.year(), month: d.month(), day: d.day() }
    }

    pub fn pairs(&self) -> Vec<(&'static str, String)> {
        let month = MONTHS.get(self.month.saturating_sub(1) as usize).copied().unwrap_or("January");
        vec![
            ("author", if self.author.trim().is_empty() { "Your name".into() } else { self.author.clone() }),
            ("date", format!("{:04}-{:02}-{:02}", self.year, self.month, self.day)),
            ("date_long", format!("{} {month} {}", self.day, self.year)),
            ("month_year", format!("{month} {}", self.year)),
            ("year", self.year.to_string()),
            ("next_year", (self.year + 1).to_string()),
            ("quarter", format!("Q{} {}", (self.month.clamp(1, 12) - 1) / 3 + 1, self.year)),
        ]
    }
}

/// `text` with its placeholders filled in.
pub fn fill(text: &str, vars: &Vars) -> String {
    let mut out = text.to_string();
    for (k, v) in vars.pairs() {
        out = out.replace(&format!("{{{{{k}}}}}"), &v);
    }
    out
}

/// The kind of template a document makes: a slide deck when it has slides.
pub fn kind_for(text: &str) -> Kind {
    if super::edit::slide_markers(text).is_empty() { Kind::Document } else { Kind::Presentation }
}

/// A file name for a document made from a template called `name`.
pub fn document_name(name: &str) -> String {
    let n: String = name.chars().filter(|c| !matches!(c, '/' | '\\' | ':' | '*' | '?' | '"' | '<' | '>' | '|')).collect();
    let n = n.trim();
    if n.is_empty() { "Untitled".into() } else { n.to_string() }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn built_ins_have_front_matter_and_known_themes() {
        for t in BUILT_IN {
            assert!(t.source.starts_with("---\n"), "{}", t.id);
            assert!(super::super::library::theme(t.theme).is_some(), "{} has theme {}", t.id, t.theme);
            assert_eq!(
                t.kind == Kind::Presentation,
                !super::super::edit::slide_markers(t.source).is_empty() || t.id == "blank-deck",
                "{}",
                t.id
            );
        }
    }

    #[test]
    fn placeholders_fill() {
        let v = Vars { author: "Ada".into(), year: 2026, month: 11, day: 5 };
        assert_eq!(fill("{{author}} · {{date}} · {{quarter}} · {{date_long}}", &v), "Ada · 2026-11-05 · Q4 2026 · 5 November 2026");
    }

    #[test]
    fn extensions() {
        assert_eq!(kind_of(Path::new("a/b.mdd")), Some(Kind::Document));
        assert_eq!(kind_of(Path::new("b.MDS")), Some(Kind::Presentation));
        assert_eq!(kind_of(Path::new("b.md")), None);
        assert_eq!(kind_for("# A\n<!-- slide -->\n# B"), Kind::Presentation);
    }

    #[test]
    fn user_templates_are_listed() {
        let d = tempfile::tempdir().expect("dir");
        std::fs::write(d.path().join("Report.mdd"), "# R").expect("w");
        std::fs::write(d.path().join("Deck.mds"), "# D").expect("w");
        std::fs::write(d.path().join("notes.md"), "# N").expect("w");
        let t = user_templates(&[d.path().to_path_buf()]);
        assert_eq!(t.iter().map(|t| t.name.as_str()).collect::<Vec<_>>(), ["Deck", "Report"]);
    }
}
