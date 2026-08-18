use crate::tools::builtin::BuiltinTools;
use crate::tools::policy::ToolDeclaration;
use std::sync::Arc;
use tracing::{info, warn};

pub struct ToolRouter {
    builtin: Arc<BuiltinTools>,
}

impl ToolRouter {
    pub fn new(builtin: Arc<BuiltinTools>) -> Self {
        Self { builtin }
    }

    pub fn get_declarations(&self) -> Vec<ToolDeclaration> {
        self.builtin.get_declarations()
    }

    pub async fn dispatch(
        &self,
        name: &str,
        arguments: &serde_json::Value,
    ) -> Result<serde_json::Value, String> {
        info!("Executing tool: {}", name);
        match self.builtin.execute(name, arguments).await {
            Ok(val) => {
                info!("Tool {} executed successfully", name);
                Ok(val)
            }
            Err(e) => {
                warn!("Tool {} execution failed: {}", name, e);
                Err(e)
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::character::personality::CharacterConfig;
    use crate::persistence::sqlite::Database;
    use serde_json::json;

    #[tokio::test]
    async fn test_tool_router_time_and_memory() {
        let db = Arc::new(Database::new_in_memory().unwrap());
        let mem = Arc::new(crate::memory::MemoryManager::new(db));
        let builtin = Arc::new(BuiltinTools {
            memory_manager: mem.clone(),
            character_config: CharacterConfig::default(),
            active_app_permitted: true,
        });
        let router = ToolRouter::new(builtin);

        // Test time tool
        let res = router
            .dispatch("get_current_time", &json!({}))
            .await
            .unwrap();
        assert!(res.get("time").is_some());

        // Test remember_fact tool
        let rem_res = router
            .dispatch(
                "remember_fact",
                &json!({ "fact": "User is building Talking Moose" }),
            )
            .await
            .unwrap();
        assert_eq!(rem_res["status"], "remembered");

        let memories = mem.get_all_memories().unwrap();
        assert_eq!(memories.len(), 1);

        // Test arbitrary tool rejection
        let bad_tool = router
            .dispatch("execute_shell", &json!({ "cmd": "ls" }))
            .await;
        assert!(bad_tool.is_err());
    }
}
