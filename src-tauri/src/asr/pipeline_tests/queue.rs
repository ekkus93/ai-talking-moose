use super::*;

#[tokio::test]
async fn ingress_is_hard_bounded_and_drops_newest() {
    let state = Arc::new(FakeState::default());
    state.block_push.store(true, Ordering::SeqCst);
    let mut pipeline = fake_pipeline(state.clone()).await;
    let sender = pipeline.test_sender();

    sender.try_send(vec![0, 0]).unwrap();
    wait_until(|| state.pushes.load(Ordering::SeqCst) == 1);
    for _ in 0..LOCAL_ASR_QUEUE_CAPACITY_CHUNKS {
        sender.try_send(vec![0, 0]).unwrap();
    }
    assert_eq!(
        pipeline.diagnostics().queue_depth,
        LOCAL_ASR_QUEUE_CAPACITY_CHUNKS
    );
    assert!(matches!(
        sender.try_send(vec![0, 0]),
        Err(mpsc::error::TrySendError::Full(_))
    ));

    state.block_push.store(false, Ordering::SeqCst);
    pipeline.stop_and_join().await.unwrap();
    assert_eq!(pipeline.diagnostics().queue_depth, 0);
}
#[tokio::test]
async fn wake_handoff_primes_existing_ingress_in_exact_sample_order() {
    let state = Arc::new(FakeState::default());
    let mut pipeline = fake_pipeline(state.clone()).await;
    let samples = vec![i16::MIN, -1234, 0, 2345, i16::MAX];
    let handoff =
        WakeCommandHandoffAudio::new(LOCAL_ASR_INPUT_SAMPLE_RATE_HZ, samples.clone()).unwrap();

    pipeline.prime_wake_handoff(handoff).unwrap();
    wait_until(|| state.pushes.load(Ordering::SeqCst) == 1);

    let expected = AudioResampler::i16_to_f32(&samples);
    assert_eq!(state.received_pcm.lock().unwrap().as_slice(), &[expected]);
    pipeline.stop_and_join().await.unwrap();
}
#[tokio::test]
async fn wake_handoff_queue_full_fails_without_dropping_or_reordering_payload() {
    let state = Arc::new(FakeState::default());
    state.block_push.store(true, Ordering::SeqCst);
    let mut pipeline = fake_pipeline(state.clone()).await;
    let sender = pipeline.test_sender();
    sender.try_send(vec![0, 0]).unwrap();
    wait_until(|| state.pushes.load(Ordering::SeqCst) == 1);
    for _ in 0..LOCAL_ASR_QUEUE_CAPACITY_CHUNKS {
        sender.try_send(vec![0, 0]).unwrap();
    }

    let handoff =
        WakeCommandHandoffAudio::new(LOCAL_ASR_INPUT_SAMPLE_RATE_HZ, vec![7, 8, 9]).unwrap();
    let error = pipeline.prime_wake_handoff(handoff).unwrap_err();
    assert_eq!(error.kind, AsrErrorKind::AudioInput);

    state.block_push.store(false, Ordering::SeqCst);
    pipeline.stop_and_join().await.unwrap();
}
#[tokio::test]
async fn whisper_wake_handoff_precedes_live_pcm_on_the_same_ingress() {
    let state = Arc::new(FakeState::default());
    let mut pipeline =
        fake_pipeline_for_architecture(LocalAsrArchitecture::WhisperSmall, state.clone()).await;
    let handoff_samples = vec![i16::MIN, -1, 100, i16::MAX];
    let live_samples = vec![77, -88, 99];
    let handoff =
        WakeCommandHandoffAudio::new(LOCAL_ASR_INPUT_SAMPLE_RATE_HZ, handoff_samples.clone())
            .unwrap();

    pipeline.prime_wake_handoff(handoff).unwrap();
    pipeline
        .test_sender()
        .try_send(AudioResampler::i16_to_bytes(&live_samples))
        .unwrap();
    wait_until(|| state.received_pcm.lock().unwrap().len() == 2);

    assert_eq!(
        state.received_pcm.lock().unwrap().as_slice(),
        &[
            AudioResampler::i16_to_f32(&handoff_samples),
            AudioResampler::i16_to_f32(&live_samples),
        ]
    );
    assert_eq!(
        pipeline.diagnostics().queue_capacity,
        WHISPER_LOCAL_ASR_QUEUE_CAPACITY_CHUNKS
    );
    pipeline.stop_and_join().await.unwrap();
    assert_eq!(state.stops.load(Ordering::SeqCst), 1);
}

#[tokio::test]
async fn whisper_full_wake_handoff_queue_fails_closed_without_overwriting_pcm() {
    let state = Arc::new(FakeState::default());
    state.block_push.store(true, Ordering::SeqCst);
    let mut pipeline =
        fake_pipeline_for_architecture(LocalAsrArchitecture::WhisperSmall, state.clone()).await;
    let sender = pipeline.test_sender();
    sender.try_send(vec![0, 0]).unwrap();
    wait_until(|| state.pushes.load(Ordering::SeqCst) == 1);
    for _ in 0..WHISPER_LOCAL_ASR_QUEUE_CAPACITY_CHUNKS {
        sender.try_send(vec![0, 0]).unwrap();
    }

    let error = pipeline
        .prime_wake_handoff(
            WakeCommandHandoffAudio::new(LOCAL_ASR_INPUT_SAMPLE_RATE_HZ, vec![7, 8, 9]).unwrap(),
        )
        .unwrap_err();
    assert_eq!(error.kind, AsrErrorKind::AudioInput);
    assert_eq!(
        pipeline.diagnostics().queue_depth,
        WHISPER_LOCAL_ASR_QUEUE_CAPACITY_CHUNKS
    );

    state.block_push.store(false, Ordering::SeqCst);
    pipeline.stop_and_join().await.unwrap();
    assert_eq!(
        state.received_pcm.lock().unwrap().len(),
        WHISPER_LOCAL_ASR_QUEUE_CAPACITY_CHUNKS + 1
    );
}

#[tokio::test]
async fn processed_chunk_releases_queue_depth() {
    let state = Arc::new(FakeState::default());
    let mut pipeline = fake_pipeline(state.clone()).await;
    pipeline.test_sender().try_send(vec![0, 0]).unwrap();
    wait_until(|| state.pushes.load(Ordering::SeqCst) == 1);
    wait_until(|| pipeline.diagnostics().queue_depth == 0);
    pipeline.stop_and_join().await.unwrap();
}

#[tokio::test]
async fn whisper_ingress_rejects_newest_chunk_and_drains_older_pcm_in_order() {
    let state = Arc::new(FakeState::default());
    state.block_push.store(true, Ordering::SeqCst);
    let mut pipeline =
        fake_pipeline_for_architecture(LocalAsrArchitecture::WhisperSmall, state.clone()).await;
    let sender = pipeline.test_sender();

    let first = [i16::MIN, -111];
    sender
        .try_send(AudioResampler::i16_to_bytes(&first))
        .unwrap();
    wait_until(|| state.pushes.load(Ordering::SeqCst) == 1);

    for index in 0..WHISPER_LOCAL_ASR_QUEUE_CAPACITY_CHUNKS {
        sender
            .try_send(AudioResampler::i16_to_bytes(&[(index + 1) as i16]))
            .unwrap();
    }
    assert_eq!(
        pipeline.diagnostics().queue_depth,
        WHISPER_LOCAL_ASR_QUEUE_CAPACITY_CHUNKS
    );

    let rejected = AudioResampler::i16_to_bytes(&[i16::MAX]);
    assert!(matches!(
        sender.try_send(rejected),
        Err(mpsc::error::TrySendError::Full(_))
    ));

    state.block_push.store(false, Ordering::SeqCst);
    pipeline.stop_and_join().await.unwrap();

    let received = state.received_pcm.lock().unwrap();
    assert_eq!(received.len(), WHISPER_LOCAL_ASR_QUEUE_CAPACITY_CHUNKS + 1);
    assert_eq!(received[0], AudioResampler::i16_to_f32(&first));
    for index in 0..WHISPER_LOCAL_ASR_QUEUE_CAPACITY_CHUNKS {
        assert_eq!(
            received[index + 1],
            AudioResampler::i16_to_f32(&[(index + 1) as i16]),
            "queued PCM must retain its original chronological position"
        );
    }
    assert_eq!(state.stops.load(Ordering::SeqCst), 1);
}

#[tokio::test]
async fn whisper_empty_pcm_does_not_infer_or_reorder_following_audio() {
    let state = Arc::new(FakeState::default());
    let mut pipeline =
        fake_pipeline_for_architecture(LocalAsrArchitecture::WhisperSmall, state.clone()).await;
    let sender = pipeline.test_sender();

    // An empty capture buffer is not a spoken utterance and must not reach
    // native inference or displace the following valid 16-bit PCM sample.
    sender.try_send(Vec::new()).unwrap();
    sender
        .try_send(AudioResampler::i16_to_bytes(&[i16::MIN, i16::MAX]))
        .unwrap();
    pipeline.stop_and_join().await.unwrap();

    assert_eq!(state.pushes.load(Ordering::SeqCst), 1);
    assert_eq!(state.stops.load(Ordering::SeqCst), 1);
    assert_eq!(
        state.received_pcm.lock().unwrap().as_slice(),
        &[AudioResampler::i16_to_f32(&[i16::MIN, i16::MAX])]
    );
}

#[tokio::test]
async fn whisper_full_wake_handoff_can_retry_after_queue_drains_without_duplication() {
    let state = Arc::new(FakeState::default());
    state.block_push.store(true, Ordering::SeqCst);
    let mut pipeline =
        fake_pipeline_for_architecture(LocalAsrArchitecture::WhisperSmall, state.clone()).await;
    let sender = pipeline.test_sender();

    sender.try_send(vec![0, 0]).unwrap();
    wait_until(|| state.pushes.load(Ordering::SeqCst) == 1);
    for _ in 0..WHISPER_LOCAL_ASR_QUEUE_CAPACITY_CHUNKS {
        sender.try_send(vec![0, 0]).unwrap();
    }

    let handoff_samples = vec![1234, -2345, 3456];
    let handoff =
        WakeCommandHandoffAudio::new(LOCAL_ASR_INPUT_SAMPLE_RATE_HZ, handoff_samples.clone())
            .unwrap();
    let error = pipeline.prime_wake_handoff(handoff.clone()).unwrap_err();
    assert_eq!(error.kind, AsrErrorKind::AudioInput);
    assert_eq!(
        pipeline.diagnostics().queue_depth,
        WHISPER_LOCAL_ASR_QUEUE_CAPACITY_CHUNKS,
        "a rejected wake handoff must not occupy a queue slot"
    );

    state.block_push.store(false, Ordering::SeqCst);
    wait_until(|| pipeline.diagnostics().queue_depth == 0);
    pipeline.prime_wake_handoff(handoff).unwrap();
    pipeline.stop_and_join().await.unwrap();

    let received = state.received_pcm.lock().unwrap();
    let expected_pcm = AudioResampler::i16_to_f32(&handoff_samples);
    assert_eq!(received.len(), WHISPER_LOCAL_ASR_QUEUE_CAPACITY_CHUNKS + 2);
    assert_eq!(
        received.last().map(Vec::as_slice),
        Some(expected_pcm.as_slice()),
        "the accepted retry must retain its exact sample order"
    );
    assert_eq!(
        received
            .iter()
            .filter(|pcm| pcm.as_slice() == expected_pcm.as_slice())
            .count(),
        1,
        "a rejected handoff must not be replayed or duplicated"
    );
    assert_eq!(state.stops.load(Ordering::SeqCst), 1);
}

#[tokio::test]
async fn whisper_wake_handoff_rejected_during_graceful_stop_preserves_queued_pcm() {
    let state = Arc::new(FakeState::default());
    state.block_push.store(true, Ordering::SeqCst);
    let mut pipeline =
        fake_pipeline_for_architecture(LocalAsrArchitecture::WhisperSmall, state.clone()).await;
    let sender = pipeline.test_sender();
    let first = [111_i16, -222];
    let queued = [333_i16, -444];
    sender
        .try_send(AudioResampler::i16_to_bytes(&first))
        .unwrap();
    wait_until(|| state.pushes.load(Ordering::SeqCst) == 1);
    sender
        .try_send(AudioResampler::i16_to_bytes(&queued))
        .unwrap();
    assert_eq!(pipeline.diagnostics().queue_depth, 1);

    // A stop request closes the owned ingress even while native inference is
    // still finishing. Wake-word handoff must not re-open that generation.
    pipeline.request_stop();
    let handoff =
        WakeCommandHandoffAudio::new(LOCAL_ASR_INPUT_SAMPLE_RATE_HZ, vec![555, -666]).unwrap();
    let error = pipeline.prime_wake_handoff(handoff).unwrap_err();
    assert_eq!(error.kind, AsrErrorKind::InvalidState);
    assert!(error.retryable);
    assert_eq!(pipeline.diagnostics().queue_depth, 0);

    state.block_push.store(false, Ordering::SeqCst);
    pipeline.stop_and_join().await.unwrap();
    let received = state.received_pcm.lock().unwrap();
    assert_eq!(
        received.as_slice(),
        &[
            AudioResampler::i16_to_f32(&first),
            AudioResampler::i16_to_f32(&queued),
        ],
        "stop must drain accepted PCM without accepting a late wake handoff"
    );
    assert_eq!(state.stops.load(Ordering::SeqCst), 1);
    assert!(!pipeline.is_running());
}

#[tokio::test]
async fn whisper_wake_handoff_after_abort_is_rejected_without_replaying_pcm() {
    let state = Arc::new(FakeState::default());
    state.block_push.store(true, Ordering::SeqCst);
    let (callback, events) = callback_events();
    let worker_state = state.clone();
    let mut pipeline = LocalAsrPipeline::start_architecture(
        LocalAsrArchitecture::WhisperSmall,
        WHISPER_LOCAL_ASR_QUEUE_CAPACITY_CHUNKS,
        move || {
            Ok(Box::new(FakeEngine {
                state: worker_state,
                sample_rate: LOCAL_ASR_INPUT_SAMPLE_RATE_HZ,
            }) as Box<dyn PipelineEngine>)
        },
        callback,
        WORKER_STARTUP_TIMEOUT,
        None,
    )
    .await
    .unwrap();

    let sender = pipeline.test_sender();
    sender
        .try_send(AudioResampler::i16_to_bytes(&[111]))
        .unwrap();
    wait_until(|| state.pushes.load(Ordering::SeqCst) == 1);
    sender
        .try_send(AudioResampler::i16_to_bytes(&[222]))
        .unwrap();
    assert_eq!(pipeline.diagnostics().queue_depth, 1);

    // A retired generation must reject new wake audio and discard queued
    // PCM, even when the native inference thread has not yet returned.
    pipeline.request_abort();
    let handoff = WakeCommandHandoffAudio::new(LOCAL_ASR_INPUT_SAMPLE_RATE_HZ, vec![333]).unwrap();
    let error = pipeline.prime_wake_handoff(handoff).unwrap_err();
    assert_eq!(error.kind, AsrErrorKind::InvalidState);
    assert!(error.retryable);

    state.block_push.store(false, Ordering::SeqCst);
    pipeline.stop_and_join().await.unwrap();
    assert_eq!(state.pushes.load(Ordering::SeqCst), 1);
    assert_eq!(state.stops.load(Ordering::SeqCst), 1);
    assert!(events.lock().unwrap().is_empty());
    assert!(!pipeline.is_running());
    assert_eq!(
        state.received_pcm.lock().unwrap().as_slice(),
        &[AudioResampler::i16_to_f32(&[111])],
        "aborted queued PCM and rejected wake handoff must not reach inference"
    );
}
