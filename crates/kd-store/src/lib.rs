//! kd-store: everything the start screen needs that is not UI.
//!
//! The module catalogue, the Crafting Apps catalogue, the recent/starred library
//! (SQLite), user settings (`home.json`) and file helpers. Ported from KadeeStudio's
//! `kadee-core`. This crate never depends on a UI toolkit, so the CLI, the SDKs and
//! the server can reuse it.

pub mod craft;
pub mod db;
pub mod doctype;
pub mod files;
pub mod installed;
pub mod library;
pub mod modules;
pub mod office;
pub mod settings;
pub mod store;

pub use craft::{CRAFT_APPS, CRAFT_PHASE, CraftApp, LAUNCHER_REPO, SOLIDCRAFT_REPO};
pub use db::LibraryDb;
pub use doctype::{DocGroup, DocKind, Opener, all_extensions, open_dialog_types};
pub use files::{
    FileEntry, create_folder, create_new_document, duplicate_file, human_size, list_documents, list_subfolders, now, relative_time,
    rename_file, rename_folder, unique_path,
};
pub use installed::{InstalledApp, Launch, detect_installed};
pub use library::{Library, RecentEntry};
pub use modules::ModuleKind;
pub use office::{OfficeKind, create_office_document};
pub use settings::{OpenIn, Settings, StoredShortcut, ThemePref};
pub use store::{HomeState, Store};

/// The source code on GitHub.
pub const SOURCE_REPO: &str = "https://github.com/kdugardyn66/KaDeePlatform";
/// The KadeeStudio 0.x repository the start screen is ported from.
pub const STUDIO_REPO: &str = "https://github.com/kdugardyn66/KaDeeStudio";
/// Product name shown in the UI.
pub const APP_NAME: &str = "KadeeStudio";
/// Version from Cargo.
pub const VERSION: &str = env!("CARGO_PKG_VERSION");
