//! The document library (Recent and Starred) in SQLite: `library.sqlite3` next to `home.json`.
//!
//! The library is small (at most [`MAX_RECENT`](crate::library::MAX_RECENT) recent entries plus
//! the starred files), so it is read whole at start-up and written whole in one transaction.
//! SQLite gives the platform one place to grow into: version history, search and the offline
//! queue of `kd-sync` will add tables to the same file.

use crate::library::{Library, RecentEntry};
use rusqlite::{Connection, params};
use std::path::{Path, PathBuf};

pub const FILE_NAME: &str = "library.sqlite3";
const SCHEMA_VERSION: i64 = 1;

/// An open library database.
pub struct LibraryDb {
    conn: Connection,
}

impl LibraryDb {
    /// Open (and create or migrate) the database at `path`.
    pub fn open(path: &Path) -> rusqlite::Result<LibraryDb> {
        if let Some(dir) = path.parent() {
            let _ = std::fs::create_dir_all(dir);
        }
        Self::init(Connection::open(path)?)
    }

    /// A database in memory (tests).
    pub fn in_memory() -> rusqlite::Result<LibraryDb> {
        Self::init(Connection::open_in_memory()?)
    }

    fn init(conn: Connection) -> rusqlite::Result<LibraryDb> {
        conn.execute_batch(
            "PRAGMA journal_mode = WAL;
             CREATE TABLE IF NOT EXISTS meta    (key TEXT PRIMARY KEY, value TEXT NOT NULL);
             CREATE TABLE IF NOT EXISTS recent  (path TEXT PRIMARY KEY, opened INTEGER NOT NULL);
             CREATE TABLE IF NOT EXISTS starred (path TEXT PRIMARY KEY, position INTEGER NOT NULL);",
        )?;
        conn.execute("INSERT OR IGNORE INTO meta (key, value) VALUES ('schema', ?1)", params![SCHEMA_VERSION.to_string()])?;
        Ok(LibraryDb { conn })
    }

    /// True when nothing was ever saved (a first launch, or an import is due).
    pub fn is_empty(&self) -> rusqlite::Result<bool> {
        let n: i64 = self.conn.query_row("SELECT (SELECT COUNT(*) FROM recent) + (SELECT COUNT(*) FROM starred)", [], |r| r.get(0))?;
        let saved: i64 = self.conn.query_row("SELECT COUNT(*) FROM meta WHERE key = 'saved'", [], |r| r.get(0))?;
        Ok(n == 0 && saved == 0)
    }

    pub fn load(&self) -> rusqlite::Result<Library> {
        let mut stmt = self.conn.prepare("SELECT path, opened FROM recent ORDER BY opened DESC, rowid DESC")?;
        let recent = stmt
            .query_map([], |r| Ok(RecentEntry { path: PathBuf::from(r.get::<_, String>(0)?), opened: r.get::<_, i64>(1)?.max(0) as u64 }))?
            .collect::<rusqlite::Result<Vec<_>>>()?;
        let mut stmt = self.conn.prepare("SELECT path FROM starred ORDER BY position")?;
        let starred = stmt.query_map([], |r| Ok(PathBuf::from(r.get::<_, String>(0)?)))?.collect::<rusqlite::Result<Vec<_>>>()?;
        Ok(Library { recent, starred })
    }

    /// Replace what is stored with `lib`.
    pub fn save(&mut self, lib: &Library) -> rusqlite::Result<()> {
        let tx = self.conn.transaction()?;
        tx.execute("DELETE FROM recent", [])?;
        tx.execute("DELETE FROM starred", [])?;
        {
            let mut ins = tx.prepare("INSERT OR REPLACE INTO recent (path, opened) VALUES (?1, ?2)")?;
            for e in &lib.recent {
                ins.execute(params![e.path.to_string_lossy(), e.opened as i64])?;
            }
            let mut ins = tx.prepare("INSERT OR REPLACE INTO starred (path, position) VALUES (?1, ?2)")?;
            for (i, p) in lib.starred.iter().enumerate() {
                ins.execute(params![p.to_string_lossy(), i as i64])?;
            }
        }
        tx.execute("INSERT OR REPLACE INTO meta (key, value) VALUES ('saved', '1')", [])?;
        tx.commit()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn round_trip_keeps_order() {
        let mut db = LibraryDb::in_memory().unwrap();
        assert!(db.is_empty().unwrap());
        let mut lib = Library::default();
        lib.touch(Path::new("/a.md"), 10);
        lib.touch(Path::new("/b.md"), 20);
        lib.toggle_star(Path::new("/b.md"));
        lib.toggle_star(Path::new("/a.md"));
        db.save(&lib).unwrap();
        assert!(!db.is_empty().unwrap());
        assert_eq!(db.load().unwrap(), lib);
    }

    #[test]
    fn an_emptied_library_is_not_reimported() {
        let mut db = LibraryDb::in_memory().unwrap();
        db.save(&Library::default()).unwrap();
        assert!(!db.is_empty().unwrap());
    }
}
