use super::*;

impl ConversationManager {
    pub async fn start_session(&self, request: ConversationStartRequest) -> Result<String, String> {
        self.start_session_with_wake_handoff(request, None).await
    }

    pub async fn start_session_with_wake_handoff(
        &self,
        request: ConversationStartRequest,
        wake_handoff_audio: Option<WakeCommandHandoffAudio>,
    ) -> Result<String, String> {
        let ConversationStartRequest {
            provider,
            config,
            asr_mode,
            moonshine_installer,
            whisper_installer,
            capture,
            input_device,
            playback,
            output_device,
            muted,
            tool_router,
            callbacks,
        } = request;
        let ConversationCallbacks {
            state: state_callback,
            lifecycle: lifecycle_callback,
            provider_error: provider_error_callback,
            transcript: transcript_callback,
            speech_bubble: speech_bubble_callback,
            input_level: input_level_callback,
        } = callbacks;

        // Invalidate and detach any prior session while serialized, but close its provider handle
        // only after releasing the global operation lock. If Stop races that close it increments
        // generation, and this start notices the invalidation before doing any new provider I/O.
        let operation_guard = self.operation_lock.lock().await;
        let previous_session = self
            .begin_shutdown_locked(
                capture.clone(),
                playback.clone(),
                ConversationLifecycle::Idle,
            )
            .await;
        let teardown_generation = self.generation.load(Ordering::SeqCst);
        drop(operation_guard);
        Self::close_detached_session(previous_session).await;

        let operation_guard = self.operation_lock.lock().await;
        let reservation =
            self.reserve_start_locked(teardown_generation, muted.as_ref(), &lifecycle_callback)?;
        let generation = reservation.generation;
        let session_id = reservation.session_id.clone();

        // Local ASR is prepared before opening the cloud Live session. The focused helper
        // fails closed before microphone capture or provider traffic if local prerequisites fail.
        // Reserve the generation while serialized, then release the lifecycle lock before
        // potentially expensive local-ASR model/worker preparation. Stop can now acquire the
        // lock immediately and invalidate this generation instead of waiting for ASR startup.
        drop(operation_guard);
        let preparation_result = self
            .prepare_local_asr(LocalAsrPreparation {
                generation,
                asr_mode,
                installer: moonshine_installer,
                whisper_installer,
                session_id: session_id.clone(),
                capture: capture.clone(),
                playback: playback.clone(),
                state_callback: state_callback.clone(),
                provider_error_callback: provider_error_callback.clone(),
            })
            .await;

        // Re-enter the lifecycle boundary immediately after preparation. A Stop that raced
        // local-ASR startup has already advanced generation; stale prepared state (including a
        // stale preparation failure) is disposed before it can mutate lifecycle/UI state.
        let operation_guard = self.operation_lock.lock().await;
        let mut local_pipeline = match preparation_result {
            Ok(pipeline) => pipeline,
            Err(error) if self.start_reservation_is_current(&reservation) => {
                Self::set_lifecycle(
                    &self.lifecycle,
                    ConversationLifecycle::Failed,
                    Some(&lifecycle_callback),
                );
                state_callback(CharacterState::Error);
                return Err(error);
            }
            Err(_) => {
                return Err("Conversation start was cancelled".to_string());
            }
        };
        if !self.start_reservation_is_current(&reservation) {
            drop(operation_guard);
            Self::dispose_cancelled_start(&mut local_pipeline, None).await;
            return Err("Conversation start was cancelled".to_string());
        }

        if let Some(handoff_audio) = wake_handoff_audio {
            let Some(pipeline) = local_pipeline.as_ref() else {
                let message = concat!(
                    "Wake Word handoff requires local Moonshine command ASR; ",
                    "no microphone audio was sent."
                )
                .to_string();
                Self::set_lifecycle(
                    &self.lifecycle,
                    ConversationLifecycle::Failed,
                    Some(&lifecycle_callback),
                );
                state_callback(CharacterState::Error);
                return Err(message);
            };
            if let Err(error) = pipeline.prime_wake_handoff(handoff_audio) {
                self.local_asr_diagnostics
                    .remember_error(asr_mode, error.clone());
                Self::set_lifecycle(
                    &self.lifecycle,
                    ConversationLifecycle::Failed,
                    Some(&lifecycle_callback),
                );
                state_callback(CharacterState::Error);
                drop(operation_guard);
                Self::stop_provisional_local_asr(&mut local_pipeline).await;
                return Err(error.message);
            }
        }

        let input_sample_rate = config.sample_rate_in;
        let output_sample_rate = config.sample_rate_out;
        let (server_ev_tx, server_ev_rx) = mpsc::channel::<LiveServerEvent>(64);
        drop(operation_guard);

        // connect() is intentionally outside operation_lock. A concurrent Stop can invalidate
        // generation immediately; timeout bounds providers that do not cooperate with cancellation.
        let connect_result =
            Self::bounded_provider_operation(provider.connect(config, server_ev_tx)).await;

        let operation_guard = self.operation_lock.lock().await;
        if !self.start_reservation_is_current(&reservation) {
            drop(operation_guard);
            Self::dispose_cancelled_start(&mut local_pipeline, connect_result.ok()).await;
            return Err("Conversation start was cancelled".to_string());
        }

        let mut session = match connect_result {
            Ok(session) => session,
            Err(error_value) => {
                Self::stop_provisional_local_asr(&mut local_pipeline).await;
                Self::set_lifecycle(
                    &self.lifecycle,
                    ConversationLifecycle::Failed,
                    Some(&lifecycle_callback),
                );
                provider_error_callback(error_value.clone());
                state_callback(CharacterState::Error);
                return Err(error_value.to_string());
            }
        };

        if let Err(error_value) = playback.start(output_device) {
            Self::stop_provisional_local_asr(&mut local_pipeline).await;
            playback.flush();
            Self::set_lifecycle(
                &self.lifecycle,
                ConversationLifecycle::Failed,
                Some(&lifecycle_callback),
            );
            state_callback(CharacterState::Error);
            drop(operation_guard);
            Self::close_provisional_session(&mut session).await;
            return Err(format!("failed to start audio output: {error_value}"));
        }

        let (level_tx, mut level_rx) = mpsc::channel::<f32>(32);
        let mut cloud_pcm_rx = None;
        let capture_result = match asr_mode {
            AsrMode::MoonshineTinyStreaming
            | AsrMode::MoonshineSmallStreaming
            | AsrMode::WhisperSmall => local_pipeline
                .as_ref()
                .expect("local ASR mode must have a provisional ASR pipeline")
                .start_capture(&mut capture.lock(), input_device, Some(level_tx))
                .map_err(|error| {
                    self.local_asr_diagnostics
                        .remember_error(asr_mode, error.clone());
                    error.message
                }),
            AsrMode::GeminiLiveAudio => {
                let (pcm_tx, pcm_rx) = mpsc::channel::<Vec<u8>>(32);
                let result = capture
                    .lock()
                    .start(input_device, input_sample_rate, pcm_tx, Some(level_tx))
                    .map_err(|error| format!("failed to start microphone: {error}"));
                if result.is_ok() {
                    cloud_pcm_rx = Some(pcm_rx);
                }
                result
            }
        };

        if let Err(error_value) = capture_result {
            capture.lock().stop();
            playback.flush();
            Self::stop_provisional_local_asr(&mut local_pipeline).await;
            Self::set_lifecycle(
                &self.lifecycle,
                ConversationLifecycle::Failed,
                Some(&lifecycle_callback),
            );
            state_callback(CharacterState::Error);
            drop(operation_guard);
            Self::close_provisional_session(&mut session).await;
            return Err(error_value);
        }

        if let Some(pipeline) = local_pipeline.take() {
            if let Err(error) = self.local_asr.attach(generation, Box::new(pipeline)).await {
                self.local_asr_diagnostics
                    .remember_error(asr_mode, error.clone());
                capture.lock().stop();
                playback.flush();
                Self::set_lifecycle(
                    &self.lifecycle,
                    ConversationLifecycle::Failed,
                    Some(&lifecycle_callback),
                );
                state_callback(CharacterState::Error);
                drop(operation_guard);
                Self::close_provisional_session(&mut session).await;
                return Err(error.message);
            }
        }

        *self.live_session.lock().await = Some(session);
        *self.active_session_id.lock() = Some(session_id.clone());
        *self.active_asr_mode.lock() = Some(asr_mode);
        *self.state_callback.lock() = Some(state_callback.clone());
        *self.lifecycle_callback.lock() = Some(lifecycle_callback.clone());
        *self.transcript_callback.lock() = Some(transcript_callback.clone());
        self.is_in_conversation.store(true, Ordering::SeqCst);
        Self::set_lifecycle(
            &self.lifecycle,
            ConversationLifecycle::Listening,
            Some(&lifecycle_callback),
        );
        state_callback(CharacterState::Listening);
        drop(operation_guard);

        if let Some(mut pcm_rx) = cloud_pcm_rx {
            let manager_for_pcm = self.clone();
            let capture_for_pcm = capture.clone();
            let playback_for_pcm = playback.clone();
            let state_for_pcm = state_callback.clone();
            let provider_error_for_pcm = provider_error_callback.clone();
            tauri::async_runtime::spawn(async move {
                while let Some(chunk) = pcm_rx.recv().await {
                    if !manager_for_pcm.is_in_conversation.load(Ordering::SeqCst)
                        || manager_for_pcm.generation.load(Ordering::SeqCst) != generation
                    {
                        break;
                    }

                    let send_error = manager_for_pcm
                        .forward_microphone_chunk(generation, asr_mode, &chunk)
                        .await
                        .err();

                    if let Some(error_value) = send_error {
                        let cleaned = manager_for_pcm
                            .shutdown_if_generation_current(
                                generation,
                                capture_for_pcm.clone(),
                                playback_for_pcm.clone(),
                                ConversationLifecycle::Failed,
                            )
                            .await;
                        if cleaned {
                            warn!(
                                kind = ?error_value.kind,
                                "Failed to stream microphone audio frame"
                            );
                            provider_error_for_pcm(error_value);
                            state_for_pcm(CharacterState::Error);
                        }
                        break;
                    }
                }
            });
        }

        let is_running_for_level = self.is_in_conversation.clone();
        let generation_for_level = self.generation.clone();
        tauri::async_runtime::spawn(async move {
            while let Some(level) = level_rx.recv().await {
                if !is_running_for_level.load(Ordering::SeqCst)
                    || generation_for_level.load(Ordering::SeqCst) != generation
                {
                    break;
                }
                input_level_callback(level);
            }
        });

        let manager = self.clone();
        let event_loop_context = ConversationEventLoopContext {
            generation,
            session_id: session_id.clone(),
            capture,
            playback,
            output_sample_rate,
            tool_router,
            state_callback,
            lifecycle_callback,
            provider_error_callback,
            transcript_callback,
            speech_bubble_callback,
        };
        tauri::async_runtime::spawn(async move {
            manager
                .run_event_loop(server_ev_rx, event_loop_context)
                .await;
        });

        Ok(session_id)
    }
}
