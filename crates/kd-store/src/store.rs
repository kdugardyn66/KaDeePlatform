//! Persistence of the start-screen state: settings in `home.json`, the document library
//! (Recent, Starred) in `library.sqlite3` (see [`crate::db`]).
//!
//! Location: the platform config directory for the KaDee Platform
//! (`~/Library/Application Support/be.Kadee.KaDeePlatform` on macOS,
//! `~/.config/kadeeplatform` on Linux, `%APPDATA%\Kadee\KaDeePlatform\config` on Windows).
//! `KADEE_CONFIG_DIR` overrides it.
//!
//! On the first launch, settings, Recent, Starred and the KaDee AI providers (`ai.toml`) are
//! imported from KadeeStudio 0.x (`be.Kadee.KadeeStudio`) when it is installed, so the start
//! screen opens with the same documents and the same AI keys. KadeeStudio's files are only read.

use crate::db::LibraryDb;
use crate::library::Library;
use crate::settings::Settings;
use serde::{Deserialize, Serialize};
use std::fs;
use std::io;
use std::path::{Path, PathBuf};

const FILE_NAME: &str = "home.json";
const WELCOME_NAME: &str = "Welcome to KaDeePlatform.md";
const WELCOME: &str = include_str!("../assets/welcome.md");

/// Everything the start screen persists.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct HomeState {
    pub version: u32,
    pub settings: Settings,
    /// Kept in SQLite; read from `home.json` only when importing from KadeeStudio 0.x.
    #[serde(default, skip_serializing)]
    pub library: Library,
}

impl HomeState {
    pub fn new(documents_dir: PathBuf) -> Self {
        Self { version: 1, settings: Settings::with_documents_dir(documents_dir), library: Library::default() }
    }
}

/// Reads and writes `home.json` in one directory.
#[derive(Debug, Clone)]
pub struct Store {
    dir: PathBuf,
}

impl Store {
    /// The store in the platform config dir (or `KADEE_CONFIG_DIR`).
    pub fn platform() -> Self {
        if let Some(dir) = std::env::var_os("KADEE_CONFIG_DIR") {
            return Self::at(PathBuf::from(dir));
        }
        let dir = directories::ProjectDirs::from("be", "Kadee", "KaDeePlatform")
            .map(|d| d.config_dir().to_path_buf())
            .unwrap_or_else(|| PathBuf::from(".kadeeplatform"));
        Self::at(dir)
    }

    /// KadeeStudio 0.x's settings folder, imported from on the first launch.
    pub fn legacy() -> Option<Self> {
        if std::env::var_os("KADEE_CONFIG_DIR").is_some() {
            return None; // tests and portable setups: no import
        }
        directories::ProjectDirs::from("be", "Kadee", "KadeeStudio").map(|d| Self::at(d.config_dir().to_path_buf()))
    }

    pub fn at(dir: PathBuf) -> Self {
        Self { dir }
    }

    pub fn dir(&self) -> &Path {
        &self.dir
    }

    /// Where the user's own templates live: `Templates` next to `home.json`
    /// (`~/Library/Application Support/be.Kadee.KadeeStudio/Templates` on macOS).
    pub fn templates_dir(&self) -> PathBuf {
        self.dir.join("Templates")
    }

    pub fn file(&self) -> PathBuf {
        self.dir.join(FILE_NAME)
    }

    pub fn library_file(&self) -> PathBuf {
        self.dir.join(crate::db::FILE_NAME)
    }

    /// Default documents folder: `~/Documents/KadeeStudio`.
    pub fn default_documents_dir() -> PathBuf {
        directories::UserDirs::new()
            .and_then(|u| u.document_dir().map(Path::to_path_buf).or_else(|| Some(u.home_dir().join("Documents"))))
            .unwrap_or_else(|| PathBuf::from("."))
            .join("KadeeStudio")
    }

    /// Load the state; on first launch import KadeeStudio 0.x's (see [`Store::legacy`]) or
    /// create it, make the documents folder and drop the welcome document into it.
    /// Returns the state and whether this was a first launch.
    pub fn load_or_init(&self) -> (HomeState, bool) {
        self.load_or_import(Self::legacy().as_ref())
    }

    /// [`Store::load_or_init`] with the store to import from on a first launch.
    pub fn load_or_import(&self, legacy: Option<&Store>) -> (HomeState, bool) {
        let read = |store: &Store| fs::read_to_string(store.file()).ok().and_then(|s| serde_json::from_str::<HomeState>(&s).ok());
        let (mut state, first) = match read(self) {
            Some(state) => (state, false),
            None => {
                let imported = legacy.filter(|l| l.dir != self.dir).and_then(|l| {
                    let state = read(l)?;
                    // KaDee AI: the providers, keys and models chosen in KadeeStudio
                    let ai = l.dir.join("ai.toml");
                    let ours = self.dir.join("ai.toml");
                    if ai.is_file() && !ours.exists() {
                        let _ = fs::create_dir_all(&self.dir);
                        let _ = fs::copy(&ai, &ours);
                    }
                    Some(state)
                });
                let state = imported.unwrap_or_else(|| HomeState::new(Self::default_documents_dir()));
                let _ = seed_documents(&state.settings.documents_dir);
                (state, true)
            }
        };
        // The library lives in SQLite; an empty database takes what home.json had (an import).
        match LibraryDb::open(&self.library_file()) {
            Ok(db) if !db.is_empty().unwrap_or(true) => {
                if let Ok(lib) = db.load() {
                    state.library = lib;
                }
            }
            _ => {}
        }
        if first {
            let _ = self.save(&state);
        }
        (state, first)
    }

    /// Write `home.json` atomically (temp file + rename) and the library to SQLite.
    pub fn save(&self, state: &HomeState) -> io::Result<()> {
        fs::create_dir_all(&self.dir)?;
        let tmp = self.dir.join(format!("{FILE_NAME}.tmp"));
        let json = serde_json::to_string_pretty(state).map_err(io::Error::other)?;
        fs::write(&tmp, json)?;
        fs::rename(tmp, self.file())?;
        let mut db = LibraryDb::open(&self.library_file()).map_err(io::Error::other)?;
        db.save(&state.library).map_err(io::Error::other)
    }
}

/// Create the documents folder with a welcome document if it is new.
pub fn seed_documents(dir: &Path) -> io::Result<()> {
    fs::create_dir_all(dir)?;
    let welcome = dir.join(WELCOME_NAME);
    if !welcome.exists() {
        fs::write(welcome, WELCOME)?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn save_and_load_round_trip() {
        let tmp = tempfile::tempdir().unwrap();
        let store = Store::at(tmp.path().join("cfg"));
        let mut state = HomeState::new(tmp.path().join("docs"));
        state.settings.display_name = "Kris".into();
        state.library.touch(Path::new("/x.md"), 42);
        store.save(&state).unwrap();
        let (loaded, first) = store.load_or_import(None);
        assert!(!first);
        assert_eq!(loaded, state);
    }

    #[test]
    fn corrupt_file_falls_back_to_defaults() {
        let tmp = tempfile::tempdir().unwrap();
        let store = Store::at(tmp.path().to_path_buf());
        fs::write(store.file(), "{ not json").unwrap();
        let (state, first) = store.load_or_import(None);
        assert!(first);
        assert_eq!(state.version, 1);
    }

    #[test]
    fn seeding_writes_welcome_once() {
        let tmp = tempfile::tempdir().unwrap();
        let dir = tmp.path().join("KadeeStudio");
        seed_documents(&dir).unwrap();
        let w = dir.join(WELCOME_NAME);
        fs::write(&w, "edited").unwrap();
        seed_documents(&dir).unwrap();
        assert_eq!(fs::read_to_string(w).unwrap(), "edited");
    }

    #[test]
    fn first_launch_imports_kadeestudio() {
        let tmp = tempfile::tempdir().unwrap();
        let old = Store::at(tmp.path().join("old"));
        let mut legacy = HomeState::new(tmp.path().join("docs"));
        legacy.settings.display_name = "Kris".into();
        legacy.library.touch(Path::new("/x.md"), 7);
        legacy.library.toggle_star(Path::new("/x.md"));
        // KadeeStudio 0.x wrote the library into home.json
        let mut json = serde_json::to_value(&legacy).unwrap();
        json["library"] = serde_json::to_value(&legacy.library).unwrap();
        fs::create_dir_all(old.dir()).unwrap();
        fs::write(old.file(), json.to_string()).unwrap();
        fs::write(old.dir().join("ai.toml"), "# keys").unwrap();

        let new = Store::at(tmp.path().join("new"));
        let (state, first) = new.load_or_import(Some(&old));
        assert!(first);
        assert_eq!(state.settings.display_name, "Kris");
        assert_eq!(state.library, legacy.library);
        assert!(new.dir().join("ai.toml").is_file());
        // the second launch reads its own files
        let (again, first) = new.load_or_import(Some(&old));
        assert!(!first);
        assert_eq!(again.library, legacy.library);
    }

    #[test]
    fn templates_folder_is_next_to_home_json() {
        let store = Store::at(PathBuf::from("/x/be.Kadee.KadeeStudio"));
        assert_eq!(store.templates_dir(), PathBuf::from("/x/be.Kadee.KadeeStudio/Templates"));
    }
}
