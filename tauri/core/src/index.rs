use crate::{fuzzy, Unit};
use rusqlite::{params, Connection};
use std::collections::HashSet;
use std::fs;
use std::path::{Path, PathBuf};
use std::time::UNIX_EPOCH;

pub struct Index(Connection);

fn database_path() -> Option<PathBuf> {
    let root = std::env::var_os("LOCALAPPDATA")
        .or_else(|| std::env::var_os("XDG_CACHE_HOME"))
        .or_else(|| {
            std::env::var_os("HOME").map(|home| PathBuf::from(home).join(".cache").into_os_string())
        })?;
    Some(
        PathBuf::from(root)
            .join("docs-search")
            .join("search-index.sqlite3"),
    )
}

fn stamp(path: &Path) -> Option<(i64, i64)> {
    let metadata = fs::metadata(path).ok()?;
    let modified = metadata.modified().ok()?.duration_since(UNIX_EPOCH).ok()?;
    Some((
        i64::try_from(metadata.len()).ok()?,
        i64::try_from(modified.as_nanos()).ok()?,
    ))
}

impl Index {
    pub fn open() -> rusqlite::Result<Self> {
        match Self::open_inner() {
            Ok(index) => Ok(index),
            Err(error) => {
                if clear().is_ok() {
                    Self::open_inner()
                } else {
                    Err(error)
                }
            }
        }
    }

    fn open_inner() -> rusqlite::Result<Self> {
        let path = database_path().ok_or(rusqlite::Error::InvalidPath(PathBuf::new()))?;
        fs::create_dir_all(path.parent().unwrap_or(Path::new(".")))
            .map_err(|_| rusqlite::Error::InvalidPath(path.clone()))?;
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let _ = fs::set_permissions(
                path.parent().unwrap_or(Path::new(".")),
                fs::Permissions::from_mode(0o700),
            );
        }
        let connection = Connection::open(path)?;
        let version: i64 = connection.query_row("PRAGMA user_version", [], |row| row.get(0))?;
        if version != 1 {
            connection.execute_batch("DROP TABLE IF EXISTS grams; DROP TABLE IF EXISTS tokens; DROP TABLE IF EXISTS units; DROP TABLE IF EXISTS files;")?;
        }
        connection.execute_batch("\
            CREATE TABLE IF NOT EXISTS files(path TEXT PRIMARY KEY, size INTEGER NOT NULL, modified INTEGER NOT NULL);\
            CREATE TABLE IF NOT EXISTS units(id INTEGER PRIMARY KEY, file_path TEXT NOT NULL, source_kind TEXT NOT NULL, location TEXT NOT NULL, text TEXT NOT NULL, loose TEXT NOT NULL);\
            CREATE INDEX IF NOT EXISTS units_file ON units(file_path);\
            CREATE TABLE IF NOT EXISTS grams(gram TEXT NOT NULL, unit_id INTEGER NOT NULL, PRIMARY KEY(gram, unit_id));\
            CREATE INDEX IF NOT EXISTS grams_unit ON grams(unit_id);\
            CREATE TABLE IF NOT EXISTS tokens(token TEXT NOT NULL, first TEXT NOT NULL, length INTEGER NOT NULL, unit_id INTEGER NOT NULL, PRIMARY KEY(token, unit_id));\
            CREATE INDEX IF NOT EXISTS tokens_lookup ON tokens(first, length);\
            PRAGMA user_version=1;")?;
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            if let Some(path) = database_path() {
                let _ = fs::set_permissions(path, fs::Permissions::from_mode(0o600));
            }
        }
        Ok(Self(connection))
    }

    pub fn current(&self, path: &Path) -> bool {
        let Some((size, modified)) = stamp(path) else {
            return false;
        };
        self.0
            .query_row(
                "SELECT size, modified FROM files WHERE path=?1",
                [path.to_string_lossy().as_ref()],
                |row| Ok((row.get::<_, i64>(0)?, row.get::<_, i64>(1)?)),
            )
            .is_ok_and(|stored| stored == (size, modified))
    }

    pub fn prune_missing(&mut self) -> rusqlite::Result<()> {
        let paths = {
            let mut statement = self.0.prepare("SELECT path FROM files")?;
            let rows = statement.query_map([], |row| row.get::<_, String>(0))?;
            rows.collect::<rusqlite::Result<Vec<_>>>()?
        };
        let missing: Vec<String> = paths
            .into_iter()
            .filter(|path| !Path::new(path).is_file())
            .collect();
        if missing.is_empty() {
            return Ok(());
        }
        let tx = self.0.transaction()?;
        for path in missing {
            tx.execute(
                "DELETE FROM grams WHERE unit_id IN (SELECT id FROM units WHERE file_path=?1)",
                [&path],
            )?;
            tx.execute(
                "DELETE FROM tokens WHERE unit_id IN (SELECT id FROM units WHERE file_path=?1)",
                [&path],
            )?;
            tx.execute("DELETE FROM units WHERE file_path=?1", [&path])?;
            tx.execute("DELETE FROM files WHERE path=?1", [&path])?;
        }
        tx.commit()
    }

    pub fn replace(&mut self, path: &Path, units: &[Unit]) -> rusqlite::Result<()> {
        let Some((size, modified)) = stamp(path) else {
            return Ok(());
        };
        let file_path = path.to_string_lossy();
        let tx = self.0.transaction()?;
        tx.execute(
            "DELETE FROM grams WHERE unit_id IN (SELECT id FROM units WHERE file_path=?1)",
            [file_path.as_ref()],
        )?;
        tx.execute(
            "DELETE FROM tokens WHERE unit_id IN (SELECT id FROM units WHERE file_path=?1)",
            [file_path.as_ref()],
        )?;
        tx.execute("DELETE FROM units WHERE file_path=?1", [file_path.as_ref()])?;
        tx.execute("DELETE FROM files WHERE path=?1", [file_path.as_ref()])?;
        for unit in units {
            let loose = fuzzy::loose_key(&unit.text);
            tx.execute("INSERT INTO units(file_path,source_kind,location,text,loose) VALUES(?1,?2,?3,?4,?5)",
                params![file_path.as_ref(), unit.source_kind, unit.location.to_string(), unit.text, loose])?;
            let id = tx.last_insert_rowid();
            for gram in fuzzy::grams(&loose, 3)
                .into_iter()
                .chain(fuzzy::grams(&loose, 2))
            {
                tx.execute(
                    "INSERT INTO grams(gram,unit_id) VALUES(?1,?2)",
                    params![gram, id],
                )?;
            }
            for token in fuzzy::tokens(&unit.text)
                .into_iter()
                .collect::<HashSet<_>>()
            {
                let first = token.chars().next().unwrap_or_default().to_string();
                tx.execute(
                    "INSERT INTO tokens(token,first,length,unit_id) VALUES(?1,?2,?3,?4)",
                    params![token, first, token.chars().count() as i64, id],
                )?;
            }
        }
        tx.execute(
            "INSERT INTO files(path,size,modified) VALUES(?1,?2,?3)",
            params![file_path.as_ref(), size, modified],
        )?;
        tx.commit()
    }

    pub fn candidates(&self, path: &Path, query: &fuzzy::Query) -> rusqlite::Result<Vec<Unit>> {
        let needle = &query.loose;
        let gram_size = if needle.chars().count() >= 3 { 3 } else { 2 };
        let query_grams = if needle.chars().any(|c| c.is_ascii_digit())
            && needle
                .chars()
                .any(|c| c.is_whitespace() || matches!(c, '_' | '-' | '.' | '/'))
        {
            Vec::new()
        } else {
            fuzzy::grams(needle, gram_size)
        };
        let mut ids = HashSet::new();
        if !query_grams.is_empty() {
            let mut statement = self.0.prepare("SELECT u.id FROM units u JOIN grams g ON g.unit_id=u.id WHERE u.file_path=?1 AND g.gram=?2")?;
            for (number, gram) in query_grams.iter().take(3).enumerate() {
                let rows = statement
                    .query_map(params![path.to_string_lossy().as_ref(), gram], |row| {
                        row.get::<_, i64>(0)
                    })?;
                let group: HashSet<i64> = rows.collect::<rusqlite::Result<HashSet<_>>>()?;
                if number == 0 {
                    ids = group;
                } else {
                    ids.retain(|id| group.contains(id));
                }
                if ids.is_empty() {
                    break;
                }
            }
        } else {
            let mut statement = self.0.prepare("SELECT id FROM units WHERE file_path=?1")?;
            let rows = statement.query_map([path.to_string_lossy().as_ref()], |row| {
                row.get::<_, i64>(0)
            })?;
            for row in rows {
                ids.insert(row?);
            }
        }
        if let Some(typo) = &query.typo {
            let first = typo.chars().next().unwrap_or_default().to_string();
            let len = typo.chars().count() as i64;
            let limit = if len >= 11 { 2 } else { 1 };
            let mut statement = self.0.prepare("SELECT t.unit_id FROM tokens t JOIN units u ON u.id=t.unit_id WHERE u.file_path=?1 AND t.first=?2 AND t.length BETWEEN ?3 AND ?4")?;
            let rows = statement.query_map(
                params![
                    path.to_string_lossy().as_ref(),
                    first,
                    len - limit,
                    len + limit
                ],
                |row| row.get::<_, i64>(0),
            )?;
            for row in rows {
                ids.insert(row?);
            }
        }
        let mut statement = self
            .0
            .prepare("SELECT source_kind, location, text FROM units WHERE id=?1")?;
        let mut result = Vec::with_capacity(ids.len());
        for id in ids {
            let unit = statement.query_row([id], |row| {
                let kind: String = row.get(0)?;
                let location: String = row.get(1)?;
                let text: String = row.get(2)?;
                Ok(Unit {
                    source_kind: source_kind(&kind),
                    location: serde_json::from_str(&location).unwrap_or_default(),
                    text,
                })
            })?;
            result.push(unit);
        }
        Ok(result)
    }
}

fn source_kind(kind: &str) -> &'static str {
    match kind {
        "fileName" => "fileName",
        "cell" => "cell",
        "shape" => "shape",
        "slideTableCell" => "slideTableCell",
        "paragraph" => "paragraph",
        "wordTableParagraph" => "wordTableParagraph",
        _ => "textLine",
    }
}

pub fn clear() -> Result<(), String> {
    let Some(path) = database_path() else {
        return Ok(());
    };
    for candidate in [
        path.clone(),
        path.with_extension("sqlite3-wal"),
        path.with_extension("sqlite3-shm"),
    ] {
        if candidate.exists() {
            fs::remove_file(candidate).map_err(|e| e.to_string())?;
        }
    }
    Ok(())
}
