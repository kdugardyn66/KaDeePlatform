//! File-type registry: which KadeeStudio module or Crafting App opens a file,
//! and which filter pill it appears under on the start screen.
//!
//! KadeeStudio modules are checked first, so a PDF opens in Kadee PDF rather
//! than PdfCraft.

use crate::craft::{CRAFT_APPS, CraftApp, craft_for_extension};
use crate::modules::ModuleKind;
use std::path::Path;

/// Filter pills on the start screen ("All" is no group).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum DocGroup {
    Markdown,
    Pdf,
    Dashboard,
    Report,
    Office,
    Graphics,
    Media,
    Cad,
}

impl DocGroup {
    /// In filter-pill order (after "All").
    pub const ALL: [DocGroup; 8] = [
        DocGroup::Office,
        DocGroup::Markdown,
        DocGroup::Pdf,
        DocGroup::Dashboard,
        DocGroup::Report,
        DocGroup::Graphics,
        DocGroup::Media,
        DocGroup::Cad,
    ];

    pub fn label(self) -> &'static str {
        match self {
            DocGroup::Markdown => "Markdown",
            DocGroup::Pdf => "PDF",
            DocGroup::Dashboard => "Dashboard",
            DocGroup::Report => "Report",
            DocGroup::Office => "Office",
            DocGroup::Graphics => "Graphics",
            DocGroup::Media => "Media",
            DocGroup::Cad => "CAD",
        }
    }
}

/// Who opens a file.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Opener {
    Module(ModuleKind),
    Craft(&'static CraftApp),
}

/// The type of a document: its group and the app that opens it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DocKind {
    pub group: DocGroup,
    pub opener: Opener,
}

impl DocKind {
    pub fn from_path(path: &Path) -> Option<DocKind> {
        let ext = path.extension()?.to_str()?.to_ascii_lowercase();
        if let Some(m) = ModuleKind::from_extension(&ext) {
            return Some(DocKind { group: m.group()?, opener: Opener::Module(m) });
        }
        craft_for_extension(&ext).map(|a| DocKind { group: a.group, opener: Opener::Craft(a) })
    }

    /// Badge text.
    pub fn badge(&self) -> &'static str {
        match self.opener {
            Opener::Module(m) => m.badge(),
            Opener::Craft(a) => a.badge,
        }
    }

    /// Name of the app that opens it ("Kadee Markdown", "SolidCraft").
    pub fn app_name(&self) -> &'static str {
        match self.opener {
            Opener::Module(m) => m.name(),
            Opener::Craft(a) => a.name,
        }
    }

    /// Roadmap phase in which KadeeStudio opens it itself.
    pub fn phase(&self) -> u8 {
        match self.opener {
            Opener::Module(m) => m.phase(),
            Opener::Craft(_) => crate::craft::CRAFT_PHASE,
        }
    }

    pub fn is_craft(&self) -> bool {
        matches!(self.opener, Opener::Craft(_))
    }
}

/// Every extension KadeeStudio lists (for the Open dialog filter).
pub fn all_extensions() -> Vec<&'static str> {
    let mut v: Vec<&'static str> = ModuleKind::EXTENSIONS.iter().map(|(e, _)| *e).collect();
    for a in &CRAFT_APPS {
        for e in a.extensions {
            if !v.contains(e) {
                v.push(e);
            }
        }
    }
    v
}

/// The file types of the Open dialog: "All supported documents", then one entry per filter
/// pill with its extensions (`("Markdown", ["md", …])`).
pub fn open_dialog_types() -> Vec<(String, Vec<String>)> {
    let all = all_extensions();
    let mut out = vec![("All supported documents".to_string(), all.iter().map(|e| e.to_string()).collect())];
    for g in DocGroup::ALL {
        let exts: Vec<String> = all
            .iter()
            .filter(|e| DocKind::from_path(Path::new(&format!("x.{e}"))).is_some_and(|k| k.group == g))
            .map(|e| e.to_string())
            .collect();
        if !exts.is_empty() {
            out.push((format!("{} documents", g.label()), exts));
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn modules_win_over_crafting_apps() {
        let k = DocKind::from_path(Path::new("a/Report.PDF")).unwrap();
        assert_eq!(k.opener, Opener::Module(ModuleKind::Pdf));
        assert_eq!(k.group, DocGroup::Pdf);
    }

    #[test]
    fn crafting_app_types_are_recognised() {
        let k = DocKind::from_path(Path::new("bracket.step")).unwrap();
        assert_eq!(k.app_name(), "SolidCraft");
        assert_eq!(k.group, DocGroup::Cad);
        assert_eq!(DocKind::from_path(Path::new("deck.pptx")).unwrap().app_name(), "DeckCraft");
        assert_eq!(DocKind::from_path(Path::new("photo.psd")).unwrap().group, DocGroup::Graphics);
        assert!(DocKind::from_path(Path::new("notes.txt")).is_none());
    }

    #[test]
    fn open_dialog_lists_all_then_each_group() {
        let t = open_dialog_types();
        assert_eq!(t[0].0, "All supported documents");
        assert_eq!(t[0].1.len(), all_extensions().len());
        let md = t.iter().find(|(l, _)| l == "Markdown documents").unwrap();
        assert!(md.1.contains(&"md".to_string()) && !md.1.contains(&"pdf".to_string()));
        assert_eq!(t.len(), 1 + DocGroup::ALL.len());
    }

    #[test]
    fn pills_start_with_office_markdown_pdf() {
        assert_eq!(&DocGroup::ALL[..3], &[DocGroup::Office, DocGroup::Markdown, DocGroup::Pdf]);
    }

    #[test]
    fn every_group_has_at_least_one_type() {
        let exts = all_extensions();
        for g in DocGroup::ALL {
            assert!(exts.iter().any(|e| DocKind::from_path(Path::new(&format!("x.{e}"))).unwrap().group == g), "{g:?} is empty");
        }
    }
}
