//! Who is asking: the start screen or one of the modules. A scope picks the default model and
//! adds the module's own instructions, so every module gets a model that knows its job and the
//! format its output has to be in.

use serde::{Deserialize, Serialize};

/// The places KaDee AI is used from: KadeeStudio's own modules and the Office documents it opens.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum AppKind {
    Markdown,
    Pdf,
    Dashboard,
    Report,
    Word,
    Sheet,
    Slides,
}

impl AppKind {
    pub const ALL: [AppKind; 7] =
        [AppKind::Markdown, AppKind::Pdf, AppKind::Dashboard, AppKind::Report, AppKind::Word, AppKind::Sheet, AppKind::Slides];

    pub fn id(self) -> &'static str {
        match self {
            AppKind::Markdown => "markdown",
            AppKind::Pdf => "pdf",
            AppKind::Dashboard => "dashboard",
            AppKind::Report => "report",
            AppKind::Word => "word",
            AppKind::Sheet => "sheet",
            AppKind::Slides => "slides",
        }
    }

    pub fn name(self) -> &'static str {
        match self {
            AppKind::Markdown => "Kadee Markdown",
            AppKind::Pdf => "Kadee PDF",
            AppKind::Dashboard => "Kadee Dashboards",
            AppKind::Report => "Kadee Reports",
            AppKind::Word => "Word documents",
            AppKind::Sheet => "Spreadsheets",
            AppKind::Slides => "Presentations",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
pub enum Scope {
    /// The start screen's KaDee AI chat.
    #[default]
    Home,
    /// A module or document type.
    App(AppKind),
}

/// The text format a module wants generated content in, so it can import it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OutputFormat {
    Markdown,
    /// CSV with a header row.
    Csv,
    /// JSON matching [`crate::schemas::deck_outline`].
    DeckOutlineJson,
    /// draw.io XML (`<mxfile>`).
    DrawioXml,
    /// One self-contained HTML file.
    Html,
}

impl Scope {
    pub const ALL: [Scope; 8] = [
        Scope::Home,
        Scope::App(AppKind::Markdown),
        Scope::App(AppKind::Pdf),
        Scope::App(AppKind::Dashboard),
        Scope::App(AppKind::Report),
        Scope::App(AppKind::Word),
        Scope::App(AppKind::Sheet),
        Scope::App(AppKind::Slides),
    ];

    /// Stable id used in `ai.toml` (`home`, `word`, `grid`, ...).
    pub fn id(self) -> &'static str {
        match self {
            Scope::Home => "home",
            Scope::App(k) => k.id(),
        }
    }

    pub fn from_id(id: &str) -> Option<Scope> {
        Scope::ALL.into_iter().find(|s| s.id() == id)
    }

    pub fn label(self) -> &'static str {
        match self {
            Scope::Home => "Start screen (KaDee AI)",
            Scope::App(k) => k.name(),
        }
    }

    /// The format this module imports generated content in.
    pub fn output_format(self) -> OutputFormat {
        match self {
            Scope::App(AppKind::Sheet) => OutputFormat::Csv,
            Scope::App(AppKind::Slides) => OutputFormat::DeckOutlineJson,
            _ => OutputFormat::Markdown,
        }
    }

    /// The module's standing instructions, put in front of every request's own system prompt.
    pub fn system_prompt(self) -> &'static str {
        match self {
            Scope::Home => {
                "You are KaDee AI, the assistant in KaDeePlatform, a native office suite with Kadee Markdown, Kadee PDF, \
                 Kadee Dashboards, Kadee Reports and Convert, and which also opens Word, Excel and PowerPoint files. \
                 Answer clearly and concisely in the user's language. Use Markdown for structure when it helps."
            }
            Scope::App(AppKind::Markdown) => {
                "You are KaDee AI in Kadee Markdown. Return CommonMark/GitHub-flavoured Markdown only (tables, task \
                 lists and fenced code allowed), with no preamble, so it can be inserted as is."
            }
            Scope::App(AppKind::Pdf) => {
                "You are KaDee AI in Kadee PDF, the reading assistant. Answer from the attached PDF; quote the passage and \
                 name the page when you can. Say so plainly when the document doesn't contain the answer. Use Markdown."
            }
            Scope::App(AppKind::Dashboard) => {
                "You are KaDee AI in Kadee Dashboards. Help design dashboards: choose the right chart for the data, name \
                 the measures and dimensions, and keep labels short. Use Markdown, and tables for sample data."
            }
            Scope::App(AppKind::Report) => {
                "You are KaDee AI in Kadee Reports. Write structured reports: a title, a summary, numbered sections and \
                 tables where they help. Return Markdown only, with no preamble."
            }
            Scope::App(AppKind::Word) => {
                "You are KaDee AI working on a Word document. Write clear, well-structured prose in the user's language. \
                 When you produce document content, return Markdown only: # headings, paragraphs, lists and pipe tables, \
                 with no preamble or closing remarks, so it can be inserted into the document as is."
            }
            Scope::App(AppKind::Sheet) => {
                "You are KaDee AI working on a spreadsheet. When you produce table data, return CSV only (comma-separated, \
                 a header row first, quotes around fields that contain commas). Write formulas in Excel syntax \
                 (=SUM(B2:B9)), using only functions that Excel 2010 and LibreOffice both support. Explain briefly when asked."
            }
            Scope::App(AppKind::Slides) => {
                "You are KaDee AI working on a presentation. One message per slide, short titles (at most 8 words), \
                 at most 5 bullets of at most 12 words each, and speaker notes that carry the detail. When you produce a deck \
                 outline, return JSON only, matching the requested schema."
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ids_round_trip_and_cover_every_module() {
        for s in Scope::ALL {
            assert_eq!(Scope::from_id(s.id()), Some(s));
            assert!(!s.system_prompt().is_empty());
        }
        for k in AppKind::ALL {
            assert!(Scope::ALL.contains(&Scope::App(k)), "{} has no AI scope", k.name());
        }
    }
}
