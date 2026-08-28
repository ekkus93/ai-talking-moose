use super::*;
use crate::app::state::AppSettings;
use crate::character::personality::CharacterConfig;
use crate::persistence::sqlite::Database;
use crate::test_support::{assert_log_capture_live, capture_logs};
use crate::tools::builtin::V1_TOOL_NAMES;
use parking_lot::RwLock;
use serde_json::json;

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
async fn concurrent_execution_limit_fails_closed_and_recovers() {
    let (router, _) = router();
    let mut permits = Vec::new();
    for _ in 0..MAX_CONCURRENT_TOOL_EXECUTIONS {
        permits.push(router.execution_slots.clone().try_acquire_owned().unwrap());
    }

    let error = router
        .dispatch("get_current_time", &json!({}))
        .await
        .unwrap_err();
    assert_eq!(error.kind, ToolErrorKind::ConcurrencyLimit);
    assert_eq!(
        router.audit_snapshot().last().unwrap().result_category,
        ToolResultCategory::ConcurrencyLimit
    );

    let _ = permits.pop();
    assert!(router
        .dispatch("get_current_time", &json!({}))
        .await
        .is_ok());
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
fn hypothetical_character_action_policy_requires_explicit_local_confirmation() {
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

#[test]
fn private_tool_payloads_and_unknown_names_never_enter_normal_logs() {
    const TRANSCRIPT_SENTINEL: &str = "PRIVATE_TRANSCRIPT_SENTINEL_7c2a";
    const PROMPT_SENTINEL: &str = "PRIVATE_SYSTEM_PROMPT_SENTINEL_91bd";
    const SECRET_SENTINEL: &str = "AIzaSyPRIVATE_SECRET_SENTINEL_d41f";
    const MEMORY_SENTINEL: &str = "PRIVATE_MEMORY_FACT_SENTINEL_a331";
    const WINDOW_SENTINEL: &str = "PRIVATE_WINDOW_TITLE_SENTINEL_448e";
    const ACTIVE_APP_SENTINEL: &str = "PRIVATE_ACTIVE_APP_SENTINEL_a885";
    const RAW_AUDIO_SENTINEL: &str = "AAECAwQFBgcICQoLDA0ODw_PRIVATE_AUDIO_5ef0";

    let ((remembered_audit, unknown_audit), logs) = capture_logs(|| {
        let runtime = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .unwrap();
        runtime.block_on(async {
            let (router, settings) = router();
            settings.write().memory_enabled = true;

            let private_payload = format!(
                "{TRANSCRIPT_SENTINEL} {PROMPT_SENTINEL} {SECRET_SENTINEL} {MEMORY_SENTINEL} {WINDOW_SENTINEL} {ACTIVE_APP_SENTINEL} {RAW_AUDIO_SENTINEL}"
            );
            router
                .dispatch("remember_fact", &json!({ "fact": private_payload }))
                .await
                .unwrap();
            let remembered_audit = router.audit_snapshot().last().cloned().unwrap();

            let private_unknown_name = format!("unregistered_{SECRET_SENTINEL}");
            let error = router
                .dispatch(
                    &private_unknown_name,
                    &json!({
                        "window_title": WINDOW_SENTINEL,
                        "active_app": ACTIVE_APP_SENTINEL,
                        "raw_audio_base64": RAW_AUDIO_SENTINEL,
                        "secret": SECRET_SENTINEL
                    }),
                )
                .await
                .unwrap_err();
            assert_eq!(error.kind, ToolErrorKind::NotFound);

            let unknown_audit = router.audit_snapshot().last().cloned().unwrap();
            (remembered_audit, unknown_audit)
        })
    });

    assert_log_capture_live(&logs);
    for sentinel in [
        TRANSCRIPT_SENTINEL,
        PROMPT_SENTINEL,
        SECRET_SENTINEL,
        MEMORY_SENTINEL,
        WINDOW_SENTINEL,
        ACTIVE_APP_SENTINEL,
        RAW_AUDIO_SENTINEL,
    ] {
        assert!(
            !logs.contains(sentinel),
            "private value leaked into log output"
        );
    }
    // Positive routing assertions stay on the structured audit rather than formatted
    // tracing text. The shared capture harness separately proves log-capture
    // liveness before these privacy assertions run.
    assert_eq!(remembered_audit.tool_name, "remember_fact");
    assert_eq!(
        remembered_audit.result_category,
        ToolResultCategory::Success
    );
    assert_eq!(unknown_audit.tool_name, UNREGISTERED_AUDIT_NAME);
    assert_eq!(unknown_audit.result_category, ToolResultCategory::NotFound);
}
