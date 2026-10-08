use super::*;

#[tokio::test]
async fn cancel_during_readiness_cooperatively_stops_and_reaps_worker() {
    let state = Arc::new(FakeState::default());
    let factory_entered = Arc::new(AtomicBool::new(false));
    let release_factory = Arc::new(AtomicBool::new(false));
    let reaped = Arc::new(AtomicBool::new(false));
    let (callback, _) = callback_events();

    let worker_state = state.clone();
    let entered = factory_entered.clone();
    let release = release_factory.clone();
    let reaped_for_start = reaped.clone();
    let start_task = tokio::spawn(async move {
        LocalAsrPipeline::start_with_factory_and_reaper_probe(
            move || {
                entered.store(true, Ordering::SeqCst);
                while !release.load(Ordering::SeqCst) {
                    thread::sleep(Duration::from_millis(1));
                }
                Ok(Box::new(FakeEngine {
                    state: worker_state,
                    sample_rate: LOCAL_ASR_INPUT_SAMPLE_RATE_HZ,
                }) as Box<dyn PipelineEngine>)
            },
            callback,
            reaped_for_start,
        )
        .await
    });

    wait_until_async(|| factory_entered.load(Ordering::SeqCst)).await;
    start_task.abort();
    let join_error = match start_task.await {
        Ok(_) => panic!("aborted startup task unexpectedly completed"),
        Err(error) => error,
    };
    assert!(join_error.is_cancelled());

    release_factory.store(true, Ordering::SeqCst);
    wait_until_async(|| state.stops.load(Ordering::SeqCst) == 1).await;
    wait_until_async(|| reaped.load(Ordering::SeqCst)).await;
}
#[tokio::test]
async fn readiness_timeout_returns_without_detaching_worker() {
    let state = Arc::new(FakeState::default());
    let reaped = Arc::new(AtomicBool::new(false));
    let (callback, _) = callback_events();
    let worker_state = state.clone();
    let reaped_for_start = reaped.clone();

    let started = Instant::now();
    let result = LocalAsrPipeline::start_with_factory_and_reaper_probe(
        move || {
            thread::sleep(WORKER_STARTUP_TIMEOUT + Duration::from_millis(75));
            Ok(Box::new(FakeEngine {
                state: worker_state,
                sample_rate: LOCAL_ASR_INPUT_SAMPLE_RATE_HZ,
            }) as Box<dyn PipelineEngine>)
        },
        callback,
        reaped_for_start,
    )
    .await;

    let error = result
        .err()
        .expect("startup should have a bounded readiness timeout");
    assert_eq!(error.kind, AsrErrorKind::RuntimeUnavailable);
    assert!(started.elapsed() < WORKER_STARTUP_TIMEOUT + Duration::from_millis(200));
    wait_until_async(|| state.stops.load(Ordering::SeqCst) == 1).await;
    wait_until_async(|| reaped.load(Ordering::SeqCst)).await;
}
#[tokio::test]
async fn worker_runs_off_the_async_caller_thread() {
    let caller_thread = thread::current().id();
    let state = Arc::new(FakeState::default());
    let mut pipeline = fake_pipeline(state.clone()).await;
    pipeline.test_sender().try_send(vec![0, 0]).unwrap();
    wait_until(|| state.pushes.load(Ordering::SeqCst) == 1);
    assert_ne!(
        state.worker_thread.lock().unwrap().as_ref(),
        Some(&caller_thread)
    );
    pipeline.stop_and_join().await.unwrap();
}
#[tokio::test]
async fn startup_failure_never_exposes_running_pipeline() {
    let (callback, _) = callback_events();
    let result = LocalAsrPipeline::start_with_factory(
        || {
            Err(AsrError {
                kind: AsrErrorKind::ModelNotInstalled,
                message: "missing".to_string(),
                retryable: true,
            })
        },
        callback,
    )
    .await;
    let error = match result {
        Ok(_) => panic!("startup unexpectedly succeeded"),
        Err(error) => error,
    };
    assert_eq!(error.kind, AsrErrorKind::ModelNotInstalled);
}
#[tokio::test]
async fn unexpected_engine_sample_rate_fails_before_capture() {
    let state = Arc::new(FakeState::default());
    let (callback, _) = callback_events();
    let result = LocalAsrPipeline::start_with_factory(
        move || {
            Ok(Box::new(FakeEngine {
                state,
                sample_rate: 48_000,
            }))
        },
        callback,
    )
    .await;
    let error = match result {
        Ok(_) => panic!("startup unexpectedly succeeded"),
        Err(error) => error,
    };
    assert_eq!(error.kind, AsrErrorKind::Internal);
}
