fn reconnect_delay(attempt: u32, jitter_ms: u64) -> Duration {
    let exponent = attempt.saturating_sub(1).min(6);
    let base = BASE_RECONNECT_DELAY_MS.saturating_mul(1_u64 << exponent);
    Duration::from_millis(base.min(MAX_RECONNECT_DELAY_MS) + jitter_ms.min(MAX_RECONNECT_JITTER_MS))
}

async fn await_reconnect_attempt<T, F>(
    attempt: F,
    cancellation: &CancellationToken,
) -> Result<T, ProviderError>
where
    F: Future<Output = Result<T, ProviderError>>,
{
    tokio::select! {
        result = attempt => result,
        () = cancellation.cancelled() => Err(ProviderError::from_kind(ProviderErrorKind::Closed)),
    }
}

async fn reconnect(
    api_key: &str,
    config: &LiveSessionConfig,
    resume_handle: Option<&str>,
    is_active: &AtomicBool,
    cancellation: &CancellationToken,
) -> Result<GeminiSocket, ProviderError> {
    let started = Instant::now();
    let mut last_error = ProviderError::from_kind(ProviderErrorKind::Network);
    for attempt in 1..=MAX_RECONNECT_ATTEMPTS {
        if !is_active.load(Ordering::SeqCst)
            || cancellation.is_cancelled()
            || started.elapsed() >= MAX_RECONNECT_ELAPSED
        {
            break;
        }
        let jitter = rand::thread_rng().gen_range(0..=MAX_RECONNECT_JITTER_MS);
        let delay = reconnect_delay(attempt, jitter);
        tokio::select! {
            () = tokio::time::sleep(delay) => {}
            () = cancellation.cancelled() => {
                return Err(ProviderError::from_kind(ProviderErrorKind::Closed));
            }
        }

        let reconnect_result = await_reconnect_attempt(
            open_and_setup(api_key, config, resume_handle, Some(cancellation)),
            cancellation,
        )
        .await;
        match reconnect_result {
            Ok(socket) => return Ok(socket),
            Err(error) => {
                if !is_active.load(Ordering::SeqCst) || cancellation.is_cancelled() {
                    return Err(error);
                }
                if !error.retryable {
                    return Err(error);
                }
                last_error = error;
            }
        }
    }
    Err(last_error)
}

fn retain_for_retry(
    pending_outbound: &mut Option<OutboundEnvelope>,
    envelope: OutboundEnvelope,
    diagnostics: &LiveOutboundDiagnosticsStore,
) {
    diagnostics.record(LiveOutboundDeliveryState::Retried);
    *pending_outbound = Some(envelope);
}

fn record_terminal_outbound_failures(
    pending_outbound: &mut Option<OutboundEnvelope>,
    receiver: &mut mpsc::Receiver<OutboundEnvelope>,
    diagnostics: &LiveOutboundDiagnosticsStore,
) -> usize {
    let mut failed = 0;
    if pending_outbound.take().is_some() {
        diagnostics.record(LiveOutboundDeliveryState::TerminallyFailed);
        diagnostics.record_drop();
        failed += 1;
    }
    while receiver.try_recv().is_ok() {
        diagnostics.record(LiveOutboundDeliveryState::TerminallyFailed);
        diagnostics.record_drop();
        failed += 1;
    }
    failed
}
