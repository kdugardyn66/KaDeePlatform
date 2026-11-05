//! Document listing, creation and display helpers.

use crate::doctype::DocKind;
use crate::modules::ModuleKind;
use std::fs;
use std::io;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

/// A document shown in the start-screen file list.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FileEntry {
    pub path: PathBuf,
    pub name: String,
    pub kind: DocKind,
    pub size: u64,
    /// Unix seconds; 0 when unknown.
    pub modified: u64,
    /// False when the file no longer exists (kept in Recent).
    pub exists: bool,
}

impl FileEntry {
    /// Read metadata for `path`; `None` if no module owns the file type.
    pub fn from_path(path: &Path) -> Option<FileEntry> {
        let kind = DocKind::from_path(path)?;
        let name = path.file_name()?.to_string_lossy().into_owned();
        let meta = fs::metadata(path).ok();
        let modified = meta.as_ref().and_then(|m| m.modified().ok()).map(unix).unwrap_or(0);
        Some(FileEntry {
            path: path.to_path_buf(),
            name,
            kind,
            size: meta.as_ref().map(|m| m.len()).unwrap_or(0),
            modified,
            exists: meta.is_some(),
        })
    }
}

/// Unix seconds now.
pub fn now() -> u64 {
    unix(SystemTime::now())
}

fn unix(t: SystemTime) -> u64 {
    t.duration_since(UNIX_EPOCH).map(|d| d.as_secs()).unwrap_or(0)
}

/// KadeeStudio documents directly inside `dir` (not recursive).
pub fn list_documents(dir: &Path) -> Vec<FileEntry> {
    let Ok(rd) = fs::read_dir(dir) else { return Vec::new() };
    let mut out: Vec<FileEntry> = rd
        .filter_map(Result::ok)
        .map(|e| e.path())
        .filter(|p| p.is_file())
        .filter(|p| !p.file_name().is_some_and(|n| n.to_string_lossy().starts_with('.')))
        .filter_map(|p| FileEntry::from_path(&p))
        .collect();
    out.sort_by_key(|e| std::cmp::Reverse(e.modified));
    out
}

/// The visible folders directly inside `dir`, by name (hidden folders and bundles such as
/// `.app` packages are left out).
pub fn list_subfolders(dir: &Path) -> Vec<PathBuf> {
    let Ok(rd) = fs::read_dir(dir) else { return Vec::new() };
    let mut out: Vec<PathBuf> = rd
        .filter_map(Result::ok)
        .map(|e| e.path())
        .filter(|p| p.is_dir())
        .filter(|p| {
            p.file_name().is_some_and(|n| {
                let n = n.to_string_lossy();
                !n.starts_with('.') && !n.ends_with(".app") && !n.ends_with(".bundle")
            })
        })
        .collect();
    out.sort_by_key(|p| p.file_name().map(|n| n.to_string_lossy().to_lowercase()).unwrap_or_default());
    out
}

/// Create the folder `parent/name` and return its path; fails when it exists or the name is
/// not a plain folder name.
pub fn create_folder(parent: &Path, name: &str) -> io::Result<PathBuf> {
    let name = valid_name(name)?;
    let path = parent.join(name);
    fs::create_dir(&path)?;
    Ok(path)
}

/// Rename the folder `dir` to `name` in the same parent and return the new path.
pub fn rename_folder(dir: &Path, name: &str) -> io::Result<PathBuf> {
    let name = valid_name(name)?;
    let parent = dir.parent().ok_or_else(|| io::Error::new(io::ErrorKind::InvalidInput, "no parent folder"))?;
    let to = parent.join(name);
    if to == dir {
        return Ok(to);
    }
    if to.exists() {
        return Err(io::Error::new(io::ErrorKind::AlreadyExists, format!("“{name}” already exists")));
    }
    fs::rename(dir, &to)?;
    Ok(to)
}

/// Rename the file `path` to `stem` in the same folder, keeping its extension; returns the new path.
pub fn rename_file(path: &Path, stem: &str) -> io::Result<PathBuf> {
    let stem = valid_name(stem)?;
    let parent = path.parent().ok_or_else(|| io::Error::new(io::ErrorKind::InvalidInput, "no parent folder"))?;
    let name = match path.extension() {
        Some(ext) => format!("{stem}.{}", ext.to_string_lossy()),
        None => stem.to_string(),
    };
    let to = parent.join(&name);
    if to == path {
        return Ok(to);
    }
    if to.exists() {
        return Err(io::Error::new(io::ErrorKind::AlreadyExists, format!("“{name}” already exists")));
    }
    fs::rename(path, &to)?;
    Ok(to)
}

/// Copy the file `path` next to itself as `<name> copy.<ext>` (numbered if taken); returns the copy.
pub fn duplicate_file(path: &Path) -> io::Result<PathBuf> {
    let parent = path.parent().ok_or_else(|| io::Error::new(io::ErrorKind::InvalidInput, "no parent folder"))?;
    let stem = path.file_stem().map(|s| s.to_string_lossy().into_owned()).unwrap_or_default();
    let ext = path.extension().map(|e| e.to_string_lossy().into_owned()).unwrap_or_default();
    let to = unique_path(parent, &format!("{stem} copy"), &ext);
    fs::copy(path, &to)?;
    Ok(to)
}

fn valid_name(name: &str) -> io::Result<&str> {
    let name = name.trim();
    if name.is_empty() || name == "." || name == ".." || name.contains(['/', '\\', ':']) {
        return Err(io::Error::new(io::ErrorKind::InvalidInput, "Use a name without / \\ or :"));
    }
    Ok(name)
}

/// Create `Untitled <kind>.<ext>` (numbered if taken) in `dir` and return its path.
pub fn create_new_document(kind: ModuleKind, dir: &Path) -> io::Result<PathBuf> {
    let ext = kind
        .new_extension()
        .ok_or_else(|| io::Error::new(io::ErrorKind::Unsupported, format!("{} does not create documents", kind.name())))?;
    fs::create_dir_all(dir)?;
    let base = format!("Untitled {}", kind.short().to_lowercase());
    let path = unique_path(dir, &base, ext);
    let title = path.file_stem().map(|s| s.to_string_lossy().into_owned()).unwrap_or(base);
    fs::write(&path, kind.starter_content(&title))?;
    Ok(path)
}

/// A valid one-page blank A4 PDF with `title` in its document info.
pub fn blank_pdf(title: &str) -> String {
    let esc: String = title.chars().flat_map(|c| if matches!(c, '(' | ')' | '\\') { vec!['\\', c] } else { vec![c] }).collect();
    let esc: String = esc.chars().filter(|c| c.is_ascii() && !c.is_ascii_control()).collect();
    let objects = [
        "<< /Type /Catalog /Pages 2 0 R >>".to_string(),
        "<< /Type /Pages /Kids [3 0 R] /Count 1 >>".to_string(),
        "<< /Type /Page /Parent 2 0 R /MediaBox [0 0 595 842] /Resources << >> >>".to_string(),
        format!("<< /Title ({esc}) /Producer (KaDeePlatform) >>"),
    ];
    let mut out = String::from("%PDF-1.4\n");
    let mut offsets = Vec::new();
    for (i, body) in objects.iter().enumerate() {
        offsets.push(out.len());
        out.push_str(&format!("{} 0 obj\n{body}\nendobj\n", i + 1));
    }
    let xref = out.len();
    out.push_str(&format!("xref\n0 {}\n0000000000 65535 f \n", objects.len() + 1));
    for o in offsets {
        out.push_str(&format!("{o:010} 00000 n \n"));
    }
    out.push_str(&format!("trailer\n<< /Size {} /Root 1 0 R /Info 4 0 R >>\nstartxref\n{xref}\n%%EOF\n", objects.len() + 1));
    out
}

/// `dir/base.ext`, or `dir/base 2.ext`, `dir/base 3.ext`, … if taken.
pub fn unique_path(dir: &Path, base: &str, ext: &str) -> PathBuf {
    let first = dir.join(format!("{base}.{ext}"));
    if !first.exists() {
        return first;
    }
    (2..).map(|n| dir.join(format!("{base} {n}.{ext}"))).find(|p| !p.exists()).expect("an unused name exists")
}

/// `1.2 KB`, `3.4 MB`.
pub fn human_size(bytes: u64) -> String {
    const UNITS: [&str; 5] = ["B", "KB", "MB", "GB", "TB"];
    if bytes < 1024 {
        return format!("{bytes} B");
    }
    let mut v = bytes as f64;
    let mut i = 0;
    while v >= 1024.0 && i < UNITS.len() - 1 {
        v /= 1024.0;
        i += 1;
    }
    format!("{v:.1} {}", UNITS[i])
}

/// `just now`, `5 min ago`, `3 h ago`, `yesterday`, `4 days ago`, or a day count.
pub fn relative_time(then: u64, now: u64) -> String {
    if then == 0 {
        return "—".into();
    }
    let d = now.saturating_sub(then);
    match d {
        0..=59 => "just now".into(),
        60..=3_599 => format!("{} min ago", d / 60),
        3_600..=86_399 => format!("{} h ago", d / 3_600),
        86_400..=172_799 => "yesterday".into(),
        _ if d < 30 * 86_400 => format!("{} days ago", d / 86_400),
        _ if d < 365 * 86_400 => format!("{} months ago", d / (30 * 86_400)),
        _ => format!("{} years ago", d / (365 * 86_400)),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn subfolders_create_and_rename() {
        let tmp = tempfile::tempdir().unwrap();
        let a = create_folder(tmp.path(), "Clients").unwrap();
        create_folder(tmp.path(), "archive").unwrap();
        fs::create_dir(tmp.path().join(".hidden")).unwrap();
        fs::write(tmp.path().join("note.md"), "x").unwrap();
        let names: Vec<String> =
            list_subfolders(tmp.path()).iter().map(|p| p.file_name().unwrap().to_string_lossy().into_owned()).collect();
        assert_eq!(names, vec!["archive", "Clients"]);
        assert!(create_folder(tmp.path(), "Clients").is_err());
        assert!(create_folder(tmp.path(), "a/b").is_err());
        let b = rename_folder(&a, "Customers").unwrap();
        assert!(b.is_dir() && !a.exists());
        assert!(rename_folder(&b, "archive").is_err());
    }

    #[test]
    fn rename_and_duplicate_files() {
        let tmp = tempfile::tempdir().unwrap();
        let a = tmp.path().join("Notes.md");
        fs::write(&a, "hello").unwrap();
        let b = rename_file(&a, "Minutes").unwrap();
        assert_eq!(b, tmp.path().join("Minutes.md"));
        let c = duplicate_file(&b).unwrap();
        assert_eq!(c, tmp.path().join("Minutes copy.md"));
        assert_eq!(fs::read_to_string(&c).unwrap(), "hello");
        assert_eq!(duplicate_file(&b).unwrap(), tmp.path().join("Minutes copy 2.md"));
        assert!(rename_file(&c, "Minutes").is_err());
    }

    #[test]
    fn creates_numbered_documents() {
        let tmp = tempfile::tempdir().unwrap();
        let a = create_new_document(ModuleKind::Markdown, tmp.path()).unwrap();
        let b = create_new_document(ModuleKind::Markdown, tmp.path()).unwrap();
        assert_eq!(a.file_name().unwrap(), "Untitled markdown.md");
        assert_eq!(b.file_name().unwrap(), "Untitled markdown 2.md");
        assert_eq!(fs::read_to_string(&b).unwrap(), "# Untitled markdown 2\n\n");
        assert!(create_new_document(ModuleKind::Convert, tmp.path()).is_err());
        let pdf = create_new_document(ModuleKind::Pdf, tmp.path()).unwrap();
        assert_eq!(pdf.file_name().unwrap(), "Untitled pdf.pdf");
    }

    #[test]
    fn blank_pdf_has_a_correct_xref() {
        let pdf = blank_pdf("Q4 (draft)");
        assert!(pdf.starts_with("%PDF-1.4") && pdf.ends_with("%%EOF\n"));
        assert!(pdf.contains("/Title (Q4 \\(draft\\))"));
        let start: usize = pdf.lines().rev().nth(1).unwrap().parse().unwrap();
        assert!(pdf[start..].starts_with("xref"));
        // every xref entry points at its object header
        for (i, line) in pdf[start..].lines().skip(3).take(4).enumerate() {
            let off: usize = line[..10].parse().unwrap();
            assert!(pdf[off..].starts_with(&format!("{} 0 obj", i + 1)), "object {}", i + 1);
        }
    }

    #[test]
    fn lists_only_known_documents() {
        let tmp = tempfile::tempdir().unwrap();
        fs::write(tmp.path().join("a.md"), "x").unwrap();
        fs::write(tmp.path().join("b.txt"), "x").unwrap();
        fs::write(tmp.path().join("part.step"), "x").unwrap();
        fs::write(tmp.path().join(".hidden.md"), "x").unwrap();
        fs::write(tmp.path().join("c.kdash"), "{}").unwrap();
        let names: Vec<_> = list_documents(tmp.path()).into_iter().map(|e| e.name).collect();
        assert_eq!(names.len(), 3);
        assert!(names.contains(&"part.step".to_string()));
        assert!(names.contains(&"a.md".to_string()));
        assert!(names.contains(&"c.kdash".to_string()));
    }

    #[test]
    fn formats_sizes_and_times() {
        assert_eq!(human_size(512), "512 B");
        assert_eq!(human_size(1536), "1.5 KB");
        assert_eq!(human_size(5 * 1024 * 1024), "5.0 MB");
        assert_eq!(relative_time(100, 130), "just now");
        assert_eq!(relative_time(0, 130), "—");
        assert_eq!(relative_time(1000, 1000 + 7200), "2 h ago");
        assert_eq!(relative_time(1000, 1000 + 90_000), "yesterday");
    }
}
