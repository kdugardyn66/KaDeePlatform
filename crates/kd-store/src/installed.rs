//! Which Crafting Apps are installed on this computer, and starting them.
//!
//! The places follow the ArtCraft launcher (`storytold/craft-launcher`), so apps
//! are found however they were installed:
//!
//! - **Any system:** the launcher's own record, `installed.json` in its data folder.
//! - **macOS:** `<App>.app` in `/Applications` or `~/Applications`; the version
//!   comes from its `Info.plist`.
//! - **Windows:** `<App>\<app>.exe` under `Program Files` and the per-user
//!   `Programs` folder, in an `ArtCraft` suite folder or on its own.
//! - **Linux:** AppImages in `~/Applications`, `~/AppImages` and `~/.local/bin`,
//!   a `<app>` program on `PATH` (Debian/RPM packages, tarballs), and Flatpaks.

use crate::craft::{CRAFT_APPS, CraftApp};
use std::path::{Path, PathBuf};
use std::process::Command;

/// How to start an installed app.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Launch {
    /// A macOS `.app` bundle (`open -a`).
    MacBundle(PathBuf),
    /// A program file: `.exe`, AppImage or binary.
    Program(PathBuf),
    /// A Flatpak, by its id (`flatpak run`).
    Flatpak(String),
}

/// A Crafting App found on this computer.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct InstalledApp {
    pub app: &'static CraftApp,
    /// `0.7.0`, when the install says.
    pub version: Option<String>,
    pub launch: Launch,
    /// Where it was found, in words: "ArtCraft launcher", "Applications folder" …
    pub source: &'static str,
}

impl InstalledApp {
    /// Where it lives, for a tooltip.
    pub fn location(&self) -> String {
        match &self.launch {
            Launch::MacBundle(p) | Launch::Program(p) => p.display().to_string(),
            Launch::Flatpak(id) => format!("Flatpak {id}"),
        }
    }

    /// Start the app, optionally with a file to open.
    pub fn launch(&self, file: Option<&Path>) -> std::io::Result<()> {
        let mut cmd = match &self.launch {
            Launch::MacBundle(p) => {
                let mut c = Command::new("open");
                c.arg("-a").arg(p);
                c
            }
            Launch::Program(p) => Command::new(p),
            Launch::Flatpak(id) => {
                let mut c = Command::new("flatpak");
                c.arg("run").arg(id);
                c
            }
        };
        if let Some(f) = file {
            cmd.arg(f);
        }
        cmd.spawn().map(|_| ())
    }
}

/// Every name an app's files and folders may use: `PdfCraft`, `PrintCraft`.
fn names(app: &CraftApp) -> impl Iterator<Item = &'static str> {
    std::iter::once(app.name).chain(app.aliases.iter().copied())
}

/// Every machine name its programs may use: `pdfcraft`, `printcraft`.
fn stems(app: &CraftApp) -> Vec<String> {
    let mut v = vec![app.id.to_string()];
    for a in app.aliases {
        let s = a.to_ascii_lowercase();
        if !v.contains(&s) {
            v.push(s);
        }
    }
    v
}

/// The folders searched on this computer.
#[derive(Debug, Clone, Default)]
pub struct SearchRoots {
    pub home: Option<PathBuf>,
    /// Per-user data folder (`~/Library/Application Support`, `%LOCALAPPDATA%`, `~/.local/share`).
    pub data_local: Option<PathBuf>,
    /// Folders on `PATH`.
    pub path: Vec<PathBuf>,
    /// Windows: `Program Files`, `Program Files (x86)`, `%LOCALAPPDATA%\Programs`.
    pub program_files: Vec<PathBuf>,
}

impl SearchRoots {
    pub fn system() -> Self {
        let base = directories::BaseDirs::new();
        let home = base.as_ref().map(|b| b.home_dir().to_path_buf());
        let data_local = base.as_ref().map(|b| b.data_local_dir().to_path_buf());
        let path = std::env::var_os("PATH").map(|p| std::env::split_paths(&p).collect()).unwrap_or_default();
        let mut program_files: Vec<PathBuf> =
            ["ProgramFiles", "ProgramW6432", "ProgramFiles(x86)"].iter().filter_map(std::env::var_os).map(PathBuf::from).collect();
        if let Some(l) = std::env::var_os("LOCALAPPDATA") {
            program_files.push(PathBuf::from(l).join("Programs"));
        }
        program_files.dedup();
        Self { home, data_local, path, program_files }
    }
}

/// The Crafting Apps installed on this computer, in catalogue order.
pub fn detect_installed() -> Vec<InstalledApp> {
    detect_with(&SearchRoots::system())
}

/// [`detect_installed`] with given folders (tests).
pub fn detect_with(roots: &SearchRoots) -> Vec<InstalledApp> {
    let manifest = roots.data_local.as_ref().map(|d| d.join(LAUNCHER_DIR).join("installed.json")).and_then(|p| read_manifest(&p));
    CRAFT_APPS
        .iter()
        .filter_map(|app| {
            if let Some(found) = manifest.as_ref().and_then(|m| from_manifest(app, m)) {
                return Some(found);
            }
            if cfg!(target_os = "macos") {
                let mut dirs = vec![PathBuf::from("/Applications")];
                dirs.extend(roots.home.iter().map(|h| h.join("Applications")));
                return mac_bundle(app, &dirs);
            }
            if cfg!(windows) {
                return windows_exe(app, &roots.program_files);
            }
            linux(app, roots)
        })
        .collect()
}

/// The launcher's data folder name.
const LAUNCHER_DIR: &str = if cfg!(any(target_os = "macos", target_os = "windows")) { "ArtCraft Launcher" } else { "artcraft-launcher" };

fn read_manifest(path: &Path) -> Option<serde_json::Value> {
    serde_json::from_slice(&std::fs::read(path).ok()?).ok()
}

/// An app the ArtCraft launcher installed (`installed.json` → `apps.<id>`).
pub fn from_manifest(app: &'static CraftApp, manifest: &serde_json::Value) -> Option<InstalledApp> {
    let apps = manifest.get("apps")?;
    let rec = stems(app).iter().find_map(|s| apps.get(s))?;
    let path = PathBuf::from(rec.get("path")?.as_str()?);
    if !path.exists() {
        return None;
    }
    let version = rec.get("version").and_then(|v| v.as_str()).map(str::to_string);
    let launch = if path.extension().is_some_and(|e| e == "app") {
        Launch::MacBundle(path)
    } else if path.is_dir() {
        // a folder install (MSI, portable, tarball): the program inside it
        let exe = stems(app)
            .iter()
            .flat_map(|s| [path.join(format!("{s}.exe")), path.join(s), path.join("bin").join(s)])
            .find(|p| p.is_file())?;
        Launch::Program(exe)
    } else {
        Launch::Program(path)
    };
    Some(InstalledApp { app, version, launch, source: "ArtCraft launcher" })
}

/// macOS: `<App>.app` in one of `dirs`, checked against its bundle id.
pub fn mac_bundle(app: &'static CraftApp, dirs: &[PathBuf]) -> Option<InstalledApp> {
    for dir in dirs {
        for name in names(app) {
            let bundle = dir.join(format!("{name}.app"));
            let Ok(plist) = plist::Value::from_file(bundle.join("Contents/Info.plist")) else { continue };
            let Some(dict) = plist.as_dictionary() else { continue };
            let id = dict.get("CFBundleIdentifier").and_then(plist::Value::as_string).unwrap_or_default();
            let ours = id.eq_ignore_ascii_case(app.bundle_id)
                || stems(app).iter().any(|s| id.eq_ignore_ascii_case(&format!("ai.storyteller.{s}")));
            if !ours {
                continue;
            }
            let version = ["CFBundleShortVersionString", "CFBundleVersion"]
                .iter()
                .find_map(|k| dict.get(k).and_then(plist::Value::as_string))
                .map(str::to_string);
            let source = if dir.starts_with("/Applications") { "Applications folder" } else { "your Applications folder" };
            return Some(InstalledApp { app, version, launch: Launch::MacBundle(bundle), source });
        }
    }
    None
}

/// Windows: `<root>\ArtCraft\<App>\<app>.exe` or `<root>\<App>\<app>.exe`.
pub fn windows_exe(app: &'static CraftApp, roots: &[PathBuf]) -> Option<InstalledApp> {
    for root in roots {
        for name in names(app) {
            for dir in [root.join("ArtCraft").join(name), root.join(name)] {
                for stem in stems(app) {
                    let exe = dir.join(format!("{stem}.exe"));
                    if exe.is_file() {
                        return Some(InstalledApp { app, version: None, launch: Launch::Program(exe), source: "Programs folder" });
                    }
                }
            }
        }
    }
    None
}

/// Linux: an AppImage, a program on `PATH`, or a Flatpak.
pub fn linux(app: &'static CraftApp, roots: &SearchRoots) -> Option<InstalledApp> {
    let stems = stems(app);
    if let Some(home) = &roots.home {
        for dir in [home.join("Applications"), home.join("AppImages"), home.join(".local").join("bin")] {
            let Ok(entries) = std::fs::read_dir(&dir) else { continue };
            let mut found: Vec<(PathBuf, Option<String>)> = entries
                .flatten()
                .filter_map(|e| appimage_version(&e.file_name().to_string_lossy(), &stems).map(|v| (e.path(), v)))
                .collect();
            found.sort();
            if let Some((path, version)) = found.pop() {
                return Some(InstalledApp { app, version, launch: Launch::Program(path), source: "AppImage" });
            }
        }
    }
    let mut bins = roots.path.clone();
    bins.extend(["/usr/bin", "/usr/local/bin", "/opt/bin"].map(PathBuf::from));
    for dir in &bins {
        for s in &stems {
            let p = dir.join(s);
            if p.is_file() {
                return Some(InstalledApp { app, version: None, launch: Launch::Program(p), source: "installed package" });
            }
        }
    }
    let mut flatpak = vec![PathBuf::from("/var/lib/flatpak/app")];
    flatpak.extend(roots.data_local.iter().map(|d| d.join("flatpak").join("app")));
    for dir in flatpak {
        if dir.join(app.bundle_id).join("current").exists() {
            return Some(InstalledApp { app, version: None, launch: Launch::Flatpak(app.bundle_id.to_string()), source: "Flatpak" });
        }
    }
    None
}

/// Whether a file name is an AppImage of the app, and the version in its name:
/// `photocraft-0.7.0-linux-x86_64.AppImage` → `Some(Some("0.7.0"))`,
/// `photocraft.AppImage` → `Some(None)`.
pub fn appimage_version(name: &str, stems: &[String]) -> Option<Option<String>> {
    let split = name.len().checked_sub(".appimage".len())?;
    let (base, ext) = (name.get(..split)?, name.get(split..)?);
    if !ext.eq_ignore_ascii_case(".appimage") {
        return None;
    }
    for s in stems {
        if base.eq_ignore_ascii_case(s) {
            return Some(None);
        }
        if let Some(rest) = base.get(..s.len()).filter(|h| h.eq_ignore_ascii_case(s)).and_then(|_| base.get(s.len()..))
            && let Some(rest) = rest.strip_prefix('-')
        {
            let v = rest.split('-').next().unwrap_or_default().trim_start_matches('v');
            if v.starts_with(|c: char| c.is_ascii_digit()) {
                return Some(Some(v.to_string()));
            }
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    fn app(name: &str) -> &'static CraftApp {
        CRAFT_APPS.iter().find(|a| a.name.eq_ignore_ascii_case(name)).unwrap()
    }

    #[test]
    fn appimage_names() {
        let s = vec!["photocraft".to_string()];
        assert_eq!(appimage_version("photocraft-0.7.0-linux-x86_64.AppImage", &s), Some(Some("0.7.0".into())));
        assert_eq!(appimage_version("PhotoCraft.appimage", &s), Some(None));
        assert_eq!(appimage_version("photocraft-notes.AppImage", &s), None);
        assert_eq!(appimage_version("photocraftx-1.0.AppImage", &s), None);
        assert_eq!(appimage_version("photocraft-0.7.0.tar.gz", &s), None);
    }

    #[test]
    fn finds_linux_installs() {
        let t = tempfile::tempdir().unwrap();
        let home = t.path().join("home");
        fs::create_dir_all(home.join("Applications")).unwrap();
        fs::write(home.join("Applications/photocraft-0.6.0-linux-x86_64.AppImage"), b"").unwrap();
        fs::write(home.join("Applications/photocraft-0.7.0-linux-x86_64.AppImage"), b"").unwrap();
        let bin = t.path().join("bin");
        fs::create_dir_all(&bin).unwrap();
        fs::write(bin.join("printcraft"), b"").unwrap(); // PdfCraft's former name
        let data = t.path().join("data");
        fs::create_dir_all(data.join("flatpak/app/io.tangencylabs.solidcraft/current")).unwrap();
        let roots = SearchRoots { home: Some(home), data_local: Some(data), path: vec![bin.clone()], program_files: vec![] };

        let photo = linux(app("PhotoCraft"), &roots).unwrap();
        assert_eq!(photo.version.as_deref(), Some("0.7.0"));
        assert_eq!(photo.source, "AppImage");
        assert_eq!(linux(app("PdfCraft"), &roots).unwrap().launch, Launch::Program(bin.join("printcraft")));
        assert_eq!(linux(app("SolidCraft"), &roots).unwrap().launch, Launch::Flatpak("io.tangencylabs.solidcraft".into()));
        assert!(linux(app("WordCraft"), &roots).is_none());
    }

    #[test]
    fn reads_mac_bundles_and_checks_the_bundle_id() {
        let t = tempfile::tempdir().unwrap();
        let plist = |id: &str, v: &str| {
            format!(
                r#"<?xml version="1.0" encoding="UTF-8"?><plist version="1.0"><dict>
                <key>CFBundleIdentifier</key><string>{id}</string>
                <key>CFBundleShortVersionString</key><string>{v}</string></dict></plist>"#
            )
        };
        let c = t.path().join("CADCraft.app/Contents");
        fs::create_dir_all(&c).unwrap();
        fs::write(c.join("Info.plist"), plist("ai.storyteller.cadcraft", "0.3.1")).unwrap();
        let other = t.path().join("GridCraft.app/Contents");
        fs::create_dir_all(&other).unwrap();
        fs::write(other.join("Info.plist"), plist("com.example.notgridcraft", "9.9")).unwrap();

        let dirs = vec![t.path().to_path_buf()];
        let cad = mac_bundle(app("CADCraft"), &dirs).unwrap();
        assert_eq!(cad.version.as_deref(), Some("0.3.1"));
        // macOS file systems ignore case, so only the name's letters matter, not their case.
        let Launch::MacBundle(found) = &cad.launch else { panic!("not a bundle") };
        assert!(found.to_string_lossy().eq_ignore_ascii_case(&t.path().join("CADCraft.app").to_string_lossy()));
        assert!(mac_bundle(app("GridCraft"), &dirs).is_none(), "someone else's bundle");
    }

    #[test]
    fn finds_windows_exes_and_launcher_records() {
        let t = tempfile::tempdir().unwrap();
        let pf = t.path().join("Program Files");
        fs::create_dir_all(pf.join("ArtCraft/SoundCraft")).unwrap();
        fs::write(pf.join("ArtCraft/SoundCraft/soundcraft.exe"), b"").unwrap();
        let found = windows_exe(app("SoundCraft"), std::slice::from_ref(&pf)).unwrap();
        assert_eq!(found.launch, Launch::Program(pf.join("ArtCraft/SoundCraft/soundcraft.exe")));

        let img = t.path().join("filmcraft.AppImage");
        fs::write(&img, b"").unwrap();
        let manifest = serde_json::json!({ "apps": { "filmcraft": { "version": "0.4.2", "kind": "AppImage", "path": img } } });
        let film = from_manifest(app("FilmCraft"), &manifest).unwrap();
        assert_eq!((film.version.as_deref(), film.source), (Some("0.4.2"), "ArtCraft launcher"));
        let gone = serde_json::json!({ "apps": { "filmcraft": { "version": "0.4.2", "path": "/no/such/file" } } });
        assert!(from_manifest(app("FilmCraft"), &gone).is_none());
    }
}
