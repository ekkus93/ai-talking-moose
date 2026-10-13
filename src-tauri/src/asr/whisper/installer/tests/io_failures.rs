use super::super::integrity::{StagingFile, StagingFileFactory, WhisperVerifyingFileSink};
use super::super::transport::DownloadSink;
use super::{
    fixture_installer, no_staged_or_promoted_artifacts, FakeTransport, FakeTransportBehavior,
    FIXTURE_MODEL,
};
use crate::asr::whisper::installer::{
    WhisperModelInstallCancellation, WhisperModelInstallDisposition, WhisperModelInstallErrorKind,
    WhisperModelInstallPhase, WhisperModelInstallProgress, WhisperModelInstallProgressCallback,
    WhisperModelInstaller,
};
use ring::digest::{digest, SHA256};
use std::fs::{self, File, OpenOptions};
use std::io::{self, Write};
use std::path::Path;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};

#[derive(Clone, Copy, PartialEq, Eq)]
enum IoFailure {
    PartialWrite,
    ImmediateWrite,
    Flush,
    Sync,
}

#[derive(Default)]
struct IoCallCounts {
    files_created: AtomicUsize,
    write_calls: AtomicUsize,
    flush_calls: AtomicUsize,
    sync_calls: AtomicUsize,
}

struct FailingStagingFileFactory {
    failure: IoFailure,
    calls: Arc<IoCallCounts>,
}

impl FailingStagingFileFactory {
    fn new(failure: IoFailure) -> (Arc<Self>, Arc<IoCallCounts>) {
        let calls = Arc::new(IoCallCounts::default());
        (
            Arc::new(Self {
                failure,
                calls: calls.clone(),
            }),
            calls,
        )
    }
}

impl StagingFileFactory for FailingStagingFileFactory {
    fn create_new(&self, path: &Path) -> io::Result<Box<dyn StagingFile>> {
        let file = OpenOptions::new().write(true).create_new(true).open(path)?;
        self.calls.files_created.fetch_add(1, Ordering::SeqCst);
        Ok(Box::new(FailingStagingFile {
            file,
            failure: self.failure,
            calls: self.calls.clone(),
            wrote_prefix: false,
        }))
    }
}

struct FailingStagingFile {
    file: File,
    failure: IoFailure,
    calls: Arc<IoCallCounts>,
    wrote_prefix: bool,
}

impl Write for FailingStagingFile {
    fn write(&mut self, buffer: &[u8]) -> io::Result<usize> {
        self.calls.write_calls.fetch_add(1, Ordering::SeqCst);
        match self.failure {
            IoFailure::ImmediateWrite => Err(io::Error::other(
                "private injected write error with /private/staging/path and fixture bytes",
            )),
            IoFailure::PartialWrite if !self.wrote_prefix => {
                self.wrote_prefix = true;
                let prefix = &buffer[..buffer.len().min(3)];
                self.file.write_all(prefix)?;
                Ok(prefix.len())
            }
            IoFailure::PartialWrite => Err(io::Error::other(
                "private injected partial-write error with /private/staging/path",
            )),
            IoFailure::Flush | IoFailure::Sync => self.file.write(buffer),
        }
    }

    fn flush(&mut self) -> io::Result<()> {
        self.calls.flush_calls.fetch_add(1, Ordering::SeqCst);
        if self.failure == IoFailure::Flush {
            return Err(io::Error::other(
                "private injected flush error with /private/staging/path",
            ));
        }
        self.file.flush()
    }
}

impl StagingFile for FailingStagingFile {
    fn sync_all(&mut self) -> io::Result<()> {
        self.calls.sync_calls.fetch_add(1, Ordering::SeqCst);
        if self.failure == IoFailure::Sync {
            return Err(io::Error::other(
                "private injected sync error with /private/staging/path",
            ));
        }
        self.file.sync_all()
    }
}

fn installer_with_failure(
    failure: IoFailure,
) -> (tempfile::TempDir, WhisperModelInstaller, Arc<IoCallCounts>) {
    let temp = tempfile::tempdir().expect("temporary install root");
    let transport = Arc::new(FakeTransport::new(
        fixture_chunks(FIXTURE_MODEL),
        FakeTransportBehavior::Complete,
    ));
    let (mut installer, _) = fixture_installer(
        temp.path(),
        FIXTURE_MODEL,
        transport,
        Some(FIXTURE_MODEL.len() as u64),
    );
    let (factory, calls) = FailingStagingFileFactory::new(failure);
    installer.staging_file_factory = factory;
    (temp, installer, calls)
}

fn fixture_chunks(bytes: &[u8]) -> Vec<Vec<u8>> {
    let split = bytes.len().min(7);
    vec![bytes[..split].to_vec(), bytes[split..].to_vec()]
}

fn install_error_message(kind: IoFailure) -> &'static str {
    match kind {
        IoFailure::PartialWrite | IoFailure::ImmediateWrite => {
            "The Whisper model installer could not write the staging file."
        }
        IoFailure::Flush | IoFailure::Sync => {
            "The Whisper model installer could not flush the staging file."
        }
    }
}

#[tokio::test]
async fn write_failures_are_sanitized_and_remove_partial_staging_state() {
    for failure in [IoFailure::PartialWrite, IoFailure::ImmediateWrite] {
        let (_temp, installer, calls) = installer_with_failure(failure);
        let progress = Arc::new(Mutex::new(Vec::<WhisperModelInstallProgress>::new()));
        let observed_progress = progress.clone();
        let callback: WhisperModelInstallProgressCallback = Arc::new(move |update| {
            observed_progress.lock().unwrap().push(update);
        });

        let error = installer
            .install(
                &mut WhisperModelInstallCancellation::default(),
                &Some(callback),
            )
            .await
            .unwrap_err();

        assert_eq!(error.kind, WhisperModelInstallErrorKind::Io);
        assert_eq!(error.message, install_error_message(failure));
        assert!(!error.message.contains("private injected"));
        assert!(!error.message.contains("/private/staging/path"));
        assert!(!error.message.contains("fixture bytes"));
        assert_eq!(calls.files_created.load(Ordering::SeqCst), 1);
        assert!(calls.write_calls.load(Ordering::SeqCst) >= 1);
        if failure == IoFailure::PartialWrite {
            assert_eq!(calls.write_calls.load(Ordering::SeqCst), 2);
        }
        assert_eq!(calls.flush_calls.load(Ordering::SeqCst), 0);
        assert_eq!(calls.sync_calls.load(Ordering::SeqCst), 0);
        let progress = progress.lock().unwrap();
        assert_eq!(progress.len(), 1, "failed chunk must not emit progress");
        assert_eq!(progress[0].phase, WhisperModelInstallPhase::Downloading);
        assert_eq!(progress[0].downloaded_bytes, 0);
        no_staged_or_promoted_artifacts(&installer);
    }
}

#[test]
fn failed_partial_write_does_not_advance_verified_byte_accounting() {
    let temp = tempfile::tempdir().expect("temporary directory");
    let path = temp.path().join("staging.bin");
    let (factory, _) = FailingStagingFileFactory::new(IoFailure::PartialWrite);
    let expected_sha256 = digest(&SHA256, FIXTURE_MODEL)
        .as_ref()
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect::<String>();
    let mut sink = WhisperVerifyingFileSink::create(
        &path,
        FIXTURE_MODEL.len() as u64,
        expected_sha256,
        factory.as_ref(),
    )
    .expect("create injected sink");

    let error = sink.write_chunk(FIXTURE_MODEL).unwrap_err();

    assert_eq!(error.kind, WhisperModelInstallErrorKind::Io);
    assert_eq!(sink.bytes_written, 0);
    assert_eq!(
        sink.sha256_so_far(),
        "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855"
    );
    assert_eq!(fs::read(path).unwrap(), &FIXTURE_MODEL[..3]);
}

#[tokio::test]
async fn flush_and_sync_failures_prevent_promotion_and_clean_staging() {
    for failure in [IoFailure::Flush, IoFailure::Sync] {
        let (_temp, mut installer, calls) = installer_with_failure(failure);
        let progress = Arc::new(Mutex::new(Vec::<WhisperModelInstallProgress>::new()));
        let observed_progress = progress.clone();
        let callback: WhisperModelInstallProgressCallback = Arc::new(move |update| {
            observed_progress.lock().unwrap().push(update);
        });
        // A finalization I/O error must take precedence over hash validation.
        installer.verification.expected_sha256 = "00".to_string();
        let error = installer
            .install(
                &mut WhisperModelInstallCancellation::default(),
                &Some(callback),
            )
            .await
            .unwrap_err();

        assert_eq!(error.kind, WhisperModelInstallErrorKind::Io);
        assert_eq!(error.message, install_error_message(failure));
        assert!(!error.message.contains("private injected"));
        assert!(!error.message.contains("/private/staging/path"));
        assert_eq!(calls.files_created.load(Ordering::SeqCst), 1);
        assert_eq!(calls.write_calls.load(Ordering::SeqCst), 2);
        assert_eq!(calls.flush_calls.load(Ordering::SeqCst), 1);
        assert_eq!(
            calls.sync_calls.load(Ordering::SeqCst),
            usize::from(failure == IoFailure::Sync)
        );
        let progress = progress.lock().unwrap();
        assert_eq!(progress.len(), 3);
        assert!(progress
            .iter()
            .all(|update| update.phase == WhisperModelInstallPhase::Downloading));
        assert_eq!(
            progress.last().unwrap().downloaded_bytes,
            FIXTURE_MODEL.len() as u64
        );
        no_staged_or_promoted_artifacts(&installer);
    }
}

#[tokio::test]
async fn verified_install_short_circuits_before_injected_staging_io() {
    let temp = tempfile::tempdir().expect("temporary install root");
    let transport = Arc::new(FakeTransport::new(
        fixture_chunks(FIXTURE_MODEL),
        FakeTransportBehavior::Complete,
    ));
    let (mut installer, _) = fixture_installer(
        temp.path(),
        FIXTURE_MODEL,
        transport,
        Some(FIXTURE_MODEL.len() as u64),
    );
    installer
        .install(&mut WhisperModelInstallCancellation::default(), &None)
        .await
        .expect("install fixture before injecting failures");
    let model_before = fs::read(installer.model_path()).unwrap();
    let marker_before = fs::read(installer.marker_path()).unwrap();

    let (factory, calls) = FailingStagingFileFactory::new(IoFailure::ImmediateWrite);
    installer.staging_file_factory = factory;
    let result = installer
        .install(&mut WhisperModelInstallCancellation::default(), &None)
        .await
        .expect("verified install should bypass staging");

    assert_eq!(
        result.disposition,
        WhisperModelInstallDisposition::AlreadyInstalled
    );
    assert_eq!(calls.files_created.load(Ordering::SeqCst), 0);
    assert_eq!(calls.write_calls.load(Ordering::SeqCst), 0);
    assert_eq!(fs::read(installer.model_path()).unwrap(), model_before);
    assert_eq!(fs::read(installer.marker_path()).unwrap(), marker_before);
}
