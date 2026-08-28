async fn supervise_socket(
    mut socket: GeminiSocket,
    api_key: String,
    config: LiveSessionConfig,
    mut receiver: mpsc::Receiver<OutboundEnvelope>,
    mut close_receiver: mpsc::Receiver<CloseRequest>,
    sender: mpsc::Sender<LiveServerEvent>,
    is_active: Arc<AtomicBool>,
    cancellation: CancellationToken,
    diagnostics: Arc<LiveOutboundDiagnosticsStore>,
) {
    let mut input_transcript = String::new();
    let mut output_transcript = String::new();
    let mut resume_handle = None;
    let mut pending_outbound: Option<OutboundEnvelope> = None;

    'connection: loop {
        if let Ok(request) = close_receiver.try_recv() {
            complete_protocol_close(&mut socket, &api_key, request).await;
            break;
        }
        if !is_active.load(Ordering::SeqCst) || cancellation.is_cancelled() {
            break;
        }

        let action = if let Some(envelope) = pending_outbound.take() {
            match write_socket_message(
                &mut socket,
                envelope.message.clone(),
                &api_key,
                Some(&cancellation),
                SOCKET_WRITE_TIMEOUT,
            )
            .await
            {
                Ok(()) => {
                    diagnostics.record(LiveOutboundDeliveryState::Written);
                    ServerAction::Continue
                }
                Err(_) if cancellation.is_cancelled() => continue 'connection,
                Err(_) => {
                    retain_for_retry(&mut pending_outbound, envelope, &diagnostics);
                    ServerAction::Reconnect
                }
            }
        } else {
            tokio::select! {
                biased;
                close = close_receiver.recv() => {
                    match close {
                        Some(request) => {
                            complete_protocol_close(&mut socket, &api_key, request).await;
                            break 'connection;
                        }
                        None => break 'connection,
                    }
                }
                () = cancellation.cancelled() => continue 'connection,
                outbound = receiver.recv() => {
                    match outbound {
                        Some(envelope) => {
                            match write_socket_message(
                                &mut socket,
                                envelope.message.clone(),
                                &api_key,
                                Some(&cancellation),
                                SOCKET_WRITE_TIMEOUT,
                            )
                            .await
                            {
                                Ok(()) => {
                                    diagnostics.record(LiveOutboundDeliveryState::Written);
                                    ServerAction::Continue
                                }
                                Err(_) if cancellation.is_cancelled() => continue 'connection,
                                Err(_) => {
                                    retain_for_retry(&mut pending_outbound, envelope, &diagnostics);
                                    ServerAction::Reconnect
                                }
                            }
                        }
                        None => break 'connection,
                    }
                }
                inbound = socket.next() => {
                    match inbound {
                        Some(Ok(Message::Close(frame))) => {
                            let code = frame.as_ref().map(|value| u16::from(value.code));
                            match provider_error_for_close(code) {
                                Some(error) if error.retryable => ServerAction::Reconnect,
                                Some(error) => ServerAction::Terminal(error),
                                None => {
                                    let failed = record_terminal_outbound_failures(
                                        &mut pending_outbound,
                                        &mut receiver,
                                        &diagnostics,
                                    );
                                    if failed > 0 {
                                        let _ = sender
                                            .send(LiveServerEvent::Error(ProviderError::from_kind(
                                                ProviderErrorKind::Closed,
                                            )))
                                            .await;
                                    } else {
                                        let _ = sender.send(LiveServerEvent::Closed).await;
                                    }
                                    break 'connection;
                                }
                            }
                        }
                        Some(Ok(message)) => match decode_server_frame(message) {
                            Ok(Some(server)) => handle_server_message(
                                server,
                                &sender,
                                &mut input_transcript,
                                &mut output_transcript,
                                &mut resume_handle,
                            ).await,
                            Ok(None) => ServerAction::Continue,
                            Err(error) => ServerAction::Terminal(error),
                        },
                        Some(Err(error)) => {
                            let error = provider_error_for_connect(error);
                            if error.retryable {
                                ServerAction::Reconnect
                            } else {
                                ServerAction::Terminal(error)
                            }
                        }
                        None => ServerAction::Reconnect,
                    }
                }
            }
        };

        match action {
            ServerAction::Continue => {}
            ServerAction::Terminal(error) => {
                record_terminal_outbound_failures(
                    &mut pending_outbound,
                    &mut receiver,
                    &diagnostics,
                );
                let _ = sender.send(LiveServerEvent::Error(error)).await;
                break;
            }
            ServerAction::Reconnect => {
                if !is_active.load(Ordering::SeqCst) || cancellation.is_cancelled() {
                    if let Ok(request) = close_receiver.try_recv() {
                        complete_protocol_close(&mut socket, &api_key, request).await;
                    }
                    break;
                }
                info!("Reconnecting Gemini Live session with bounded backoff");
                match reconnect(
                    &api_key,
                    &config,
                    resume_handle.as_deref(),
                    &is_active,
                    &cancellation,
                )
                .await
                {
                    Ok(new_socket) => {
                        socket = new_socket;
                        let _ = sender.send(LiveServerEvent::Connected).await;
                    }
                    Err(_error)
                        if !is_active.load(Ordering::SeqCst) || cancellation.is_cancelled() =>
                    {
                        if let Ok(request) = close_receiver.try_recv() {
                            complete_protocol_close(&mut socket, &api_key, request).await;
                        }
                        break;
                    }
                    Err(error) => {
                        record_terminal_outbound_failures(
                            &mut pending_outbound,
                            &mut receiver,
                            &diagnostics,
                        );
                        let _ = sender.send(LiveServerEvent::Error(error)).await;
                        break;
                    }
                }
            }
        }
    }

    record_terminal_outbound_failures(&mut pending_outbound, &mut receiver, &diagnostics);
    is_active.store(false, Ordering::SeqCst);
}
