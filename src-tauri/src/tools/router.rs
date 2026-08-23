use crate::tools::builtin::BuiltinTools;
use crate::tools::policy::{
    ToolAuditRecord, ToolConfirmationPolicy, ToolDeclaration, ToolError, ToolErrorKind,
    ToolPermissionLevel, ToolPermissionOutcome, ToolResultCategory,
};
use chrono::{SecondsFormat, Utc};
use parking_lot::Mutex;
use serde_json::Value;
use std::collections::VecDeque;
use std::future::Future;
use std::sync::Arc;
use std::time::{Duration, Instant};
use tracing::{info, warn};

const HARD_MAX_TOOL_TIMEOUT_MS: u64 = 5_000;
const HARD_MAX_TOOL_INPUT_BYTES: usize = 16 * 1024;
const HARD_MAX_TOOL_OUTPUT_BYTES: usize = 16 * 1024;
const MAX_TOOL_AUDIT_RECORDS: usize = 128;
const UNREGISTERED_AUDIT_NAME: &str = "unregistered";

#[derive(Debug, Clone, Copy, Default)]
pub struct ToolInvocationContext {
    pub user_confirmed: bool,
}

pub struct ToolRouter {
    builtin: Arc<BuiltinTools>,
    declarations: Vec<ToolDeclaration>,
    audit: Mutex<VecDeque<ToolAuditRecord>>,
}

impl ToolRouter {
    pub fn new(builtin: Arc<BuiltinTools>) -> Self {
        let declarations = builtin.get_declarations();
        Self {
            builtin,
            declarations,
            audit: Mutex::new(VecDeque::with_capacity(MAX_TOOL_AUDIT_RECORDS)),
        }
    }

    pub fn get_declarations(&self) -> Vec<ToolDeclaration> {
        self.declarations.clone()
    }

    pub fn audit_snapshot(&self) -> Vec<ToolAuditRecord> {
        self.audit.lock().iter().cloned().collect()
    }

    pub async fn dispatch(&self, name: &str, arguments: &Value) -> Result<Value, ToolError> {
        self.dispatch_with_context(name, arguments, ToolInvocationContext::default())
            .await
    }

    pub async fn dispatch_with_context(
        &self,
        name: &str,
        arguments: &Value,
        context: ToolInvocationContext,
    ) -> Result<Value, ToolError> {
        let started = Instant::now();
        let Some(declaration) = self.declarations.iter().find(|tool| tool.name == name) else {
            let result = Err(ToolError::from_kind(ToolErrorKind::NotFound));
            self.record_audit(
                UNREGISTERED_AUDIT_NAME,
                ToolPermissionLevel::Denied,
                ToolPermissionOutcome::Denied,
                started,
                &result,
            );
            let category = ToolResultCategory::NotFound;
            warn!(
                tool = UNREGISTERED_AUDIT_NAME,
                result = ?category,
                "Tool call rejected"
            );
            return result;
        };

        let input_limit = declaration
            .execution
            .max_input_bytes
            .min(HARD_MAX_TOOL_INPUT_BYTES);
        if serialized_size(arguments) > input_limit {
            let result = Err(ToolError::from_kind(ToolErrorKind::InputTooLarge));
            self.finish_registered(
                declaration,
                ToolPermissionOutcome::NotEvaluated,
                started,
                &result,
            );
            return result;
        }

        let permission_outcome = self.permission_outcome(declaration, context);
        match permission_outcome {
            ToolPermissionOutcome::Denied => {
                let result = Err(ToolError::from_kind(ToolErrorKind::PermissionDenied));
                self.finish_registered(declaration, permission_outcome, started, &result);
                return result;
            }
            ToolPermissionOutcome::ConfirmationRequired => {
                let result = Err(ToolError::from_kind(ToolErrorKind::ConfirmationRequired));
                self.finish_registered(declaration, permission_outcome, started, &result);
                return result;
            }
            ToolPermissionOutcome::Allowed => {}
            ToolPermissionOutcome::NotEvaluated => unreachable!("permission policy must resolve"),
        }

        if let Err(error) = declaration.validate_arguments(arguments) {
            let result = Err(error);
            self.finish_registered(declaration, permission_outcome, started, &result);
            return result;
        }

        let timeout_ms = declaration
            .execution
            .timeout_ms
            .clamp(1, HARD_MAX_TOOL_TIMEOUT_MS);
        let execution = run_with_timeout(
            Duration::from_millis(timeout_ms),
            self.builtin.execute(&declaration.name, arguments),
        )
        .await;

        let result = match execution {
            Ok(output) => {
                let output_limit = declaration
                    .execution
                    .max_output_bytes
                    .min(HARD_MAX_TOOL_OUTPUT_BYTES);
                enforce_output_size(output, output_limit)
            }
            Err(error) => Err(error),
        };
        self.finish_registered(declaration, permission_outcome, started, &result);
        result
    }

    fn permission_outcome(
        &self,
        declaration: &ToolDeclaration,
        context: ToolInvocationContext,
    ) -> ToolPermissionOutcome {
        use crate::tools::policy::ToolPrivacyGate;

        if !self.builtin.privacy_gate_allowed(declaration.privacy_gate) {
            return ToolPermissionOutcome::Denied;
        }

        match declaration.permission {
            ToolPermissionLevel::Denied => ToolPermissionOutcome::Denied,
            ToolPermissionLevel::CharacterAction => {
                if declaration.confirmation != ToolConfirmationPolicy::PerInvocation {
                    ToolPermissionOutcome::Denied
                } else if context.user_confirmed {
                    ToolPermissionOutcome::Allowed
                } else {
                    ToolPermissionOutcome::ConfirmationRequired
                }
            }
            ToolPermissionLevel::MemoryMutation => {
                if declaration.privacy_gate != ToolPrivacyGate::Memory {
                    return ToolPermissionOutcome::Denied;
                }
                match declaration.confirmation {
                    ToolConfirmationPolicy::SettingOptIn => ToolPermissionOutcome::Allowed,
                    ToolConfirmationPolicy::PerInvocation if context.user_confirmed => {
                        ToolPermissionOutcome::Allowed
                    }
                    ToolConfirmationPolicy::PerInvocation => {
                        ToolPermissionOutcome::ConfirmationRequired
                    }
                    ToolConfirmationPolicy::None => ToolPermissionOutcome::Denied,
                }
            }
            ToolPermissionLevel::SafeReadOnly => match declaration.confirmation {
                ToolConfirmationPolicy::None => ToolPermissionOutcome::Allowed,
                ToolConfirmationPolicy::SettingOptIn
                    if declaration.privacy_gate != ToolPrivacyGate::None =>
                {
                    ToolPermissionOutcome::Allowed
                }
                ToolConfirmationPolicy::SettingOptIn => ToolPermissionOutcome::Denied,
                ToolConfirmationPolicy::PerInvocation if context.user_confirmed => {
                    ToolPermissionOutcome::Allowed
                }
                ToolConfirmationPolicy::PerInvocation => {
                    ToolPermissionOutcome::ConfirmationRequired
                }
            },
        }
    }

    fn finish_registered(
        &self,
        declaration: &ToolDeclaration,
        permission_outcome: ToolPermissionOutcome,
        started: Instant,
        result: &Result<Value, ToolError>,
    ) {
        let category = ToolResultCategory::from_result(result);
        self.record_audit(
            &declaration.name,
            declaration.permission,
            permission_outcome,
            started,
            result,
        );
        match result {
            Ok(_) => info!(
                tool = declaration.name.as_str(),
                result = ?category,
                permission = ?permission_outcome,
                "Tool call completed"
            ),
            Err(_) => warn!(
                tool = declaration.name.as_str(),
                result = ?category,
                permission = ?permission_outcome,
                "Tool call rejected or failed"
            ),
        }
    }

    fn record_audit(
        &self,
        tool_name: &str,
        permission: ToolPermissionLevel,
        permission_outcome: ToolPermissionOutcome,
        started: Instant,
        result: &Result<Value, ToolError>,
    ) {
        let duration_ms = u64::try_from(started.elapsed().as_millis()).unwrap_or(u64::MAX);
        let record = ToolAuditRecord {
            tool_name: tool_name.to_string(),
            timestamp: Utc::now().to_rfc3339_opts(SecondsFormat::Millis, true),
            duration_ms,
            permission,
            permission_outcome,
            result_category: ToolResultCategory::from_result(result),
        };
        let mut audit = self.audit.lock();
        if audit.len() == MAX_TOOL_AUDIT_RECORDS {
            audit.pop_front();
        }
        audit.push_back(record);
    }
}

fn serialized_size(value: &Value) -> usize {
    serde_json::to_vec(value).map_or(usize::MAX, |encoded| encoded.len())
}

fn enforce_output_size(output: Value, limit: usize) -> Result<Value, ToolError> {
    if serialized_size(&output) > limit.min(HARD_MAX_TOOL_OUTPUT_BYTES) {
        Err(ToolError::from_kind(ToolErrorKind::OutputTooLarge))
    } else {
        Ok(output)
    }
}

async fn run_with_timeout<F>(timeout: Duration, future: F) -> Result<Value, ToolError>
where
    F: Future<Output = Result<Value, String>>,
{
    match tokio::time::timeout(timeout, future).await {
        Ok(Ok(value)) => Ok(value),
        Ok(Err(_)) => Err(ToolError::from_kind(ToolErrorKind::ExecutionFailed)),
        Err(_) => Err(ToolError::from_kind(ToolErrorKind::Timeout)),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::app::state::AppSettings;
    use crate::character::personality::CharacterConfig;
    use crate::persistence::sqlite::Database;
    use crate::tools::builtin::V1_TOOL_NAMES;
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

    fn router() -> (ToolRouter, Arc<RwLock<AppSettings>>) {
        let db = Arc::new(Database::new_in_memory().unwrap());
        let memory = Arc::new(crate::memory::MemoryManager::new(db));
        let settings = Arc::new(RwLock::new(AppSettings::default()));
        let builtin = Arc::new(BuiltinTools {
            memory_manager: memory,
            character_config: CharacterConfig::default(),
            settings: settings.clone(),
        });
        (ToolRouter::new(builtin), settings)
    }

    #[tokio::test]
    async fn router_enforces_privacy_schema_and_memory_opt_in() {
        let (router, settings) = router();

        assert!(router
            .dispatch("get_current_time", &json!({}))
            .await
            .is_ok());

        let denied = router
            .dispatch("get_active_application", &json!({}))
            .await
            .unwrap_err();
        assert_eq!(denied.kind, ToolErrorKind::PermissionDenied);

        let invalid = router
            .dispatch("get_current_time", &json!({ "extra": true }))
            .await
            .unwrap_err();
        assert_eq!(invalid.kind, ToolErrorKind::InvalidArguments);

        let memory_denied = router
            .dispatch("remember_fact", &json!({ "fact": "User likes coffee" }))
            .await
            .unwrap_err();
        assert_eq!(memory_denied.kind, ToolErrorKind::PermissionDenied);

        settings.write().memory_enabled = true;
        let remembered = router
            .dispatch("remember_fact", &json!({ "fact": "User likes coffee" }))
            .await
            .unwrap();
        assert_eq!(remembered["status"], "remembered");
    }

    #[tokio::test]
    async fn hard_input_limit_and_unknown_tools_fail_closed() {
        let (router, settings) = router();
        settings.write().memory_enabled = true;
        let oversized = "x".repeat(HARD_MAX_TOOL_INPUT_BYTES + 1);
        let error = router
            .dispatch("remember_fact", &json!({ "fact": oversized }))
            .await
            .unwrap_err();
        assert_eq!(error.kind, ToolErrorKind::InputTooLarge);

        for prohibited in [
            "execute_shell",
            "run_applescript",
            "read_file",
            "http_request",
            "spawn_process",
        ] {
            let error = router.dispatch(prohibited, &json!({})).await.unwrap_err();
            assert_eq!(error.kind, ToolErrorKind::NotFound);
        }
    }

    #[tokio::test]
    async fn timeout_wrapper_returns_structured_timeout() {
        let error = run_with_timeout(Duration::from_millis(1), async {
            std::future::pending::<Result<Value, String>>().await
        })
        .await
        .unwrap_err();
        assert_eq!(error.kind, ToolErrorKind::Timeout);
    }

    #[test]
    fn output_size_guard_rejects_oversized_results() {
        let oversized = json!({ "value": "x".repeat(256) });
        let error = enforce_output_size(oversized, 32).unwrap_err();
        assert_eq!(error.kind, ToolErrorKind::OutputTooLarge);
        assert!(enforce_output_size(json!({ "ok": true }), 32).is_ok());
    }

    #[test]
    fn character_actions_require_explicit_local_confirmation() {
        let (router, _) = router();
        let mut declaration = router.get_declarations()[0].clone();
        declaration.permission = ToolPermissionLevel::CharacterAction;
        declaration.confirmation = ToolConfirmationPolicy::PerInvocation;

        assert_eq!(
            router.permission_outcome(&declaration, ToolInvocationContext::default()),
            ToolPermissionOutcome::ConfirmationRequired
        );
        assert_eq!(
            router.permission_outcome(
                &declaration,
                ToolInvocationContext {
                    user_confirmed: true,
                },
            ),
            ToolPermissionOutcome::Allowed
        );
    }

    #[test]
    fn mutation_and_action_declarations_cannot_weaken_required_guards() {
        let (router, settings) = router();
        settings.write().memory_enabled = true;

        let mut memory = router
            .get_declarations()
            .into_iter()
            .find(|tool| tool.name == "remember_fact")
            .unwrap();
        memory.privacy_gate = crate::tools::policy::ToolPrivacyGate::None;
        assert_eq!(
            router.permission_outcome(&memory, ToolInvocationContext::default()),
            ToolPermissionOutcome::Denied
        );
        memory.privacy_gate = crate::tools::policy::ToolPrivacyGate::Memory;
        memory.confirmation = ToolConfirmationPolicy::None;
        assert_eq!(
            router.permission_outcome(&memory, ToolInvocationContext::default()),
            ToolPermissionOutcome::Denied
        );

        let mut action = router.get_declarations()[0].clone();
        action.permission = ToolPermissionLevel::CharacterAction;
        action.confirmation = ToolConfirmationPolicy::None;
        assert_eq!(
            router.permission_outcome(
                &action,
                ToolInvocationContext {
                    user_confirmed: true,
                },
            ),
            ToolPermissionOutcome::Denied
        );
    }

    #[tokio::test]
    async fn audit_is_bounded_and_contains_no_raw_arguments_or_results() {
        const PRIVATE_VALUE: &str = "PRIVATE_TOOL_ARGUMENT_SENTINEL_2d91";
        let (router, settings) = router();
        settings.write().memory_enabled = true;

        router
            .dispatch("remember_fact", &json!({ "fact": PRIVATE_VALUE }))
            .await
            .unwrap();
        let private_audit = router.audit_snapshot();
        let encoded = serde_json::to_string(&private_audit).unwrap();
        assert!(!encoded.contains(PRIVATE_VALUE));

        for _ in 0..MAX_TOOL_AUDIT_RECORDS + 4 {
            router
                .dispatch("get_current_time", &json!({}))
                .await
                .unwrap();
        }
        let audit = router.audit_snapshot();
        assert_eq!(audit.len(), MAX_TOOL_AUDIT_RECORDS);
        assert!(audit
            .iter()
            .all(|record| V1_TOOL_NAMES.contains(&record.tool_name.as_str())));
    }

    #[tokio::test(flavor = "current_thread")]
    async fn private_tool_payloads_and_unknown_names_never_enter_normal_logs() {
        const TRANSCRIPT_SENTINEL: &str = "PRIVATE_TRANSCRIPT_SENTINEL_7c2a";
        const PROMPT_SENTINEL: &str = "PRIVATE_SYSTEM_PROMPT_SENTINEL_91bd";
        const SECRET_SENTINEL: &str = "AIzaSyPRIVATE_SECRET_SENTINEL_d41f";
        const MEMORY_SENTINEL: &str = "PRIVATE_MEMORY_FACT_SENTINEL_a331";
        const WINDOW_SENTINEL: &str = "PRIVATE_WINDOW_TITLE_SENTINEL_448e";

        let (router, settings) = router();
        settings.write().memory_enabled = true;
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

        let private_unknown_name = format!("unregistered_{SECRET_SENTINEL}");
        let _ = router
            .dispatch(
                &private_unknown_name,
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
        assert!(logs.contains("remember_fact"));
        assert!(logs.contains(UNREGISTERED_AUDIT_NAME));
    }
}
