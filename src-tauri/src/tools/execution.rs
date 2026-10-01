use crate::tools::policy::{ToolError, ToolErrorKind};
use serde_json::Value;
use std::future::Future;
use std::time::Duration;

/// Internal execution outcome kept separate from the public/sanitized ToolError surface.
///
/// Body failures and worker panics intentionally collapse to the same user-visible
/// `ExecutionFailed` error, but remain distinct here for deterministic tests and future
/// observability that must never include private tool payloads.
#[derive(Debug)]
pub(crate) enum ToolExecutionOutcome {
    Success(Value),
    ToolError,
    WorkerPanic,
    WorkerCancelled,
    Timeout,
}

impl ToolExecutionOutcome {
    pub(crate) fn into_sanitized_result(self) -> Result<Value, ToolError> {
        match self {
            Self::Success(value) => Ok(value),
            Self::Timeout => Err(ToolError::from_kind(ToolErrorKind::Timeout)),
            Self::ToolError | Self::WorkerPanic | Self::WorkerCancelled => {
                Err(ToolError::from_kind(ToolErrorKind::ExecutionFailed))
            }
        }
    }
}

pub(crate) async fn run_blocking_with_timeout<F>(
    timeout: Duration,
    operation: F,
) -> ToolExecutionOutcome
where
    F: FnOnce() -> Result<Value, String> + Send + 'static,
{
    let worker = tokio::task::spawn_blocking(operation);
    match tokio::time::timeout(timeout, worker).await {
        Ok(Ok(Ok(value))) => ToolExecutionOutcome::Success(value),
        Ok(Ok(Err(_))) => ToolExecutionOutcome::ToolError,
        Ok(Err(error)) if error.is_panic() => ToolExecutionOutcome::WorkerPanic,
        Ok(Err(_)) => ToolExecutionOutcome::WorkerCancelled,
        Err(_) => ToolExecutionOutcome::Timeout,
    }
}

/// Equivalent bounded adapter for genuinely async built-ins.
///
/// V1 currently classifies its registered built-ins as blocking, but keeping the async adapter
/// beside the blocking adapter makes the timeout contract explicit and regression-testable
/// without pushing authorization or policy logic into worker orchestration.
#[allow(dead_code)]
pub(crate) async fn run_async_with_timeout<F>(
    timeout: Duration,
    operation: F,
) -> ToolExecutionOutcome
where
    F: Future<Output = Result<Value, String>> + Send + 'static,
{
    let worker = tokio::spawn(operation);
    match tokio::time::timeout(timeout, worker).await {
        Ok(Ok(Ok(value))) => ToolExecutionOutcome::Success(value),
        Ok(Ok(Err(_))) => ToolExecutionOutcome::ToolError,
        Ok(Err(error)) if error.is_panic() => ToolExecutionOutcome::WorkerPanic,
        Ok(Err(_)) => ToolExecutionOutcome::WorkerCancelled,
        Err(_) => ToolExecutionOutcome::Timeout,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;
    use std::sync::atomic::{AtomicBool, Ordering};
    use std::sync::Arc;
    use std::time::Instant;

    #[tokio::test]
    async fn blocking_timeout_returns_before_body_finishes_and_ignores_late_result() {
        let finished = Arc::new(AtomicBool::new(false));
        let worker_finished = finished.clone();
        let started = Instant::now();
        let outcome = run_blocking_with_timeout(Duration::from_millis(10), move || {
            std::thread::sleep(Duration::from_millis(100));
            worker_finished.store(true, Ordering::SeqCst);
            Ok(json!({ "late": true }))
        })
        .await;

        assert!(matches!(&outcome, ToolExecutionOutcome::Timeout));
        assert_eq!(
            outcome.into_sanitized_result().unwrap_err().kind,
            ToolErrorKind::Timeout
        );
        assert!(started.elapsed() < Duration::from_millis(80));
        assert!(!finished.load(Ordering::SeqCst));

        tokio::time::sleep(Duration::from_millis(120)).await;
        assert!(finished.load(Ordering::SeqCst));
    }

    #[tokio::test]
    async fn blocking_body_error_is_distinct_from_worker_panic_internally() {
        let body_error =
            run_blocking_with_timeout(Duration::from_secs(1), || Err("private body error".into()))
                .await;
        assert!(matches!(&body_error, ToolExecutionOutcome::ToolError));
        assert_eq!(
            body_error.into_sanitized_result().unwrap_err().kind,
            ToolErrorKind::ExecutionFailed
        );

        let panic = run_blocking_with_timeout(Duration::from_secs(1), || {
            panic!("private worker panic payload must not escape");
            #[allow(unreachable_code)]
            Ok(json!({}))
        })
        .await;
        assert!(matches!(&panic, ToolExecutionOutcome::WorkerPanic));
        assert_eq!(
            panic.into_sanitized_result().unwrap_err().kind,
            ToolErrorKind::ExecutionFailed
        );
    }

    #[tokio::test]
    async fn successful_blocking_worker_returns_normally() {
        let outcome =
            run_blocking_with_timeout(Duration::from_secs(1), || Ok(json!({ "ok": true }))).await;
        let value = outcome.into_sanitized_result().unwrap();
        assert_eq!(value["ok"], true);
    }

    #[tokio::test]
    async fn pending_async_worker_obeys_the_same_timeout_contract() {
        let outcome = run_async_with_timeout(Duration::from_millis(10), async {
            std::future::pending::<Result<Value, String>>().await
        })
        .await;
        assert!(matches!(&outcome, ToolExecutionOutcome::Timeout));
        assert_eq!(
            outcome.into_sanitized_result().unwrap_err().kind,
            ToolErrorKind::Timeout
        );
    }
}
