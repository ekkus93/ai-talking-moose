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
    fn database_connections_enable_secure_delete() {
        let db = Database::new_in_memory().unwrap();
        let conn = db.conn.lock().unwrap();
        let enabled: i64 = conn
            .query_row("PRAGMA secure_delete", [], |row| row.get(0))
            .unwrap();
        assert_ne!(enabled, 0);
    }

    #[test]
    fn poisoned_database_mutex_does_not_brick_private_persistence() {
        let db = Database::new_in_memory().unwrap();
        let poisoned = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            let _conn = db.conn.lock().unwrap();
            panic!("inject database mutex poison");
        }));
        assert!(poisoned.is_err());

        db.set_setting("app_settings", "survives-poison").unwrap();
        db.add_memory("User likes resilient storage", "preference")
            .unwrap();
        db.add_transcript("poison-test", "user", "still persisted")
            .unwrap();

        assert_eq!(
            db.get_setting("app_settings").unwrap().as_deref(),
            Some("survives-poison")
        );
        assert_eq!(db.get_memories().unwrap().len(), 1);
        assert_eq!(db.get_transcripts(10).unwrap().len(), 1);
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
        conn.execute(
            "INSERT INTO observations(event_type, summary) VALUES (?1, ?2)",
            params!["active_app", "Private legacy observation"],
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
        {
            let conn = db.conn.lock().unwrap();
            let observations_exist: bool = conn
                .query_row(
                    "SELECT EXISTS(SELECT 1 FROM sqlite_master WHERE type = 'table' AND name = 'observations')",
                    [],
                    |row| row.get(0),
                )
                .unwrap();
            assert!(!observations_exist);
        }
        drop(db);

        let reopened = Database::new(&path).unwrap();
        assert_eq!(reopened.schema_version().unwrap(), CURRENT_SCHEMA_VERSION);
        let memories = reopened.get_memories().unwrap();
        assert_eq!(memories.len(), 1);
        assert_eq!(memories[0].fact, "User likes coffee");
        assert_eq!(reopened.get_transcripts(10).unwrap().len(), 1);
    }

    #[test]
    fn schema_v3_migration_drops_legacy_observations_and_preserves_private_opt_in_data() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("v3.sqlite");
        let conn = Connection::open(&path).unwrap();
        conn.execute_batch(
            "
            CREATE TABLE schema_version(version INTEGER NOT NULL);
            INSERT INTO schema_version(version) VALUES (3);
            CREATE TABLE settings (
                key TEXT PRIMARY KEY,
                value TEXT NOT NULL
            );
            CREATE TABLE memories (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                fact TEXT NOT NULL,
                category TEXT NOT NULL,
                source TEXT NOT NULL DEFAULT 'legacy',
                confidence REAL NOT NULL DEFAULT 1.0,
                created_at DATETIME DEFAULT CURRENT_TIMESTAMP,
                updated_at TEXT
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
            CREATE INDEX idx_conversation_history_session_id
                ON conversation_history(session_id, id);
            CREATE INDEX idx_memories_updated_at
                ON memories(updated_at, id);
            INSERT INTO memories(fact, category, source, confidence, updated_at)
                VALUES ('User likes tea', 'preference', 'remember_fact', 1.0, CURRENT_TIMESTAMP);
            INSERT INTO conversation_history(session_id, role, text)
                VALUES ('session', 'user', 'retained transcript');
            INSERT INTO observations(event_type, summary)
                VALUES ('active_app', 'Private legacy observation');
            ",
        )
        .unwrap();
        drop(conn);

        let db = Database::new(&path).unwrap();
        assert_eq!(db.schema_version().unwrap(), 4);
        assert_eq!(db.get_memories().unwrap().len(), 1);
        assert_eq!(db.get_transcripts(10).unwrap().len(), 1);
        let conn = db.conn.lock().unwrap();
        let observations_exist: bool = conn
            .query_row(
                "SELECT EXISTS(SELECT 1 FROM sqlite_master WHERE type = 'table' AND name = 'observations')",
                [],
                |row| row.get(0),
            )
            .unwrap();
        assert!(!observations_exist);
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
    fn clear_transcripts_preserves_memories_and_settings() {
        let db = Database::new_in_memory().unwrap();
        db.set_setting("app_settings", "keep-me").unwrap();
        db.add_memory("User likes tea", "preference").unwrap();
        db.add_transcript("session", "user", "private transcript")
            .unwrap();

        db.clear_transcripts().unwrap();

        assert!(db.get_transcripts(10).unwrap().is_empty());
        assert_eq!(db.get_memories().unwrap().len(), 1);
        assert_eq!(
            db.get_setting("app_settings").unwrap(),
            Some("keep-me".to_string())
        );
    }

    #[test]
    fn forget_everything_rolls_back_all_private_deletes_on_failure() {
        let db = Database::new_in_memory().unwrap();
        db.add_memory("User likes tea", "preference").unwrap();
        db.add_transcript("session", "user", "private transcript")
            .unwrap();
        {
            let conn = db.conn.lock().unwrap();
            conn.execute_batch(
                "CREATE TRIGGER reject_transcript_delete
                 BEFORE DELETE ON conversation_history
                 BEGIN
                     SELECT RAISE(ABORT, 'reject transcript delete');
                 END;",
            )
            .unwrap();
        }

        assert!(db.forget_everything().is_err());
        assert_eq!(db.get_memories().unwrap().len(), 1);
        assert_eq!(db.get_transcripts(10).unwrap().len(), 1);
    }

    #[test]
    fn forget_everything_is_private_data_only_and_observations_are_zero_retention() {
        let db = Database::new_in_memory().unwrap();
        db.set_setting("app_settings", "keep-me").unwrap();
        db.add_memory("User likes tea", "preference").unwrap();
        db.add_transcript("session", "user", "private transcript")
            .unwrap();

        db.forget_everything().unwrap();

        assert!(db.get_memories().unwrap().is_empty());
        assert!(db.get_transcripts(10).unwrap().is_empty());
        assert_eq!(
            db.get_setting("app_settings").unwrap(),
            Some("keep-me".to_string())
        );
        let conn = db.conn.lock().unwrap();
        let observations_exist: bool = conn
            .query_row(
                "SELECT EXISTS(SELECT 1 FROM sqlite_master WHERE type = 'table' AND name = 'observations')",
                [],
                |row| row.get(0),
            )
            .unwrap();
        assert!(!observations_exist);
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
