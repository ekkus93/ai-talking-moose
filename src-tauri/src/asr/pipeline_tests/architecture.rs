use super::*;

#[tokio::test]
async fn mock_capture_uses_same_authoritative_capture_at_16khz() {
    let state = Arc::new(FakeState::default());
    let mut pipeline = fake_pipeline(state).await;
    let mut capture = AudioCapture::new_mock();
    pipeline.start_capture(&mut capture, None, None).unwrap();
    let diagnostics = capture.diagnostics();
    assert!(diagnostics.active);
    assert_eq!(
        diagnostics.sample_rate_hz,
        Some(LOCAL_ASR_INPUT_SAMPLE_RATE_HZ)
    );
    assert_eq!(diagnostics.channels, Some(1));
    capture.stop();
    pipeline.stop_and_join().await.unwrap();
}
#[tokio::test]
async fn small_pipeline_uses_same_bounded_worker_and_reports_small_architecture() {
    let state = Arc::new(FakeState::default());
    let mut pipeline = fake_pipeline_for_architecture(
        LocalAsrArchitecture::MoonshineSmallStreaming,
        state.clone(),
    )
    .await;

    let diagnostics = pipeline.diagnostics();
    assert_eq!(
        diagnostics.architecture,
        LocalAsrArchitecture::MoonshineSmallStreaming
    );
    assert_eq!(diagnostics.input_sample_rate_hz, 16_000);
    assert_eq!(diagnostics.queue_capacity, LOCAL_ASR_QUEUE_CAPACITY_CHUNKS);

    pipeline.test_sender().try_send(vec![0, 0]).unwrap();
    wait_until(|| state.pushes.load(Ordering::SeqCst) == 1);
    pipeline.stop_and_join().await.unwrap();
    assert_eq!(state.stops.load(Ordering::SeqCst), 1);
}
#[tokio::test]
async fn whisper_pipeline_uses_its_own_inference_absorption_queue() {
    let state = Arc::new(FakeState::default());
    let mut pipeline =
        fake_pipeline_for_architecture(LocalAsrArchitecture::WhisperSmall, state).await;

    let diagnostics = pipeline.diagnostics();
    assert_eq!(diagnostics.architecture, LocalAsrArchitecture::WhisperSmall);
    assert_eq!(
        diagnostics.queue_capacity,
        WHISPER_LOCAL_ASR_QUEUE_CAPACITY_CHUNKS
    );
    assert_eq!(diagnostics.queue_depth, 0);

    pipeline.stop_and_join().await.unwrap();
}
#[tokio::test]
async fn pipeline_diagnostics_report_bound_and_running_state() {
    let state = Arc::new(FakeState::default());
    let mut pipeline = fake_pipeline(state).await;
    let diagnostics = pipeline.diagnostics();
    assert_eq!(
        diagnostics.architecture,
        LocalAsrArchitecture::MoonshineTinyStreaming
    );
    assert_eq!(diagnostics.input_sample_rate_hz, 16_000);
    assert_eq!(diagnostics.queue_capacity, LOCAL_ASR_QUEUE_CAPACITY_CHUNKS);
    assert_eq!(diagnostics.queue_depth, 0);
    assert!(diagnostics.running);
    assert!(diagnostics.last_error.is_none());
    pipeline.stop_and_join().await.unwrap();
}
#[tokio::test]
async fn wake_word_stability_continuous_asr_idle_cpu_measurement_reports_diagnostics() {
    let state = Arc::new(FakeState::default());
    let mut pipeline = fake_pipeline(state).await;
    let observation = Duration::from_secs(2);

    tokio::time::sleep(observation).await;

    let diagnostics = pipeline.diagnostics();
    let continuous_asr_idle_cpu_percent = diagnostics
        .average_cpu_utilization_percent
        .expect("continuous ASR idle diagnostics should include CPU utilization");
    assert!(continuous_asr_idle_cpu_percent.is_finite());
    assert!(diagnostics.running);
    assert_eq!(diagnostics.queue_depth, 0);
    assert_eq!(diagnostics.queue_capacity, LOCAL_ASR_QUEUE_CAPACITY_CHUNKS);
    println!(
        "WWR630_CONTINUOUS_ASR_IDLE_CPU continuous_asr_idle_cpu_percent={:.3} observation_ms={} queue_depth={} queue_capacity={} architecture={:?}",
        continuous_asr_idle_cpu_percent,
        observation.as_millis(),
        diagnostics.queue_depth,
        diagnostics.queue_capacity,
        diagnostics.architecture
    );

    pipeline.stop_and_join().await.unwrap();
}
