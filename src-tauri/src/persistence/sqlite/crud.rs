impl Database {
    pub fn new<P: AsRef<Path>>(path: P) -> Result<Self> {
        let path = path.as_ref();
        let mut conn = Connection::open(path)?;
        configure_connection(&conn)?;
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
        configure_connection(&conn)?;
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
        let rows = stmt.query_map([], memory_record_from_row)?;
        collect_memory_rows(rows)
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
        let rows = stmt.query_map(params![bounded_limit], memory_record_from_row)?;
        collect_memory_rows(rows)
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

    pub fn clear_transcripts(&self) -> Result<()> {
        let conn = self.conn.lock().unwrap();
        conn.execute("DELETE FROM conversation_history", [])?;
        Ok(())
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
        let mut conn = self.conn.lock().unwrap();
        let tx = conn.transaction()?;
        tx.execute("DELETE FROM memories", [])?;
        tx.execute("DELETE FROM conversation_history", [])?;
        // Defensive cleanup for a database whose old observation table was
        // recreated or copied in after migration. V1 retains no observation rows.
        tx.execute_batch("DROP TABLE IF EXISTS observations;")?;
        tx.commit()?;
        info!("Forget Everything executed: purged private persistence");
        Ok(())
    }
}

