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
        info!(tool = name, "Executing tool");
        match self.builtin.execute(name, arguments).await {
            Ok(val) => {
                info!(tool = name, "Tool executed successfully");
                Ok(val)
            }
            Err(error) => {
                // Tool arguments/results can contain memory or desktop data. Keep
                // normal logs to non-sensitive metadata; return the error to the
                // caller without echoing it into the log stream.
                warn!(tool = name, "Tool execution failed");
                Err(error)
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::app::state::AppSettings;
    use crate::character::personality::CharacterConfig;
    use crate::persistence::sqlite::Database;
    use parking_lot::RwLock;
    use serde_json::json;

    #[tokio::test]
    async fn test_tool_router_time_and_memory() {
        let db = Arc::new(Database::new_in_memory().unwrap());
        let mem = Arc::new(crate::memory::MemoryManager::new(db));
        let settings = Arc::new(RwLock::new(AppSettings::default()));
        let builtin = Arc::new(BuiltinTools {
            memory_manager: mem.clone(),
            character_config: CharacterConfig::default(),
            settings: settings.clone(),
        });
        let router = ToolRouter::new(builtin);

        // Safe non-sensitive tool remains available with conservative defaults.
        let res = router
            .dispatch("get_current_time", &json!({}))
            .await
            .unwrap();
        assert!(res.get("time").is_some());

        // Privacy-sensitive tools fail closed on a fresh profile.
        let active_app = router
            .dispatch("get_active_application", &json!({}))
            .await
            .unwrap();
        assert!(active_app.get("error").is_some());
        assert!(router
            .dispatch(
                "remember_fact",
                &json!({ "fact": "User is building Talking Moose" }),
            )
            .await
            .is_err());
        assert!(mem.get_all_memories().unwrap().is_empty());

        // Explicit user opt-in enables memory writes.
        settings.write().memory_enabled = true;
        let rem_res = router
            .dispatch(
                "remember_fact",
                &json!({ "fact": "User is building Talking Moose" }),
            )
            .await
            .unwrap();
        assert_eq!(rem_res["status"], "remembered");
        assert_eq!(mem.get_all_memories().unwrap().len(), 1);

        // Test arbitrary tool rejection.
        let bad_tool = router
            .dispatch("execute_shell", &json!({ "cmd": "ls" }))
            .await;
        assert!(bad_tool.is_err());
    }
}
