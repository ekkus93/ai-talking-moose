use crate::persistence::sqlite::{Database, MemoryRecord};
use std::sync::Arc;

#[derive(Clone)]
pub struct MemoryManager {
    db: Arc<Database>,
}

impl MemoryManager {
    pub fn new(db: Arc<Database>) -> Self {
        Self { db }
    }

    pub fn remember(&self, fact: &str, category: Option<&str>) -> Result<i64, String> {
        let cat = category.unwrap_or("general");
        self.db
            .add_memory(fact, cat)
            .map_err(|e| format!("Failed to save memory: {}", e))
    }

    pub fn get_all_memories(&self) -> Result<Vec<MemoryRecord>, String> {
        self.db
            .get_memories()
            .map_err(|e| format!("Failed to retrieve memories: {}", e))
    }

    pub fn get_memory_strings(&self) -> Vec<String> {
        self.db
            .get_memories()
            .unwrap_or_default()
            .into_iter()
            .map(|m| m.fact)
            .collect()
    }

    pub fn forget(&self, id: i64) -> Result<bool, String> {
        self.db
            .delete_memory(id)
            .map_err(|e| format!("Failed to delete memory: {}", e))
    }

    pub fn forget_everything(&self) -> Result<(), String> {
        self.db
            .forget_everything()
            .map_err(|e| format!("Failed to reset memory: {}", e))
    }
}
