//! Blank Office documents: Word (.docx), spreadsheet (.xlsx) and slide deck
//! (.pptx). KadeeStudio creates them; WordCraft, GridCraft and DeckCraft open
//! them (Phase 7).
//!
//! The templates in `assets/templates/` are minimal valid files made with
//! python-docx, openpyxl and python-pptx (all MIT).

use crate::files::unique_path;
use std::fs;
use std::io;
use std::path::{Path, PathBuf};

/// A kind of Office document KadeeStudio can create.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum OfficeKind {
    Word,
    Spreadsheet,
    SlideDeck,
}

impl OfficeKind {
    pub const ALL: [OfficeKind; 3] = [OfficeKind::Word, OfficeKind::Spreadsheet, OfficeKind::SlideDeck];

    /// Stable command-id fragment (`file.new.word`).
    pub fn id(self) -> &'static str {
        match self {
            OfficeKind::Word => "word",
            OfficeKind::Spreadsheet => "spreadsheet",
            OfficeKind::SlideDeck => "slides",
        }
    }

    /// Short label for tiles and menus.
    pub fn short(self) -> &'static str {
        match self {
            OfficeKind::Word => "Word",
            OfficeKind::Spreadsheet => "Spreadsheet",
            OfficeKind::SlideDeck => "Slide deck",
        }
    }

    pub fn extension(self) -> &'static str {
        match self {
            OfficeKind::Word => "docx",
            OfficeKind::Spreadsheet => "xlsx",
            OfficeKind::SlideDeck => "pptx",
        }
    }

    /// Name of a new file, before numbering.
    pub fn base_name(self) -> &'static str {
        match self {
            OfficeKind::Word => "Untitled document",
            OfficeKind::Spreadsheet => "Untitled spreadsheet",
            OfficeKind::SlideDeck => "Untitled slide deck",
        }
    }

    /// The Crafting App that opens it.
    pub fn app(self) -> &'static str {
        match self {
            OfficeKind::Word => "WordCraft",
            OfficeKind::Spreadsheet => "GridCraft",
            OfficeKind::SlideDeck => "DeckCraft",
        }
    }

    /// The blank file's bytes.
    pub fn template(self) -> &'static [u8] {
        match self {
            OfficeKind::Word => include_bytes!("../assets/templates/blank.docx"),
            OfficeKind::Spreadsheet => include_bytes!("../assets/templates/blank.xlsx"),
            OfficeKind::SlideDeck => include_bytes!("../assets/templates/blank.pptx"),
        }
    }
}

/// Create a blank Office document in `dir` (`Untitled document.docx`, numbered if taken).
pub fn create_office_document(kind: OfficeKind, dir: &Path) -> io::Result<PathBuf> {
    fs::create_dir_all(dir)?;
    let path = unique_path(dir, kind.base_name(), kind.extension());
    fs::write(&path, kind.template())?;
    Ok(path)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::doctype::DocKind;

    #[test]
    fn creates_valid_office_files_opened_by_crafting_apps() {
        let tmp = tempfile::tempdir().unwrap();
        for kind in OfficeKind::ALL {
            let p = create_office_document(kind, tmp.path()).unwrap();
            let bytes = fs::read(&p).unwrap();
            // a zip package with the OOXML content-types part
            assert_eq!(&bytes[..2], b"PK", "{kind:?}");
            assert!(bytes.windows(19).any(|w| w == b"[Content_Types].xml"), "{kind:?}");
            assert_eq!(DocKind::from_path(&p).unwrap().app_name(), kind.app());
        }
        let second = create_office_document(OfficeKind::Word, tmp.path()).unwrap();
        assert_eq!(second.file_name().unwrap(), "Untitled document 2.docx");
    }
}
