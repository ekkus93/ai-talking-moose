pub struct GoogleLiveSession {
    sender: mpsc::Sender<OutboundEnvelope>,
    close_sender: mpsc::Sender<CloseRequest>,
    is_active: Arc<AtomicBool>,
    cancellation: CancellationToken,
    diagnostics: Arc<LiveOutboundDiagnosticsStore>,
    sample_rate_in: u32,
}

impl GoogleLiveSession {
    /// `Ok(())` means the message was accepted into this bounded local outbound queue. It does
    /// not mean the WebSocket write completed, nor that Gemini acknowledged or processed it.
    async fn enqueue(&self, envelope: OutboundEnvelope) -> Result<(), ProviderError> {
        if !self.is_active.load(Ordering::SeqCst) || self.cancellation.is_cancelled() {
            return Err(ProviderError::from_kind(ProviderErrorKind::Closed));
        }
        self.sender
            .send(envelope)
            .await
            .map_err(|_| ProviderError::from_kind(ProviderErrorKind::Closed))?;
        self.diagnostics.record(LiveOutboundDeliveryState::Queued);
        Ok(())
    }
}

#[async_trait]
impl LiveSession for GoogleLiveSession {
    fn outbound_diagnostics(&self) -> Option<LiveOutboundDiagnostics> {
        Some(self.diagnostics.snapshot())
    }

    async fn send_audio_chunk(&mut self, pcm_bytes: &[u8]) -> Result<(), ProviderError> {
        let message = encode_client_message(&audio_message(pcm_bytes, self.sample_rate_in))?;
        self.enqueue(OutboundEnvelope::new(message)).await
    }

    async fn send_text_turn(&mut self, text: &str) -> Result<(), ProviderError> {
        let message = encode_client_message(&text_turn_message(text))?;
        self.enqueue(OutboundEnvelope::new(message)).await
    }

    async fn send_tool_response(
        &mut self,
        response: ToolCallResponse,
    ) -> Result<(), ProviderError> {
        let message = encode_client_message(&tool_response_message(response))?;
        self.enqueue(OutboundEnvelope::new(message)).await
    }

    async fn interrupt(&mut self) -> Result<(), ProviderError> {
        info!("Barge-in requested on Gemini Live session; server VAD owns interruption");
        Ok(())
    }

    async fn close(&mut self) -> Result<(), ProviderError> {
        if !self.is_active.load(Ordering::SeqCst) {
            self.cancellation.cancel();
            return Ok(());
        }

        let (completion, completed) = oneshot::channel();
        let request = CloseRequest { completion };
        let request_result =
            tokio::time::timeout(CLOSE_REQUEST_TIMEOUT, self.close_sender.send(request)).await;
        match request_result {
            Ok(Ok(())) => {}
            Ok(Err(_)) => {
                self.is_active.store(false, Ordering::SeqCst);
                self.cancellation.cancel();
                return Err(ProviderError::from_kind(ProviderErrorKind::Closed));
            }
            Err(_) => {
                self.is_active.store(false, Ordering::SeqCst);
                self.cancellation.cancel();
                return Err(ProviderError::from_kind(ProviderErrorKind::Network));
            }
        }

        // Interrupt a blocked write or reconnect only after the supervisor owns the
        // protocol-close request. The supervisor prioritizes that request and writes
        // the Close frame with its own bounded timeout before acknowledging it.
        self.cancellation.cancel();
        let result = match tokio::time::timeout(CLOSE_COMPLETION_TIMEOUT, completed).await {
            Ok(Ok(result)) => result,
            Ok(Err(_)) => Err(ProviderError::from_kind(ProviderErrorKind::Closed)),
            Err(_) => Err(ProviderError::from_kind(ProviderErrorKind::Network)),
        };
        self.is_active.store(false, Ordering::SeqCst);
        result
    }
}

impl Drop for GoogleLiveSession {
    fn drop(&mut self) {
        self.is_active.store(false, Ordering::SeqCst);
        self.cancellation.cancel();
    }
}

#[async_trait]
impl RealtimeConversationProvider for GoogleLiveProvider {
    async fn connect(
        &self,
        config: LiveSessionConfig,
        event_sender: mpsc::Sender<LiveServerEvent>,
    ) -> Result<Box<dyn LiveSession>, ProviderError> {
        if !self.auth.is_valid() {
            return Err(ProviderError::from_kind(ProviderErrorKind::Auth));
        }
        validate_live_model(&config.model)
            .map_err(|_| ProviderError::from_kind(ProviderErrorKind::Model))?;

        let socket = open_and_setup(&self.auth.api_key, &config, None, None).await?;
        let (out_tx, out_rx) = mpsc::channel::<OutboundEnvelope>(OUTBOUND_QUEUE_CAPACITY);
        let (close_tx, close_rx) = mpsc::channel::<CloseRequest>(1);
        let is_active = Arc::new(AtomicBool::new(true));
        let supervisor_active = is_active.clone();
        let supervisor_sender = event_sender.clone();
        let supervisor_key = self.auth.api_key.clone();
        let supervisor_config = config.clone();
        let cancellation = CancellationToken::new();
        let supervisor_cancellation = cancellation.clone();
        let diagnostics = Arc::new(LiveOutboundDiagnosticsStore::default());
        let supervisor_diagnostics = diagnostics.clone();
        tauri::async_runtime::spawn(async move {
            supervise_socket(
                socket,
                supervisor_key,
                supervisor_config,
                out_rx,
                close_rx,
                supervisor_sender,
                supervisor_active,
                supervisor_cancellation,
                supervisor_diagnostics,
            )
            .await;
        });
        let _ = event_sender.send(LiveServerEvent::Connected).await;

        Ok(Box::new(GoogleLiveSession {
            sender: out_tx,
            close_sender: close_tx,
            is_active,
            cancellation,
            diagnostics,
            sample_rate_in: config.sample_rate_in,
        }))
    }
}
