use super::*;
use crate::ai::local_tts::DEFAULT_LOCAL_TTS_MODEL_ID;
use std::io::{Seek, SeekFrom, Write};
use tempfile::tempdir;

fn storage() -> (tempfile::TempDir, LocalTtsStorage) {
    let dir = tempdir().unwrap();
    let root = dir.path().join("models").join("tts");
    let storage = LocalTtsStorage::new(root).unwrap();
    (dir, storage)
}

fn seed_install(storage: &LocalTtsStorage, platform: LocalTtsPlatform) -> PathBuf {
    let manifest = local_tts_model_manifest(DEFAULT_LOCAL_TTS_MODEL_ID).unwrap();
    let revision_dir = storage
        .model_revision_dir(DEFAULT_LOCAL_TTS_MODEL_ID)
        .unwrap();
    fs::create_dir_all(&revision_dir).unwrap();
    let artifacts = expected_artifacts(manifest, platform).unwrap();
    for artifact in &artifacts {
        let file = fs::File::create(revision_dir.join(artifact.filename)).unwrap();
        file.set_len(artifact.expected_bytes).unwrap();
    }
    let marker = install_marker(manifest, platform, &artifacts);
    fs::write(
        revision_dir.join(INSTALL_MARKER),
        serde_json::to_vec_pretty(&marker).unwrap(),
    )
    .unwrap();
    revision_dir
}

#[test]
fn storage_layout_separates_staging_from_committed_model_data() {
    let (_dir, storage) = storage();
    let model_dir = storage
        .model_revision_dir(DEFAULT_LOCAL_TTS_MODEL_ID)
        .unwrap();
    assert_eq!(storage.staging_root(), storage.root().join(".staging"));
    assert!(storage.staging_root().is_dir());
    assert!(!model_dir.starts_with(storage.staging_root()));
    assert!(model_dir.starts_with(storage.root().join("kitten-tts-mini-0-8")));
}

#[test]
fn install_state_serialization_is_stable_and_complete() {
    let cases = [
        (LocalTtsInstallState::NotInstalled, "\"not_installed\""),
        (LocalTtsInstallState::Downloading, "\"downloading\""),
        (LocalTtsInstallState::Verifying, "\"verifying\""),
        (LocalTtsInstallState::Promoting, "\"promoting\""),
        (LocalTtsInstallState::Installed, "\"installed\""),
        (LocalTtsInstallState::Failed, "\"failed\""),
    ];
    for (state, expected) in cases {
        assert_eq!(serde_json::to_string(&state).unwrap(), expected);
    }
}

#[test]
fn missing_model_is_not_installed() {
    let (_dir, storage) = storage();
    let status = storage
        .status(DEFAULT_LOCAL_TTS_MODEL_ID, LocalTtsPlatform::LinuxX86_64)
        .unwrap();
    assert_eq!(status.install_state, LocalTtsInstallState::NotInstalled);
    assert_eq!(status.installed_bytes, None);
    assert!(status.error.is_none());
}

#[test]
fn files_without_commit_marker_are_never_installed() {
    let (_dir, storage) = storage();
    let manifest = local_tts_model_manifest(DEFAULT_LOCAL_TTS_MODEL_ID).unwrap();
    let revision_dir = storage
        .model_revision_dir(DEFAULT_LOCAL_TTS_MODEL_ID)
        .unwrap();
    fs::create_dir_all(&revision_dir).unwrap();
    for artifact in expected_artifacts(manifest, LocalTtsPlatform::LinuxX86_64).unwrap() {
        let file = fs::File::create(revision_dir.join(artifact.filename)).unwrap();
        file.set_len(artifact.expected_bytes).unwrap();
    }

    let status = storage
        .status(DEFAULT_LOCAL_TTS_MODEL_ID, LocalTtsPlatform::LinuxX86_64)
        .unwrap();
    assert_eq!(status.install_state, LocalTtsInstallState::Failed);
    assert!(status.error.is_some());
}

#[test]
fn marker_requires_every_platform_specific_artifact() {
    let (_dir, storage) = storage();
    let revision_dir = seed_install(&storage, LocalTtsPlatform::LinuxX86_64);
    fs::remove_file(revision_dir.join("onnxruntime-linux-x64-1.23.2.tgz")).unwrap();

    let status = storage
        .status(DEFAULT_LOCAL_TTS_MODEL_ID, LocalTtsPlatform::LinuxX86_64)
        .unwrap();
    assert_eq!(status.install_state, LocalTtsInstallState::Failed);
}

#[test]
fn exact_marker_and_file_shapes_report_installed_without_hashing() {
    let (_dir, storage) = storage();
    let revision_dir = seed_install(&storage, LocalTtsPlatform::LinuxX86_64);

    let status = storage
        .status(DEFAULT_LOCAL_TTS_MODEL_ID, LocalTtsPlatform::LinuxX86_64)
        .unwrap();
    assert_eq!(status.install_state, LocalTtsInstallState::Installed);
    assert_eq!(status.installed_bytes, Some(status.expected_bytes));

    // Same-size content mutation remains a fast status hit. KTT-204 will cryptographically
    // reject this before runtime use without making every UI refresh hash the model.
    let model_path = revision_dir.join("kitten_tts_mini_v0_8.onnx");
    let mut file = fs::OpenOptions::new().write(true).open(model_path).unwrap();
    file.seek(SeekFrom::Start(0)).unwrap();
    file.write_all(b"changed").unwrap();
    file.flush().unwrap();

    let status_after_mutation = storage
        .status(DEFAULT_LOCAL_TTS_MODEL_ID, LocalTtsPlatform::LinuxX86_64)
        .unwrap();
    assert_eq!(
        status_after_mutation.install_state,
        LocalTtsInstallState::Installed
    );
}

#[test]
fn wrong_platform_marker_is_not_installed() {
    let (_dir, storage) = storage();
    seed_install(&storage, LocalTtsPlatform::LinuxX86_64);
    let status = storage
        .status(DEFAULT_LOCAL_TTS_MODEL_ID, LocalTtsPlatform::MacosArm64)
        .unwrap();
    assert_eq!(status.install_state, LocalTtsInstallState::Failed);
}

#[test]
fn non_directory_model_path_is_failed_not_not_installed() {
    let (_dir, storage) = storage();
    fs::write(
        storage.root().join("kitten-tts-mini-0-8"),
        b"not a directory",
    )
    .unwrap();

    let status = storage
        .status(DEFAULT_LOCAL_TTS_MODEL_ID, LocalTtsPlatform::LinuxX86_64)
        .unwrap();
    assert_eq!(status.install_state, LocalTtsInstallState::Failed);
    assert!(status.error.is_some());
}

#[test]
fn oversized_install_marker_is_failed_without_becoming_ready() {
    let (_dir, storage) = storage();
    let revision_dir = seed_install(&storage, LocalTtsPlatform::LinuxX86_64);
    let marker = fs::OpenOptions::new()
        .write(true)
        .open(revision_dir.join(INSTALL_MARKER))
        .unwrap();
    marker.set_len(MAX_INSTALL_MARKER_BYTES + 1).unwrap();

    let status = storage
        .status(DEFAULT_LOCAL_TTS_MODEL_ID, LocalTtsPlatform::LinuxX86_64)
        .unwrap();
    assert_eq!(status.install_state, LocalTtsInstallState::Failed);
}

#[test]
fn unknown_model_error_does_not_echo_untrusted_model_id() {
    let (_dir, storage) = storage();
    let error = storage
        .status("../../secret-model", LocalTtsPlatform::LinuxX86_64)
        .unwrap_err();
    assert!(!error.message.contains("secret-model"));
    assert!(!error.retryable);
}

#[cfg(unix)]
#[test]
fn model_directory_symlink_is_failed_not_ready() {
    use std::os::unix::fs::symlink;

    let (dir, storage) = storage();
    let outside = dir.path().join("outside");
    fs::create_dir(&outside).unwrap();
    symlink(outside, storage.root().join("kitten-tts-mini-0-8")).unwrap();

    let status = storage
        .status(DEFAULT_LOCAL_TTS_MODEL_ID, LocalTtsPlatform::LinuxX86_64)
        .unwrap();
    assert_eq!(status.install_state, LocalTtsInstallState::Failed);
}
