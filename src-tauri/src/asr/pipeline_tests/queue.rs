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
async fn processed_chunk_releases_queue_depth() {
    let state = Arc::new(FakeState::default());
    let mut pipeline = fake_pipeline(state.clone()).await;
    pipeline.test_sender().try_send(vec![0, 0]).unwrap();
    wait_until(|| state.pushes.load(Ordering::SeqCst) == 1);
    wait_until(|| pipeline.diagnostics().queue_depth == 0);
    pipeline.stop_and_join().await.unwrap();
}
