use super::*;

impl ConversationManager {
    fn provider_timeout_error() -> ProviderError {
        ProviderError {
            kind: ProviderErrorKind::Network,
            message: "The conversation provider did not respond before the operation timeout."
                .to_string(),
            retryable: true,
        }
    }

    pub(super) async fn bounded_provider_operation<T, F>(operation: F) -> Result<T, ProviderError>
    where
        F: Future<Output = Result<T, ProviderError>>,
    {
        tokio::time::timeout(PROVIDER_OPERATION_TIMEOUT, operation)
            .await
            .map_err(|_| Self::provider_timeout_error())?
    }

    pub(super) async fn close_provisional_session(session: &mut Box<dyn LiveSession>) {
        if let Err(error_value) = Self::bounded_provider_operation(session.close()).await {
            warn!(
                kind = ?error_value.kind,
                "Failed to close provisional conversation session"
            );
        }
    }

    pub(super) async fn close_detached_session(mut session: Option<Box<dyn LiveSession>>) {
        if let Some(ref mut session) = session {
            if let Err(error_value) = Self::bounded_provider_operation(session.close()).await {
                warn!(
                    kind = ?error_value.kind,
                    "Failed to close conversation session"
                );
            }
        }
    }

    pub(super) async fn forward_microphone_chunk(
        &self,
        expected_generation: u64,
        asr_mode: AsrMode,
        chunk: &[u8],
    ) -> Result<bool, ProviderError> {
        if asr_mode != AsrMode::GeminiLiveAudio {
            return Ok(false);
        }

        // The session mutex is the replacement boundary: start/stop cannot swap the LiveSession
        // between this final generation check and send acceptance. Provider I/O is bounded below,
        // so teardown can wait on this mutex only for a finite interval.
        let mut session_lock = self.live_session.lock().await;
        if !self.is_in_conversation.load(Ordering::SeqCst)
            || self.generation.load(Ordering::SeqCst) != expected_generation
            || *self.active_asr_mode.lock() != Some(AsrMode::GeminiLiveAudio)
        {
            return Ok(false);
        }
        let Some(live_session) = session_lock.as_mut() else {
            return Err(ProviderError::from_kind(ProviderErrorKind::Closed));
        };
        Self::bounded_provider_operation(live_session.send_audio_chunk(chunk)).await?;
        Ok(true)
    }

    pub(super) async fn send_tool_response_if_current(
        &self,
        expected_generation: u64,
        expected_session_id: &str,
        response: ToolCallResponse,
    ) -> Result<bool, ProviderError> {
        // As with microphone forwarding, the final identity check occurs while holding the slot
        // that start/stop must acquire to replace the provider session.
        let mut session_lock = self.live_session.lock().await;
        if !self.is_in_conversation.load(Ordering::SeqCst)
            || self.generation.load(Ordering::SeqCst) != expected_generation
            || self.active_session_id.lock().as_deref() != Some(expected_session_id)
        {
            return Ok(false);
        }
        let Some(live_session) = session_lock.as_mut() else {
            return Err(ProviderError::from_kind(ProviderErrorKind::Closed));
        };
        Self::bounded_provider_operation(live_session.send_tool_response(response)).await?;
        Ok(true)
    }

    pub(super) fn accept_user_transcript(
        &self,
        session_id: &str,
        text: String,
        state_callback: &StateCallback,
        lifecycle_callback: &LifecycleCallback,
        transcript_callback: &TranscriptCallback,
    ) {
        self.output_suppressed.store(false, Ordering::SeqCst);
        transcript_callback(session_id.to_string(), "user".to_string(), text);
        Self::set_lifecycle(
            &self.lifecycle,
            ConversationLifecycle::Responding,
            Some(lifecycle_callback),
        );
        state_callback(CharacterState::Thinking);
    }

    /// Route one provider-neutral local-ASR event into the active Gemini Live conversation.
    ///
    /// Only finalized local transcript text crosses the provider boundary. Partial transcript
    /// text and speech lifecycle events remain local. The generation and session-id checks make
    /// this handoff fail closed when a callback belongs to an older conversation.
    pub async fn handle_local_asr_event(
        &self,
        generation: u64,
        expected_session_id: &str,
        event: AsrEvent,
    ) -> Result<bool, ProviderError> {
        let AsrEvent::FinalTranscript { text } = event else {
            return Ok(false);
        };
        let text = text.trim().to_string();
        if text.is_empty() {
            return Ok(false);
        }

        if !self.is_in_conversation.load(Ordering::SeqCst)
            || self.generation.load(Ordering::SeqCst) != generation
            || !self.local_asr.accepts_callback(generation).await
            || self.active_session_id.lock().as_deref() != Some(expected_session_id)
            || !matches!(
                *self.active_asr_mode.lock(),
                Some(
                    AsrMode::MoonshineTinyStreaming
                        | AsrMode::MoonshineSmallStreaming
                        | AsrMode::WhisperSmall
                )
            )
        {
            return Ok(false);
        }

        let state_callback = self
            .state_callback
            .lock()
            .clone()
            .ok_or(ProviderError::from_kind(ProviderErrorKind::Internal))?;
        let lifecycle_callback = self
            .lifecycle_callback
            .lock()
            .clone()
            .ok_or(ProviderError::from_kind(ProviderErrorKind::Internal))?;
        let transcript_callback = self
            .transcript_callback
            .lock()
            .clone()
            .ok_or(ProviderError::from_kind(ProviderErrorKind::Internal))?;

        // Acquire the replaceable session slot, then revalidate identity immediately before the
        // provider call. A stale callback may keep running after Stop, but it can never acquire a
        // replacement generation and send into that newer session.
        let mut session_lock = self.live_session.lock().await;
        if !self.is_in_conversation.load(Ordering::SeqCst)
            || self.generation.load(Ordering::SeqCst) != generation
            || self.active_session_id.lock().as_deref() != Some(expected_session_id)
            || !matches!(
                *self.active_asr_mode.lock(),
                Some(
                    AsrMode::MoonshineTinyStreaming
                        | AsrMode::MoonshineSmallStreaming
                        | AsrMode::WhisperSmall
                )
            )
        {
            return Ok(false);
        }
        let Some(live_session) = session_lock.as_mut() else {
            return Err(ProviderError::from_kind(ProviderErrorKind::Closed));
        };

        // Accept the user turn before sending it so local and cloud transcript paths expose the
        // same Responding/Thinking transition and a fast provider reply cannot overtake it.
        self.accept_user_transcript(
            expected_session_id,
            text.clone(),
            &state_callback,
            &lifecycle_callback,
            &transcript_callback,
        );
        Self::bounded_provider_operation(live_session.send_text_turn(&text)).await?;
        Ok(true)
    }
}
