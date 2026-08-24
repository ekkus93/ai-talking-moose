use rusqlite::{params, Connection, OptionalExtension, Result, Transaction};
use std::path::Path;
use std::sync::Mutex;
use tracing::info;

const SECRET_SETTING_KEYS: &[&str] = &["google_api_key"];
const CURRENT_SCHEMA_VERSION: u32 = 4;
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

fn migration_4(tx: &Transaction<'_>) -> Result<()> {
    // Desktop observations are intentionally transient in V1. Older schemas
    // created this table before the zero-persistence policy was finalized; drop
    // it so a migrated profile cannot retain stale observation summaries.
    tx.execute_batch("DROP TABLE IF EXISTS observations;")
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
            4 => migration_4(&tx)?,
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

fn configure_connection(conn: &Connection) -> Result<()> {
    // Ensure deleted private rows are overwritten inside the SQLite database file
    // rather than left readable in freelist pages until later reuse.
    conn.execute_batch("PRAGMA secure_delete = ON;")
}

