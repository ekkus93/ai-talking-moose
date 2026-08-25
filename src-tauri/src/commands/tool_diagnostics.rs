use crate::app::state::AppState;
use crate::tools::policy::ToolAuditRecord;
use tauri::State;

fn tool_audit_snapshot(state: &AppState) -> Vec<ToolAuditRecord> {
    state.tool_router.audit_snapshot()
}

#[tauri::command]
pub fn get_tool_audit(state: State<'_, AppState>) -> Vec<ToolAuditRecord> {
    tool_audit_snapshot(state.inner())
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[tokio::test]
    async fn diagnostics_expose_metadata_only_audit_records_with_duration() {
        let state = AppState::new_for_tests().unwrap();
        let sensitive_arguments = json!({"secret": "must-not-appear"});

        let _ = state
            .tool_router
            .dispatch("definitely_not_registered", &sensitive_arguments)
            .await;

        let audit = tool_audit_snapshot(&state);
        let record = audit.last().expect("tool dispatch should be audited");
        assert_eq!(record.tool_name, "unregistered");

        let encoded = serde_json::to_value(record).unwrap();
        assert!(encoded.get("duration_ms").is_some());
        assert!(encoded.get("arguments").is_none());
        assert!(encoded.get("result").is_none());
        assert!(!encoded.to_string().contains("must-not-appear"));
    }
}
