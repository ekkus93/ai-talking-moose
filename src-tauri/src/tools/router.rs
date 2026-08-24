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
use tokio::sync::Semaphore;
use tracing::{info, warn};

const HARD_MAX_TOOL_TIMEOUT_MS: u64 = 5_000;
const HARD_MAX_TOOL_INPUT_BYTES: usize = 16 * 1024;
const HARD_MAX_TOOL_OUTPUT_BYTES: usize = 16 * 1024;
const MAX_CONCURRENT_TOOL_EXECUTIONS: usize = 4;
const MAX_TOOL_AUDIT_RECORDS: usize = 128;
const UNREGISTERED_AUDIT_NAME: &str = "unregistered";

#[derive(Debug, Clone, Copy, Default)]
pub struct ToolInvocationContext {
    pub user_confirmed: bool,
}

pub struct ToolRouter {
    builtin: Arc<BuiltinTools>,
    declarations: Vec<ToolDeclaration>,
    execution_slots: Arc<Semaphore>,
    audit: Mutex<VecDeque<ToolAuditRecord>>,
}

impl ToolRouter {
    pub fn new(builtin: Arc<BuiltinTools>) -> Self {
        let declarations = builtin.get_declarations();
        Self {
            builtin,
            declarations,
            execution_slots: Arc::new(Semaphore::new(MAX_CONCURRENT_TOOL_EXECUTIONS)),
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

        let _execution_permit = match self.execution_slots.clone().try_acquire_owned() {
            Ok(permit) => permit,
            Err(_) => {
                let result = Err(ToolError::from_kind(ToolErrorKind::ConcurrencyLimit));
                self.finish_registered(declaration, permission_outcome, started, &result);
                return result;
            }
        };

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
#[path = "router/tests.rs"]
mod tests;
