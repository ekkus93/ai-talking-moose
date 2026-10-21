//! An untrusted marker symlink cannot attest model integrity, and deleting
//! the model must unlink the marker itself without touching an outside file.
use super::*;
use std::os::unix::fs::symlink;

#[tokio::test]
async fn untrusted_marker_symlink_is_not_verified_or_followed_during_delete() {
    let temp = tempfile::TempDir::new().unwrap();
    let transport = Arc::new(FakeTransport::new(
        fixture_chunks(FIXTURE_MODEL),
        FakeTransportBehavior::Complete,
    ));
    let (installer, _) = fixture_installer(
        temp.path(),
        FIXTURE_MODEL,
        transport.clone(),
        Some(FIXTURE_MODEL.len() as u64),
    );
    installer
        .install(&mut WhisperModelInstallCancellation::default(), &None)
        .await
        .unwrap();
    assert!(installer.verify_installed().unwrap().is_some());

    let outside_marker = temp.path().join("unrelated-marker.json");
    fs::write(&outside_marker, b"external metadata must remain unchanged").unwrap();
    fs::remove_file(installer.marker_path()).unwrap();
    symlink(&outside_marker, installer.marker_path()).unwrap();

    let error = installer.verify_installed().unwrap_err();
    assert_eq!(error.kind, WhisperModelInstallErrorKind::CorruptInstall);
    assert_eq!(
        fs::read(&outside_marker).unwrap(),
        b"external metadata must remain unchanged"
    );

    installer.delete().await.unwrap();
    assert!(fs::symlink_metadata(installer.marker_path()).is_err());
    assert!(fs::symlink_metadata(installer.model_path()).is_err());
    assert_eq!(
        fs::read(&outside_marker).unwrap(),
        b"external metadata must remain unchanged"
    );
    assert_eq!(transport.calls.load(Ordering::SeqCst), 1);
}
