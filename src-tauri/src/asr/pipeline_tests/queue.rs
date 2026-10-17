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
