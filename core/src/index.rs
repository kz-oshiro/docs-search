use crate::{
    extract::{ExtractedDocument, SheetMeta, UnitMeta},
    fuzzy, Unit,
};
use rusqlite::{params, Connection};
use std::collections::HashSet;
use std::fs;
use std::path::{Path, PathBuf};
use std::time::UNIX_EPOCH;

pub struct Index(Connection);
const SCHEMA_VERSION: i64 = 2;
pub const EXTRACTION_VERSION: i64 = 4;
pub const DEFAULT_SCOPE: i64 = 0;

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

pub(crate) fn stamp(path: &Path) -> Option<(i64, i64)> {
    let metadata = fs::metadata(path).ok()?;
    let modified = metadata.modified().ok()?.duration_since(UNIX_EPOCH).ok()?;
    Some((
        i64::try_from(metadata.len()).ok()?,
        i64::try_from(modified.as_nanos()).ok()?,
    ))
}

fn initialize(connection: &Connection) -> rusqlite::Result<()> {
    let version: i64 = connection.query_row("PRAGMA user_version", [], |row| row.get(0))?;
    if version != SCHEMA_VERSION {
        connection.execute_batch("DROP TABLE IF EXISTS grams; DROP TABLE IF EXISTS tokens; DROP TABLE IF EXISTS units; DROP TABLE IF EXISTS files;")?;
    }
    connection.execute_batch("\
        CREATE TABLE IF NOT EXISTS files(path TEXT PRIMARY KEY, size INTEGER NOT NULL, modified INTEGER NOT NULL, extraction_version INTEGER NOT NULL, extraction_scope INTEGER NOT NULL, sheets TEXT NOT NULL);\
        CREATE TABLE IF NOT EXISTS units(id INTEGER PRIMARY KEY, file_path TEXT NOT NULL, source_kind TEXT NOT NULL, location TEXT NOT NULL, text TEXT NOT NULL, loose TEXT NOT NULL, unit_key TEXT NOT NULL, part_key TEXT NOT NULL, group_key TEXT NOT NULL, row_number INTEGER, column_number INTEGER, content_class TEXT NOT NULL, anchor TEXT);\
        CREATE INDEX IF NOT EXISTS units_file ON units(file_path);\
        CREATE UNIQUE INDEX IF NOT EXISTS units_key ON units(file_path, unit_key);\
        CREATE INDEX IF NOT EXISTS units_group ON units(file_path, group_key);\
        CREATE INDEX IF NOT EXISTS units_context ON units(file_path, part_key, row_number, column_number);\
        CREATE TABLE IF NOT EXISTS grams(gram TEXT NOT NULL, unit_id INTEGER NOT NULL, PRIMARY KEY(gram, unit_id));\
        CREATE INDEX IF NOT EXISTS grams_unit ON grams(unit_id);\
        CREATE TABLE IF NOT EXISTS tokens(token TEXT NOT NULL, first TEXT NOT NULL, length INTEGER NOT NULL, unit_id INTEGER NOT NULL, PRIMARY KEY(token, unit_id));\
        CREATE INDEX IF NOT EXISTS tokens_lookup ON tokens(first, length);\
        PRAGMA user_version=2;")?;
    // Retire every old extraction record when indexing is next enabled, including
    // files outside today's roots. No old phonetic text remains queryable.
    let transaction = connection.unchecked_transaction()?;
    let outdated = "SELECT path FROM files WHERE extraction_version<>?1";
    transaction.execute(&format!("DELETE FROM grams WHERE unit_id IN (SELECT id FROM units WHERE file_path IN ({outdated}))"),[EXTRACTION_VERSION])?;
    transaction.execute(&format!("DELETE FROM tokens WHERE unit_id IN (SELECT id FROM units WHERE file_path IN ({outdated}))"),[EXTRACTION_VERSION])?;
    transaction.execute(
        &format!("DELETE FROM units WHERE file_path IN ({outdated})"),
        [EXTRACTION_VERSION],
    )?;
    transaction.execute(
        "DELETE FROM files WHERE extraction_version<>?1",
        [EXTRACTION_VERSION],
    )?;
    transaction.commit()
}

impl Index {
    #[cfg(test)]
    pub(crate) fn in_memory() -> rusqlite::Result<Self> {
        let connection = Connection::open_in_memory()?;
        initialize(&connection)?;
        Ok(Self(connection))
    }

    /// Composite conditions need every unit in a candidate scope, including NOT evidence.
    /// The initial implementation reads the complete indexed file to avoid false negatives.
    pub fn all_units(&self, path: &Path) -> rusqlite::Result<Vec<Unit>> {
        self.candidates(path, &fuzzy::Query::new(""), false)
    }

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
        initialize(&connection)?;
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            if let Some(path) = database_path() {
                let _ = fs::set_permissions(path, fs::Permissions::from_mode(0o600));
            }
        }
        Ok(Self(connection))
    }

    pub fn current(&self, path: &Path, scope: i64) -> bool {
        let Some((size, modified)) = stamp(path) else {
            return false;
        };
        self.0
            .query_row(
                "SELECT size, modified, extraction_version, extraction_scope FROM files WHERE path=?1",
                [path.to_string_lossy().as_ref()],
                |row| Ok((row.get::<_, i64>(0)?, row.get::<_, i64>(1)?, row.get::<_, i64>(2)?, row.get::<_, i64>(3)?)),
            )
            .is_ok_and(|stored| stored == (size, modified, EXTRACTION_VERSION, scope))
            && self.sheet_metadata(path).is_ok()
    }

    pub fn sheet_metadata(&self, path: &Path) -> rusqlite::Result<Vec<SheetMeta>> {
        let data: String = self.0.query_row(
            "SELECT sheets FROM files WHERE path=?1",
            [path.to_string_lossy().as_ref()],
            |row| row.get(0),
        )?;
        serde_json::from_str(&data).map_err(|_| rusqlite::Error::InvalidQuery)
    }

    pub(crate) fn cells_in_range(
        &self,
        path: &Path,
        part: &str,
        rows: (u32, u32),
        columns: (u32, u32),
    ) -> rusqlite::Result<Vec<(u32, u32, String)>> {
        let mut statement = self.0.prepare(
            "SELECT row_number, column_number, text FROM units \
             WHERE file_path=?1 AND part_key=?2 AND source_kind='cell' \
             AND row_number BETWEEN ?3 AND ?4 AND column_number BETWEEN ?5 AND ?6 \
             ORDER BY id",
        )?;
        let rows = statement.query_map(
            params![
                path.to_string_lossy().as_ref(),
                part,
                rows.0,
                rows.1,
                columns.0,
                columns.1
            ],
            |row| {
                let row_number: i64 = row.get(0)?;
                let column_number: i64 = row.get(1)?;
                Ok((
                    u32::try_from(row_number).map_err(|_| rusqlite::Error::InvalidQuery)?,
                    u32::try_from(column_number).map_err(|_| rusqlite::Error::InvalidQuery)?,
                    row.get(2)?,
                ))
            },
        )?;
        rows.collect()
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

    pub fn replace(
        &mut self,
        path: &Path,
        document: &ExtractedDocument,
        expected_stamp: (i64, i64),
        scope: i64,
    ) -> rusqlite::Result<bool> {
        if !document.issues.is_empty() {
            return Err(rusqlite::Error::InvalidQuery);
        }
        if stamp(path) != Some(expected_stamp) {
            return Ok(false);
        }
        let (size, modified) = expected_stamp;
        let sheets =
            serde_json::to_string(&document.sheets).map_err(|_| rusqlite::Error::InvalidQuery)?;
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
        for unit in &document.units {
            let loose = fuzzy::loose_key(&unit.text);
            tx.execute("INSERT INTO units(file_path,source_kind,location,text,loose,unit_key,part_key,group_key,row_number,column_number,content_class,anchor) VALUES(?1,?2,?3,?4,?5,?6,?7,?8,?9,?10,?11,?12)",
                params![file_path.as_ref(), unit.source_kind, unit.location.to_string(), unit.text, loose, unit.meta.unit_key, unit.meta.part_key, unit.meta.group_key, unit.meta.row, unit.meta.column, unit.meta.content_class, unit.meta.anchor])?;
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
        if stamp(path) != Some(expected_stamp) {
            return Ok(false);
        }
        tx.execute(
            "INSERT INTO files(path,size,modified,extraction_version,extraction_scope,sheets) VALUES(?1,?2,?3,?4,?5,?6)",
            params![file_path.as_ref(), size, modified, EXTRACTION_VERSION, scope, sheets],
        )?;
        tx.commit()?;
        Ok(true)
    }

    pub fn candidates(
        &self,
        path: &Path,
        query: &fuzzy::Query,
        fuzzy_search: bool,
    ) -> rusqlite::Result<Vec<Unit>> {
        let needle = &query.loose;
        let gram_size = if needle.chars().count() >= 3 { 3 } else { 2 };
        // Stored grams remove separators; queries retaining them cannot be
        // rejected by those grams, including queries made only of separators.
        let query_grams = if !fuzzy_search
            || needle
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
        // A single-token match need not contain the full raw query's grams.
        // Union exact/prefix token candidates before the final evaluator runs.
        if fuzzy_search && query.tokens.len() == 1 {
            let token = &query.tokens[0];
            let mut statement = self.0.prepare("SELECT t.unit_id FROM tokens t JOIN units u ON u.id=t.unit_id WHERE u.file_path=?1 AND (t.token=?2 OR (?3 AND substr(t.token,1,length(?2))=?2))")?;
            let rows = statement.query_map(
                params![
                    path.to_string_lossy().as_ref(),
                    token,
                    token.chars().count() >= 3
                ],
                |row| row.get::<_, i64>(0),
            )?;
            for row in rows {
                ids.insert(row?);
            }
        }
        if let Some(typo) = query.typo.as_ref().filter(|_| fuzzy_search) {
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
            .prepare("SELECT source_kind, location, text, unit_key, part_key, group_key, row_number, column_number, content_class, anchor FROM units WHERE id=?1")?;
        let mut ordered_ids: Vec<i64> = ids.into_iter().collect();
        ordered_ids.sort_unstable();
        let mut result = Vec::with_capacity(ordered_ids.len());
        for id in ordered_ids {
            let unit = statement.query_row([id], |row| {
                let kind: String = row.get(0)?;
                let location: String = row.get(1)?;
                let text: String = row.get(2)?;
                let location =
                    serde_json::from_str(&location).map_err(|_| rusqlite::Error::InvalidQuery)?;
                let row_number: Option<i64> = row.get(6)?;
                let column_number: Option<i64> = row.get(7)?;
                Ok(Unit {
                    source_kind: source_kind(&kind)?,
                    location,
                    text,
                    meta: UnitMeta {
                        unit_key: row.get(3)?,
                        part_key: row.get(4)?,
                        group_key: row.get(5)?,
                        row: row_number
                            .map(|value| {
                                u32::try_from(value).map_err(|_| rusqlite::Error::InvalidQuery)
                            })
                            .transpose()?,
                        column: column_number
                            .map(|value| {
                                u32::try_from(value).map_err(|_| rusqlite::Error::InvalidQuery)
                            })
                            .transpose()?,
                        content_class: row.get(8)?,
                        anchor: row.get(9)?,
                    },
                })
            })?;
            result.push(unit);
        }
        Ok(result)
    }
}

fn source_kind(kind: &str) -> rusqlite::Result<&'static str> {
    match kind {
        "fileName" => Ok("fileName"),
        "cell" => Ok("cell"),
        "formula" => Ok("formula"),
        "note" => Ok("note"),
        "excelComment" => Ok("excelComment"),
        "wordHeader" => Ok("wordHeader"),
        "wordFooter" => Ok("wordFooter"),
        "wordComment" => Ok("wordComment"),
        "shape" => Ok("shape"),
        "slideTableCell" => Ok("slideTableCell"),
        "paragraph" => Ok("paragraph"),
        "wordTableParagraph" => Ok("wordTableParagraph"),
        "textLine" => Ok("textLine"),
        _ => Err(rusqlite::Error::InvalidQuery),
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::extract::{CellRange, SheetMeta};
    use serde_json::json;
    use std::time::SystemTime;

    #[test]
    fn old_index_schema_is_recreated() {
        let connection = Connection::open_in_memory().unwrap();
        connection.execute_batch("CREATE TABLE files(path TEXT PRIMARY KEY, size INTEGER, modified INTEGER); INSERT INTO files VALUES('old', 1, 1); PRAGMA user_version=1;").unwrap();
        initialize(&connection).unwrap();
        let version: i64 = connection
            .query_row("PRAGMA user_version", [], |row| row.get(0))
            .unwrap();
        let count: i64 = connection
            .query_row("SELECT COUNT(*) FROM files", [], |row| row.get(0))
            .unwrap();
        assert_eq!(version, SCHEMA_VERSION);
        assert_eq!(count, 0);
    }

    #[test]
    fn index_preserves_unit_and_sheet_metadata_and_invalidates_scope() {
        let connection = Connection::open_in_memory().unwrap();
        initialize(&connection).unwrap();
        let mut index = Index(connection);
        let suffix = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let path = std::env::temp_dir().join(format!(
            "docs-search-index-{}-{suffix}.txt",
            std::process::id()
        ));
        fs::write(&path, b"customer").unwrap();
        let original = stamp(&path).unwrap();
        let mut unit = Unit::new(
            "cell",
            json!({"sheetName":"Sheet1","cellAddress":"A12"}),
            "customer".into(),
        );
        unit.meta = UnitMeta {
            unit_key: "1".into(),
            part_key: "xl/worksheets/sheet1.xml".into(),
            group_key: "xl/worksheets/sheet1.xml#row:12".into(),
            row: Some(12),
            column: Some(1),
            content_class: "body".into(),
            anchor: None,
        };
        let document = ExtractedDocument {
            issues: vec![],
            units: vec![unit],
            sheets: vec![SheetMeta {
                part_key: "xl/worksheets/sheet1.xml".into(),
                sheet_name: "Sheet1".into(),
                merge_ranges: vec![CellRange {
                    first_row: 12,
                    first_column: 1,
                    last_row: 12,
                    last_column: 4,
                }],
                hidden_rows: vec![12],
                hidden_columns: vec![],
                ..SheetMeta::default()
            }],
        };
        assert!(index
            .replace(&path, &document, original, DEFAULT_SCOPE)
            .unwrap());
        assert!(index.current(&path, DEFAULT_SCOPE));
        assert!(!index.current(&path, 1));
        let units = index
            .candidates(&path, &fuzzy::Query::new("customer"), false)
            .unwrap();
        assert_eq!(units.len(), 1);
        assert_eq!(units[0].meta.group_key, document.units[0].meta.group_key);
        assert_eq!(units[0].meta.row, Some(12));
        assert_eq!(
            index
                .cells_in_range(&path, "xl/worksheets/sheet1.xml", (10, 14), (1, 5))
                .unwrap(),
            vec![(12, 1, "customer".into())]
        );
        assert!(index
            .cells_in_range(&path, "xl/worksheets/sheet1.xml", (10, 14), (2, 5))
            .unwrap()
            .is_empty());
        assert_eq!(
            index.sheet_metadata(&path).unwrap()[0].merge_ranges[0].last_column,
            4
        );
        index
            .0
            .execute(
                "UPDATE files SET sheets='broken' WHERE path=?1",
                [path.to_string_lossy().as_ref()],
            )
            .unwrap();
        assert!(!index.current(&path, DEFAULT_SCOPE));
        fs::write(&path, b"different customer").unwrap();
        assert!(!index.current(&path, DEFAULT_SCOPE));
        fs::remove_file(path).unwrap();
    }
}
