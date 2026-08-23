use rusqlite::{params, Connection, OptionalExtension, Result, Transaction};
use std::path::Path;
use std::sync::Mutex;
use tracing::info;

const SECRET_SETTING_KEYS: &[&str] = &["google_api_key"];
const CURRENT_SCHEMA_VERSION: u32 = 3;
const MAX_MEMORY_FACT_CHARS: usize = 1_024;
const MAX_METADATA_CHARS: usize = 64;
const MAX_TRANSCRIPT_TEXT_CHARS: usize = 16_384;
const MAX_SESSION_ID_CHARS: usize = 128;
const MAX_MEMORY_QUERY_LIMIT: usize = 200;
const MAX_TRANSCRIPT_QUERY_LIMIT: usize = 200;
const MAX_TRANSCRIPT_RECORDS: i64 = 1_000;
const EXPLICIT_MEMORY_SOURCE: &str = "remember_fact";

fn is_secret_setting_key(key: &str) -> bool {
    SECRET_SETTING_KEYS.contains(&key)
}

pub struct Database {
    conn: Mutex<Connection>,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct MemoryRecord {
    pub id: i64,
    pub fact: String,
    pub category: String,
    pub source: String,
    pub confidence: f64,
    pub created_at: String,
    pub updated_at: String,
}

fn memory_record_from_row(row: &rusqlite::Row<'_>) -> rusqlite::Result<MemoryRecord> {
    Ok(MemoryRecord {
        id: row.get(0)?,
        fact: row.get(1)?,
        category: row.get(2)?,
        source: row.get(3)?,
        confidence: row.get(4)?,
        created_at: row.get(5)?,
        updated_at: row.get(6)?,
    })
}

fn collect_memory_rows<M>(rows: M) -> Result<Vec<MemoryRecord>>
where
    M: Iterator<Item = rusqlite::Result<MemoryRecord>>,
{
    rows.collect()
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct TranscriptRecord {
    pub id: i64,
    pub session_id: String,
    pub role: String,
    pub text: String,
    pub created_at: String,
}

fn read_schema_version(conn: &Connection) -> Result<u32> {
    let table_exists: bool = conn.query_row(
        "SELECT EXISTS(SELECT 1 FROM sqlite_master WHERE type = 'table' AND name = 'schema_version')",
        [],
        |row| row.get(0),
    )?;
    if !table_exists {
        return Ok(0);
    }

    let version = conn
        .query_row("SELECT version FROM schema_version LIMIT 1", [], |row| {
            row.get::<_, i64>(0)
        })
        .optional()?
        .unwrap_or(0);
    u32::try_from(version).map_err(|_| rusqlite::Error::InvalidQuery)
}

fn table_has_column(conn: &Connection, table: &str, column: &str) -> Result<bool> {
    let mut stmt = conn.prepare(&format!("PRAGMA table_info({table})"))?;
    let rows = stmt.query_map([], |row| row.get::<_, String>(1))?;
    for name in rows {
        if name? == column {
            return Ok(true);
        }
    }
    Ok(false)
}

fn migration_1(tx: &Transaction<'_>) -> Result<()> {
    tx.execute_batch(
        "
        CREATE TABLE IF NOT EXISTS settings (
            key TEXT PRIMARY KEY,
            value TEXT NOT NULL
        );

        CREATE TABLE IF NOT EXISTS memories (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            fact TEXT NOT NULL,
            category TEXT NOT NULL,
            created_at DATETIME DEFAULT CURRENT_TIMESTAMP
        );

        CREATE TABLE IF NOT EXISTS observations (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            event_type TEXT NOT NULL,
            summary TEXT NOT NULL,
            created_at DATETIME DEFAULT CURRENT_TIMESTAMP
        );

        CREATE TABLE IF NOT EXISTS conversation_history (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            session_id TEXT NOT NULL,
            role TEXT NOT NULL,
            text TEXT NOT NULL,
            created_at DATETIME DEFAULT CURRENT_TIMESTAMP
        );
        ",
    )
}

fn migration_2(tx: &Transaction<'_>) -> Result<()> {
    if !table_has_column(tx, "memories", "source")? {
        tx.execute(
            "ALTER TABLE memories ADD COLUMN source TEXT NOT NULL DEFAULT 'legacy'",
            [],
        )?;
    }
    if !table_has_column(tx, "memories", "confidence")? {
        tx.execute(
            "ALTER TABLE memories ADD COLUMN confidence REAL NOT NULL DEFAULT 1.0",
            [],
        )?;
    }
    if !table_has_column(tx, "memories", "updated_at")? {
        tx.execute("ALTER TABLE memories ADD COLUMN updated_at TEXT", [])?;
    }
    tx.execute(
        "UPDATE memories SET updated_at = COALESCE(updated_at, created_at)",
        [],
    )?;
    Ok(())
}

fn migration_3(tx: &Transaction<'_>) -> Result<()> {
    tx.execute_batch(
        "
        CREATE INDEX IF NOT EXISTS idx_conversation_history_session_id
            ON conversation_history(session_id, id);
        CREATE INDEX IF NOT EXISTS idx_memories_updated_at
            ON memories(updated_at, id);
        ",
    )
}

fn set_schema_version(tx: &Transaction<'_>, version: u32) -> Result<()> {
    tx.execute_batch(
        "CREATE TABLE IF NOT EXISTS schema_version (
            version INTEGER NOT NULL
        );
        DELETE FROM schema_version;",
    )?;
    tx.execute(
        "INSERT INTO schema_version(version) VALUES (?1)",
        params![i64::from(version)],
    )?;
    Ok(())
}

fn run_migrations(conn: &mut Connection) -> Result<()> {
    let current = read_schema_version(conn)?;
    if current > CURRENT_SCHEMA_VERSION {
        return Err(rusqlite::Error::InvalidQuery);
    }
    if current == CURRENT_SCHEMA_VERSION {
        return Ok(());
    }

    // Apply the entire pending chain atomically. A failure rolls back every schema
    // mutation from this startup attempt, so V1 does not need to create duplicate
    // backup files containing private memories/transcripts or legacy settings.
    let tx = conn.transaction()?;
    for target in (current + 1)..=CURRENT_SCHEMA_VERSION {
        match target {
            1 => migration_1(&tx)?,
            2 => migration_2(&tx)?,
            3 => migration_3(&tx)?,
            _ => return Err(rusqlite::Error::InvalidQuery),
        }
        set_schema_version(&tx, target)?;
    }
    tx.commit()
}

fn bounded_text(value: &str, name: &str, max_chars: usize) -> Result<String> {
    let trimmed = value.trim();
    if trimmed.is_empty() || trimmed.chars().count() > max_chars {
        return Err(rusqlite::Error::InvalidParameterName(name.to_string()));
    }
    Ok(trimmed.to_string())
}

impl Database {
    pub fn new<P: AsRef<Path>>(path: P) -> Result<Self> {
        let path = path.as_ref();
        let mut conn = Connection::open(path)?;
        run_migrations(&mut conn)?;
        info!(
            schema_version = CURRENT_SCHEMA_VERSION,
            "SQLite database schema initialized successfully"
        );
        Ok(Self {
            conn: Mutex::new(conn),
        })
    }

    pub fn new_in_memory() -> Result<Self> {
        let mut conn = Connection::open_in_memory()?;
        run_migrations(&mut conn)?;
        Ok(Self {
            conn: Mutex::new(conn),
        })
    }

    pub fn schema_version(&self) -> Result<u32> {
        let conn = self.conn.lock().unwrap();
        read_schema_version(&conn)
    }

    // --- Settings CRUD ---

    pub fn set_setting(&self, key: &str, value: &str) -> Result<()> {
        if is_secret_setting_key(key) {
            return Err(rusqlite::Error::InvalidParameterName(key.to_string()));
        }

        let conn = self.conn.lock().unwrap();
        conn.execute(
            "INSERT INTO settings (key, value) VALUES (?1, ?2)
             ON CONFLICT(key) DO UPDATE SET value = ?2",
            params![key, value],
        )?;
        Ok(())
    }

    pub fn get_setting(&self, key: &str) -> Result<Option<String>> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn.prepare("SELECT value FROM settings WHERE key = ?1")?;
        let mut rows = stmt.query(params![key])?;
        if let Some(row) = rows.next()? {
            Ok(Some(row.get(0)?))
        } else {
            Ok(None)
        }
    }

    pub fn delete_setting(&self, key: &str) -> Result<bool> {
        let conn = self.conn.lock().unwrap();
        let affected = conn.execute("DELETE FROM settings WHERE key = ?1", params![key])?;
        Ok(affected > 0)
    }

    #[cfg(test)]
    pub(crate) fn seed_legacy_setting_for_test(&self, key: &str, value: &str) -> Result<()> {
        let conn = self.conn.lock().unwrap();
        conn.execute(
            "INSERT INTO settings (key, value) VALUES (?1, ?2)
             ON CONFLICT(key) DO UPDATE SET value = ?2",
            params![key, value],
        )?;
        Ok(())
    }

    // --- Memories CRUD ---

    pub fn add_memory(&self, fact: &str, category: &str) -> Result<i64> {
        let fact = bounded_text(fact, "memory fact", MAX_MEMORY_FACT_CHARS)?;
        let category = bounded_text(category, "memory category", MAX_METADATA_CHARS)?;
        let conn = self.conn.lock().unwrap();

        let existing = conn
            .query_row(
                "SELECT id FROM memories
                 WHERE lower(trim(fact)) = lower(?1)
                 ORDER BY id DESC LIMIT 1",
                params![fact],
                |row| row.get::<_, i64>(0),
            )
            .optional()?;

        if let Some(id) = existing {
            conn.execute(
                "UPDATE memories
                 SET fact = ?1,
                     category = ?2,
                     source = ?3,
                     confidence = 1.0,
                     updated_at = strftime('%Y-%m-%dT%H:%M:%fZ', 'now')
                 WHERE id = ?4",
                params![fact, category, EXPLICIT_MEMORY_SOURCE, id],
            )?;
            return Ok(id);
        }

        conn.execute(
            "INSERT INTO memories (
                fact, category, source, confidence, created_at, updated_at
             ) VALUES (
                ?1, ?2, ?3, 1.0,
                strftime('%Y-%m-%dT%H:%M:%fZ', 'now'),
                strftime('%Y-%m-%dT%H:%M:%fZ', 'now')
             )",
            params![fact, category, EXPLICIT_MEMORY_SOURCE],
        )?;
        Ok(conn.last_insert_rowid())
    }

    pub fn get_memories(&self) -> Result<Vec<MemoryRecord>> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn.prepare(
            "SELECT id, fact, category, source, confidence, created_at,
                    COALESCE(updated_at, created_at)
             FROM memories
             ORDER BY COALESCE(updated_at, created_at) DESC, id DESC",
        )?;
        collect_memory_rows(stmt.query_map([], memory_record_from_row)?)
    }

    pub fn get_recent_memories(&self, limit: usize) -> Result<Vec<MemoryRecord>> {
        let bounded_limit = limit.min(MAX_MEMORY_QUERY_LIMIT);
        if bounded_limit == 0 {
            return Ok(Vec::new());
        }
        let bounded_limit =
            i64::try_from(bounded_limit).map_err(|_| rusqlite::Error::InvalidQuery)?;
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn.prepare(
            "SELECT id, fact, category, source, confidence, created_at,
                    COALESCE(updated_at, created_at)
             FROM memories
             ORDER BY COALESCE(updated_at, created_at) DESC, id DESC
             LIMIT ?1",
        )?;
        collect_memory_rows(stmt.query_map(params![bounded_limit], memory_record_from_row)?)
    }

    pub fn delete_memory(&self, id: i64) -> Result<bool> {
        let conn = self.conn.lock().unwrap();
        let affected = conn.execute("DELETE FROM memories WHERE id = ?1", params![id])?;
        Ok(affected > 0)
    }

    // --- Transcript CRUD ---

    pub fn add_transcript(&self, session_id: &str, role: &str, text: &str) -> Result<i64> {
        let session_id = bounded_text(session_id, "transcript session ID", MAX_SESSION_ID_CHARS)?;
        if !matches!(role, "user" | "moose") {
            return Err(rusqlite::Error::InvalidParameterName(
                "transcript role".to_string(),
            ));
        }
        let text = bounded_text(text, "transcript text", MAX_TRANSCRIPT_TEXT_CHARS)?;
        let conn = self.conn.lock().unwrap();

        let latest = conn
            .query_row(
                "SELECT id, role, text
                 FROM conversation_history
                 WHERE session_id = ?1
                 ORDER BY id DESC LIMIT 1",
                params![session_id],
                |row| {
                    Ok((
                        row.get::<_, i64>(0)?,
                        row.get::<_, String>(1)?,
                        row.get::<_, String>(2)?,
                    ))
                },
            )
            .optional()?;
        if let Some((id, previous_role, previous_text)) = latest {
            if previous_role == role && previous_text == text {
                return Ok(id);
            }
        }

        conn.execute(
            "INSERT INTO conversation_history (
                session_id, role, text, created_at
             ) VALUES (
                ?1, ?2, ?3, strftime('%Y-%m-%dT%H:%M:%fZ', 'now')
             )",
            params![session_id, role, text],
        )?;
        let id = conn.last_insert_rowid();
        conn.execute(
            "DELETE FROM conversation_history
             WHERE id NOT IN (
                 SELECT id FROM conversation_history
                 ORDER BY id DESC LIMIT ?1
             )",
            params![MAX_TRANSCRIPT_RECORDS],
        )?;
        Ok(id)
    }

    pub fn get_transcripts(&self, limit: usize) -> Result<Vec<TranscriptRecord>> {
        if limit == 0 {
            return Ok(Vec::new());
        }
        let bounded_limit = limit.min(MAX_TRANSCRIPT_QUERY_LIMIT);
        let bounded_limit =
            i64::try_from(bounded_limit).map_err(|_| rusqlite::Error::InvalidQuery)?;
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn.prepare(
            "SELECT id, session_id, role, text, created_at
             FROM (
                 SELECT id, session_id, role, text, created_at
                 FROM conversation_history
                 WHERE role IN ('user', 'moose')
                 ORDER BY id DESC
                 LIMIT ?1
             )
             ORDER BY id ASC",
        )?;
        let rows = stmt.query_map(params![bounded_limit], |row| {
            Ok(TranscriptRecord {
                id: row.get(0)?,
                session_id: row.get(1)?,
                role: row.get(2)?,
                text: row.get(3)?,
                created_at: row.get(4)?,
            })
        })?;

        let mut list = Vec::new();
        for row in rows {
            list.push(row?);
        }
        Ok(list)
    }

    // --- Forget Everything ---

    pub fn forget_everything(&self) -> Result<()> {
        let conn = self.conn.lock().unwrap();
        conn.execute_batch(
            "
            DELETE FROM memories;
            DELETE FROM observations;
            DELETE FROM conversation_history;
            ",
        )?;
        info!("Forget Everything executed: purged memories, observations, and transcripts");
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    fn legacy_schema() -> &'static str {
        "
        CREATE TABLE settings (
            key TEXT PRIMARY KEY,
            value TEXT NOT NULL
        );
        CREATE TABLE memories (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            fact TEXT NOT NULL,
            category TEXT NOT NULL,
            created_at DATETIME DEFAULT CURRENT_TIMESTAMP
        );
        CREATE TABLE observations (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            event_type TEXT NOT NULL,
            summary TEXT NOT NULL,
            created_at DATETIME DEFAULT CURRENT_TIMESTAMP
        );
        CREATE TABLE conversation_history (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            session_id TEXT NOT NULL,
            role TEXT NOT NULL,
            text TEXT NOT NULL,
            created_at DATETIME DEFAULT CURRENT_TIMESTAMP
        );
        "
    }

    #[test]
    fn database_crud_and_forget() {
        let db = Database::new_in_memory().unwrap();
        assert_eq!(db.schema_version().unwrap(), CURRENT_SCHEMA_VERSION);

        db.set_setting("talkativeness", "0.8").unwrap();
        assert_eq!(
            db.get_setting("talkativeness").unwrap(),
            Some("0.8".to_string())
        );
        assert!(db
            .set_setting("google_api_key", "must-not-persist")
            .is_err());
        assert_eq!(db.get_setting("google_api_key").unwrap(), None);

        let mem_id = db.add_memory("User likes coffee", "preference").unwrap();
        let memories = db.get_memories().unwrap();
        assert_eq!(memories.len(), 1);
        assert_eq!(memories[0].fact, "User likes coffee");
        assert_eq!(memories[0].source, EXPLICIT_MEMORY_SOURCE);
        assert_eq!(memories[0].confidence, 1.0);

        db.add_transcript("sess-1", "user", "Hello").unwrap();
        db.add_transcript("sess-1", "moose", "Hi there").unwrap();
        assert_eq!(db.get_transcripts(10).unwrap().len(), 2);

        db.delete_memory(mem_id).unwrap();
        assert!(db.get_memories().unwrap().is_empty());

        db.add_memory("User likes tea", "preference").unwrap();
        db.forget_everything().unwrap();
        assert!(db.get_memories().unwrap().is_empty());
        assert!(db.get_transcripts(10).unwrap().is_empty());
    }

    #[test]
    fn legacy_schema_migrates_idempotently_and_preserves_data() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("legacy.sqlite");
        let conn = Connection::open(&path).unwrap();
        conn.execute_batch(legacy_schema()).unwrap();
        conn.execute(
            "INSERT INTO memories(fact, category) VALUES (?1, ?2)",
            params!["User likes coffee", "preference"],
        )
        .unwrap();
        conn.execute(
            "INSERT INTO conversation_history(session_id, role, text) VALUES (?1, ?2, ?3)",
            params!["legacy-session", "user", "legacy transcript"],
        )
        .unwrap();
        drop(conn);

        let db = Database::new(&path).unwrap();
        assert_eq!(db.schema_version().unwrap(), CURRENT_SCHEMA_VERSION);
        let memories = db.get_memories().unwrap();
        assert_eq!(memories.len(), 1);
        assert_eq!(memories[0].source, "legacy");
        assert_eq!(memories[0].confidence, 1.0);
        assert_eq!(db.get_transcripts(10).unwrap()[0].text, "legacy transcript");
        drop(db);

        let reopened = Database::new(&path).unwrap();
        assert_eq!(reopened.schema_version().unwrap(), CURRENT_SCHEMA_VERSION);
        let memories = reopened.get_memories().unwrap();
        assert_eq!(memories.len(), 1);
        assert_eq!(memories[0].fact, "User likes coffee");
        assert_eq!(reopened.get_transcripts(10).unwrap().len(), 1);
    }

    #[test]
    fn failed_migration_rolls_back_version_and_aborts_open() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("broken-v1.sqlite");
        let conn = Connection::open(&path).unwrap();
        conn.execute_batch(
            "CREATE TABLE schema_version(version INTEGER NOT NULL);
             INSERT INTO schema_version(version) VALUES (1);",
        )
        .unwrap();
        drop(conn);

        assert!(Database::new(&path).is_err());

        let conn = Connection::open(&path).unwrap();
        let version: i64 = conn
            .query_row("SELECT version FROM schema_version", [], |row| row.get(0))
            .unwrap();
        assert_eq!(version, 1);
        let source_column_exists = table_has_column(&conn, "memories", "source").unwrap();
        assert!(!source_column_exists);
    }

    #[test]
    fn memory_duplicate_updates_metadata_without_duplicating_fact() {
        let db = Database::new_in_memory().unwrap();
        let first = db.add_memory("User likes coffee", "preference").unwrap();
        let second = db.add_memory("  user likes coffee  ", "profile").unwrap();
        assert_eq!(first, second);

        let memories = db.get_memories().unwrap();
        assert_eq!(memories.len(), 1);
        assert_eq!(memories[0].fact, "user likes coffee");
        assert_eq!(memories[0].category, "profile");
        assert_eq!(memories[0].source, EXPLICIT_MEMORY_SOURCE);
        assert_eq!(memories[0].confidence, 1.0);
        assert!(!memories[0].created_at.is_empty());
        assert!(!memories[0].updated_at.is_empty());
    }

    #[test]
    fn transcript_query_returns_recent_window_in_chronological_order() {
        let db = Database::new_in_memory().unwrap();
        for index in 0..6 {
            let role = if index % 2 == 0 { "user" } else { "moose" };
            db.add_transcript("session", role, &format!("line-{index}"))
                .unwrap();
        }

        let transcripts = db.get_transcripts(3).unwrap();
        let text: Vec<_> = transcripts
            .iter()
            .map(|record| record.text.as_str())
            .collect();
        assert_eq!(text, vec!["line-3", "line-4", "line-5"]);
    }

    #[test]
    fn duplicate_final_transcript_is_idempotent_and_retention_is_bounded() {
        let db = Database::new_in_memory().unwrap();
        let first = db.add_transcript("session", "user", "same final").unwrap();
        let duplicate = db
            .add_transcript("session", "user", " same final ")
            .unwrap();
        assert_eq!(first, duplicate);

        for index in 0..1_005 {
            let role = if index % 2 == 0 { "user" } else { "moose" };
            db.add_transcript("retention", role, &format!("line-{index}"))
                .unwrap();
        }
        let conn = db.conn.lock().unwrap();
        let count: i64 = conn
            .query_row("SELECT COUNT(*) FROM conversation_history", [], |row| {
                row.get(0)
            })
            .unwrap();
        assert!(count <= MAX_TRANSCRIPT_RECORDS);
    }

    #[test]
    fn transcript_validation_rejects_partial_roles_and_unbounded_queries() {
        let db = Database::new_in_memory().unwrap();
        assert!(db
            .add_transcript("session", "user_partial", "partial")
            .is_err());
        for index in 0..250 {
            let role = if index % 2 == 0 { "user" } else { "moose" };
            db.add_transcript("session", role, &format!("line-{index}"))
                .unwrap();
        }
        assert_eq!(
            db.get_transcripts(usize::MAX).unwrap().len(),
            MAX_TRANSCRIPT_QUERY_LIMIT
        );
    }

    #[test]
    fn v1_schema_does_not_create_conversation_summaries() {
        let db = Database::new_in_memory().unwrap();
        let conn = db.conn.lock().unwrap();
        let exists: bool = conn
            .query_row(
                "SELECT EXISTS(SELECT 1 FROM sqlite_master WHERE type = 'table' AND name = 'conversation_summaries')",
                [],
                |row| row.get(0),
            )
            .unwrap();
        assert!(!exists);
    }
}
