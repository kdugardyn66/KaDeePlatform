//! The KadeeStudio modules and the file types they own.

use serde::{Deserialize, Serialize};
use std::path::Path;

/// A KadeeStudio module. Phase 0 only shows them on the start screen;
/// the editors arrive in later phases.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum ModuleKind {
    Markdown,
    Pdf,
    Convert,
    Dashboard,
    Report,
}

impl ModuleKind {
    /// All modules in start-screen order.
    pub const ALL: [ModuleKind; 5] =
        [ModuleKind::Markdown, ModuleKind::Pdf, ModuleKind::Convert, ModuleKind::Dashboard, ModuleKind::Report];

    /// Modules with a quick-start tile on the start screen (Convert has none:
    /// it is reached from File › Convert, the right-click menu and the AI).
    pub const TILES: [ModuleKind; 4] = [ModuleKind::Markdown, ModuleKind::Pdf, ModuleKind::Dashboard, ModuleKind::Report];

    /// Extensions owned by KadeeStudio's own modules.
    pub const EXTENSIONS: [(&'static str, ModuleKind); 8] = [
        ("md", ModuleKind::Markdown),
        ("markdown", ModuleKind::Markdown),
        ("mdd", ModuleKind::Markdown),
        ("mds", ModuleKind::Markdown),
        ("mdp", ModuleKind::Markdown),
        ("pdf", ModuleKind::Pdf),
        ("kdash", ModuleKind::Dashboard),
        ("kreport", ModuleKind::Report),
    ];

    /// Product name.
    pub fn name(self) -> &'static str {
        match self {
            ModuleKind::Markdown => "Kadee Markdown",
            ModuleKind::Pdf => "Kadee PDF",
            ModuleKind::Convert => "Kadee Convert",
            ModuleKind::Dashboard => "Kadee Dashboards",
            ModuleKind::Report => "Kadee Reports",
        }
    }

    /// Short label for pills and badges.
    pub fn short(self) -> &'static str {
        match self {
            ModuleKind::Markdown => "Markdown",
            ModuleKind::Pdf => "PDF",
            ModuleKind::Convert => "Convert",
            ModuleKind::Dashboard => "Dashboard",
            ModuleKind::Report => "Report",
        }
    }

    /// One-line description for the quick-start card.
    pub fn tagline(self) -> &'static str {
        match self {
            ModuleKind::Markdown => "Documents and slides in plain text",
            ModuleKind::Pdf => "View, annotate, organise and sign",
            ModuleKind::Convert => "Markdown, PDF, HTML and Office files",
            ModuleKind::Dashboard => "Live charts on your data",
            ModuleKind::Report => "Paginated reports to PDF",
        }
    }

    /// Two-letter badge text.
    pub fn badge(self) -> &'static str {
        match self {
            ModuleKind::Markdown => "MD",
            ModuleKind::Pdf => "PDF",
            ModuleKind::Convert => "CV",
            ModuleKind::Dashboard => "DB",
            ModuleKind::Report => "RP",
        }
    }

    /// Roadmap phase in which the editor ships.
    pub fn phase(self) -> u8 {
        match self {
            ModuleKind::Markdown => 2,
            ModuleKind::Pdf => 4,
            ModuleKind::Convert => 5,
            ModuleKind::Dashboard | ModuleKind::Report => 6,
        }
    }

    /// Stable command-id fragment (`file.new.markdown`).
    pub fn id(self) -> &'static str {
        match self {
            ModuleKind::Markdown => "markdown",
            ModuleKind::Pdf => "pdf",
            ModuleKind::Convert => "convert",
            ModuleKind::Dashboard => "dashboard",
            ModuleKind::Report => "report",
        }
    }

    /// Extension of a new document, if this module creates documents.
    /// Convert works on existing files, so it creates none; a new PDF is one blank A4 page.
    pub fn new_extension(self) -> Option<&'static str> {
        match self {
            ModuleKind::Markdown => Some("md"),
            ModuleKind::Pdf => Some("pdf"),
            ModuleKind::Dashboard => Some("kdash"),
            ModuleKind::Report => Some("kreport"),
            ModuleKind::Convert => None,
        }
    }

    /// Starter content for a new document.
    pub fn starter_content(self, title: &str) -> String {
        match self {
            ModuleKind::Markdown => format!("# {title}\n\n"),
            ModuleKind::Dashboard => format!(
                "{{\n  \"kadee\": \"dashboard\",\n  \"version\": 1,\n  \"title\": \"{title}\",\n  \"sources\": [],\n  \"cards\": []\n}}\n"
            ),
            ModuleKind::Report => format!(
                "{{\n  \"kadee\": \"report\",\n  \"version\": 1,\n  \"title\": \"{title}\",\n  \"page\": \"A4\",\n  \"sections\": []\n}}\n"
            ),
            ModuleKind::Pdf => crate::files::blank_pdf(title),
            ModuleKind::Convert => String::new(),
        }
    }

    /// The module that owns a file, judged by extension.
    pub fn from_path(path: &Path) -> Option<ModuleKind> {
        Self::from_extension(&path.extension()?.to_str()?.to_ascii_lowercase())
    }

    /// The module that owns a lower-case extension.
    pub fn from_extension(ext: &str) -> Option<ModuleKind> {
        Self::EXTENSIONS.iter().find(|(e, _)| *e == ext).map(|(_, m)| *m)
    }

    /// Start-screen filter group of this module's documents (Convert has none).
    pub fn group(self) -> Option<crate::doctype::DocGroup> {
        use crate::doctype::DocGroup;
        match self {
            ModuleKind::Markdown => Some(DocGroup::Markdown),
            ModuleKind::Pdf => Some(DocGroup::Pdf),
            ModuleKind::Dashboard => Some(DocGroup::Dashboard),
            ModuleKind::Report => Some(DocGroup::Report),
            ModuleKind::Convert => None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn detects_module_from_extension() {
        assert_eq!(ModuleKind::from_path(Path::new("a/b.MD")), Some(ModuleKind::Markdown));
        assert_eq!(ModuleKind::from_path(Path::new("x.pdf")), Some(ModuleKind::Pdf));
        assert_eq!(ModuleKind::from_path(Path::new("x.kdash")), Some(ModuleKind::Dashboard));
        assert_eq!(ModuleKind::from_path(Path::new("x.kreport")), Some(ModuleKind::Report));
        assert_eq!(ModuleKind::from_path(Path::new("x.txt")), None);
        assert_eq!(ModuleKind::from_path(Path::new("noext")), None);
    }

    #[test]
    fn new_extension_round_trips() {
        for kind in ModuleKind::ALL {
            if let Some(ext) = kind.new_extension() {
                let p = format!("file.{ext}");
                assert_eq!(ModuleKind::from_path(Path::new(&p)), Some(kind));
            }
        }
    }
}
