use rusqlite::{params, Connection, Result};
use std::path::Path;
use std::sync::Mutex;
use tracing::info;

const SECRET_SETTING_KEYS: &[&str] = &["google_api_key"];

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
    pub created_at: String,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct TranscriptRecord {
    pub id: i64,
    pub session_id: String,
    pub role: String,
    pub text: String,
    pub created_at: String,
}

impl Database {
    pub fn new<P: AsRef<Path>>(path: P) -> Result<Self> {
        let conn = Connection::open(path)?;
        let db = Self {
            conn: Mutex::new(conn),
        };
        db.init_schema()?;
        Ok(db)
    }

    pub fn new_in_memory() -> Result<Self> {
        let conn = Connection::open_in_memory()?;
        let db = Self {
            conn: Mutex::new(conn),
        };
        db.init_schema()?;
        Ok(db)
    }

    fn init_schema(&self) -> Result<()> {
        let conn = self.conn.lock().unwrap();

        conn.execute_batch(
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
        )?;

        info!("SQLite database schema initialized successfully");
        Ok(())
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
        let conn = self.conn.lock().unwrap();
        conn.execute(
            "INSERT INTO memories (fact, category) VALUES (?1, ?2)",
            params![fact, category],
        )?;
        Ok(conn.last_insert_rowid())
    }

    pub fn get_memories(&self) -> Result<Vec<MemoryRecord>> {
        let conn = self.conn.lock().unwrap();
        let mut stmt =
            conn.prepare("SELECT id, fact, category, created_at FROM memories ORDER BY id DESC")?;
        let rows = stmt.query_map([], |row| {
            Ok(MemoryRecord {
                id: row.get(0)?,
                fact: row.get(1)?,
                category: row.get(2)?,
                created_at: row.get(3)?,
            })
        })?;

        let mut list = Vec::new();
        for r in rows {
            list.push(r?);
        }
        Ok(list)
    }

    pub fn delete_memory(&self, id: i64) -> Result<bool> {
        let conn = self.conn.lock().unwrap();
        let affected = conn.execute("DELETE FROM memories WHERE id = ?1", params![id])?;
        Ok(affected > 0)
    }

    // --- Transcript CRUD ---

    pub fn add_transcript(&self, session_id: &str, role: &str, text: &str) -> Result<i64> {
        let conn = self.conn.lock().unwrap();
        conn.execute(
            "INSERT INTO conversation_history (session_id, role, text) VALUES (?1, ?2, ?3)",
            params![session_id, role, text],
        )?;
        Ok(conn.last_insert_rowid())
    }

    pub fn get_transcripts(&self, limit: usize) -> Result<Vec<TranscriptRecord>> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn.prepare(
            "SELECT id, session_id, role, text, created_at FROM conversation_history ORDER BY id ASC LIMIT ?1",
        )?;
        let rows = stmt.query_map(params![limit as i64], |row| {
            Ok(TranscriptRecord {
                id: row.get(0)?,
                session_id: row.get(1)?,
                role: row.get(2)?,
                text: row.get(3)?,
                created_at: row.get(4)?,
            })
        })?;

        let mut list = Vec::new();
        for r in rows {
            list.push(r?);
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

    #[test]
    fn test_database_crud_and_forget() {
        let db = Database::new_in_memory().unwrap();

        // Settings
        db.set_setting("talkativeness", "0.8").unwrap();
        assert_eq!(
            db.get_setting("talkativeness").unwrap(),
            Some("0.8".to_string())
        );
        assert!(db
            .set_setting("google_api_key", "must-not-persist")
            .is_err());
        assert_eq!(db.get_setting("google_api_key").unwrap(), None);

        db.set_setting("temporary", "value").unwrap();
        assert!(db.delete_setting("temporary").unwrap());
        assert_eq!(db.get_setting("temporary").unwrap(), None);

        // Memory
        let mem_id = db.add_memory("User likes coffee", "fact").unwrap();
        let memories = db.get_memories().unwrap();
        assert_eq!(memories.len(), 1);
        assert_eq!(memories[0].fact, "User likes coffee");

        // Transcripts
        db.add_transcript("sess-1", "user", "Hello").unwrap();
        db.add_transcript("sess-1", "moose", "Hi there").unwrap();
        let transcripts = db.get_transcripts(10).unwrap();
        assert_eq!(transcripts.len(), 2);

        // Delete single memory
        db.delete_memory(mem_id).unwrap();
        assert_eq!(db.get_memories().unwrap().len(), 0);

        // Re-add and Forget Everything
        db.add_memory("User likes tea", "fact").unwrap();
        db.forget_everything().unwrap();
        assert_eq!(db.get_memories().unwrap().len(), 0);
        assert_eq!(db.get_transcripts(10).unwrap().len(), 0);
    }
}
