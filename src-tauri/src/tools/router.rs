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
    use std::io::{self, Write};
    use std::sync::Mutex as StdMutex;
    use tracing_subscriber::fmt::MakeWriter;

    #[derive(Clone, Default)]
    struct CapturedLogs(Arc<StdMutex<Vec<u8>>>);

    impl CapturedLogs {
        fn text(&self) -> String {
            String::from_utf8(self.0.lock().unwrap().clone()).unwrap()
        }
    }

    impl Write for CapturedLogs {
        fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
            self.0.lock().unwrap().extend_from_slice(buf);
            Ok(buf.len())
        }

        fn flush(&mut self) -> io::Result<()> {
            Ok(())
        }
    }

    impl<'a> MakeWriter<'a> for CapturedLogs {
        type Writer = Self;

        fn make_writer(&'a self) -> Self::Writer {
            self.clone()
        }
    }

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

        // Permission changes are read from the shared settings object at dispatch
        // time. Turning the permission back off must take effect immediately.
        settings.write().active_app_observation = true;
        settings.write().active_app_observation = false;
        let denied_again = router
            .dispatch("get_active_application", &json!({}))
            .await
            .unwrap();
        assert!(denied_again.get("error").is_some());
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

    #[tokio::test(flavor = "current_thread")]
    async fn private_tool_payloads_and_observer_results_never_enter_normal_logs() {
        const TRANSCRIPT_SENTINEL: &str = "PRIVATE_TRANSCRIPT_SENTINEL_7c2a";
        const PROMPT_SENTINEL: &str = "PRIVATE_SYSTEM_PROMPT_SENTINEL_91bd";
        const SECRET_SENTINEL: &str = "AIzaSyPRIVATE_SECRET_SENTINEL_d41f";
        const MEMORY_SENTINEL: &str = "PRIVATE_MEMORY_FACT_SENTINEL_a331";
        const WINDOW_SENTINEL: &str = "PRIVATE_WINDOW_TITLE_SENTINEL_448e";

        let db = Arc::new(Database::new_in_memory().unwrap());
        let mem = Arc::new(crate::memory::MemoryManager::new(db));
        let settings = Arc::new(RwLock::new(AppSettings::default()));
        settings.write().memory_enabled = true;
        settings.write().active_app_observation = true;
        let router = ToolRouter::new(Arc::new(BuiltinTools {
            memory_manager: mem,
            character_config: CharacterConfig::default(),
            settings,
        }));

        let captured = CapturedLogs::default();
        let subscriber = tracing_subscriber::fmt()
            .without_time()
            .with_ansi(false)
            .with_writer(captured.clone())
            .finish();
        let _default = tracing::subscriber::set_default(subscriber);

        let private_payload = format!(
            "{TRANSCRIPT_SENTINEL} {PROMPT_SENTINEL} {SECRET_SENTINEL} {MEMORY_SENTINEL} {WINDOW_SENTINEL}"
        );
        router
            .dispatch("remember_fact", &json!({ "fact": private_payload }))
            .await
            .unwrap();

        let observer_result = router
            .dispatch("get_active_application", &json!({}))
            .await
            .unwrap();
        let observed_app = observer_result
            .get("active_application")
            .and_then(serde_json::Value::as_str)
            .unwrap_or("");

        // Unknown-tool arguments are model-controlled too. Their payload must not be
        // echoed when the router rejects the request.
        let _ = router
            .dispatch(
                "unregistered_private_probe",
                &json!({ "window_title": WINDOW_SENTINEL, "secret": SECRET_SENTINEL }),
            )
            .await;

        let logs = captured.text();
        for sentinel in [
            TRANSCRIPT_SENTINEL,
            PROMPT_SENTINEL,
            SECRET_SENTINEL,
            MEMORY_SENTINEL,
            WINDOW_SENTINEL,
        ] {
            assert!(
                !logs.contains(sentinel),
                "private value leaked into log output"
            );
        }
        if !observed_app.is_empty() && observed_app != "Unknown" {
            assert!(
                !logs.contains(observed_app),
                "active-application observation leaked into log output"
            );
        }
        assert!(logs.contains("remember_fact"));
        assert!(logs.contains("get_active_application"));
        assert!(logs.contains("unregistered_private_probe"));
    }
}
