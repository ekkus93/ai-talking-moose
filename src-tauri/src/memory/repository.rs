use crate::persistence::sqlite::{Database, MemoryRecord};
use std::sync::Arc;

pub(crate) const MAX_MODEL_MEMORY_RECORDS: usize = 64;

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
            .get_recent_memories(MAX_MODEL_MEMORY_RECORDS)
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn model_memory_retrieval_has_a_hard_record_cap_and_prefers_recent_facts() {
        let db = Arc::new(Database::new_in_memory().unwrap());
        let memory = MemoryManager::new(db);
        for index in 0..(MAX_MODEL_MEMORY_RECORDS + 5) {
            memory
                .remember(&format!("fact-{index}"), Some("test"))
                .unwrap();
        }

        let facts = memory.get_memory_strings();
        assert_eq!(facts.len(), MAX_MODEL_MEMORY_RECORDS);
        assert_eq!(facts.first().map(String::as_str), Some("fact-68"));
        assert_eq!(facts.last().map(String::as_str), Some("fact-5"));
        assert!(!facts.iter().any(|fact| fact == "fact-0"));
    }
}
