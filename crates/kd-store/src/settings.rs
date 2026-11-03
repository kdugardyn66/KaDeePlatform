//! User settings shown on the Settings page.

use serde::{Deserialize, Serialize};
use std::path::PathBuf;

/// Light, dark or follow the system.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum ThemePref {
    #[default]
    System,
    Light,
    Dark,
}

impl ThemePref {
    pub const ALL: [ThemePref; 3] = [ThemePref::System, ThemePref::Light, ThemePref::Dark];

    pub fn label(self) -> &'static str {
        match self {
            ThemePref::System => "System",
            ThemePref::Light => "Light",
            ThemePref::Dark => "Dark",
        }
    }

    /// Next value for the one-click theme switch in the tab strip.
    pub fn cycle(self) -> ThemePref {
        match self {
            ThemePref::System => ThemePref::Light,
            ThemePref::Light => ThemePref::Dark,
            ThemePref::Dark => ThemePref::System,
        }
    }
}

/// Where something opens: in a tab of this window, or in a window of its own.
/// When it is open already, KadeeStudio switches to that tab or window.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum OpenIn {
    #[default]
    Tab,
    /// A separate window (planned: until then, documents open in tabs).
    Window,
}

impl OpenIn {
    pub const ALL: [OpenIn; 2] = [OpenIn::Tab, OpenIn::Window];

    pub fn label(self) -> &'static str {
        match self {
            OpenIn::Tab => "Tab",
            OpenIn::Window => "Window",
        }
    }
}

/// A saved keyboard shortcut: a command id and its keys as text
/// (`"file.new.markdown"`, `"Primary+N M"`). Empty keys = not assigned.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct StoredShortcut {
    pub command: String,
    #[serde(default)]
    pub keys: String,
}

/// Persisted user settings.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Settings {
    /// Name used in the greeting.
    #[serde(default)]
    pub display_name: String,
    #[serde(default)]
    pub theme: ThemePref,
    /// Where new documents are saved.
    pub documents_dir: PathBuf,
    /// Extra folders shown in the sidebar.
    #[serde(default)]
    pub folders: Vec<PathBuf>,
    /// Customised keyboard shortcuts; `None` = the defaults.
    #[serde(default)]
    pub shortcuts: Option<Vec<StoredShortcut>>,
    /// Where a module (Kadee Markdown, Kadee PDF …) opens.
    #[serde(default)]
    pub open_modules_in: OpenIn,
    /// Where each file opens.
    #[serde(default)]
    pub open_files_in: OpenIn,
    /// Folders open in the sidebar's folder tree.
    #[serde(default)]
    pub expanded_folders: Vec<PathBuf>,
}

impl Settings {
    /// Defaults for a first launch.
    pub fn with_documents_dir(documents_dir: PathBuf) -> Self {
        Self {
            display_name: String::new(),
            theme: ThemePref::System,
            documents_dir,
            folders: Vec::new(),
            shortcuts: None,
            open_modules_in: OpenIn::Tab,
            open_files_in: OpenIn::Tab,
            expanded_folders: Vec::new(),
        }
    }
}
