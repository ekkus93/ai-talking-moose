use super::*;

#[test]
fn failed_atomic_promotion_never_creates_install_marker() {
    let dir = tempdir().unwrap();
    let missing_staging = dir.path().join(STAGING_DIR).join("missing.partial");

    let error = promote_artifact(dir.path(), &TEST_ENTRY, &missing_staging).unwrap_err();

    assert_eq!(error.kind, LocalModelInstallErrorKind::Promotion);
    let revision_dir = dir.path().join(TEST_ENTRY.id).join(TEST_ENTRY.revision);
    assert!(!revision_dir.join(TEST_ENTRY.artifact_filename).exists());
    assert!(!revision_dir.join(INSTALL_MARKER).exists());
}

#[cfg(unix)]
#[test]
fn model_root_symlink_is_rejected_without_touching_target() {
    use std::os::unix::fs::symlink;

    let parent = tempdir().unwrap();
    let outside = tempdir().unwrap();
    let sentinel = outside.path().join("keep.txt");
    fs::write(&sentinel, b"keep").unwrap();
    let root = parent.path().join("llm-root");
    symlink(outside.path(), &root).unwrap();

    let error = LocalModelInstaller::new(root)
        .err()
        .expect("model-root symlink must fail closed");

    assert_eq!(error.kind, LocalModelInstallErrorKind::CorruptInstall);
    assert_eq!(fs::read(&sentinel).unwrap(), b"keep");
}

#[cfg(unix)]
#[test]
fn staging_directory_symlink_is_rejected_without_touching_target() {
    use std::os::unix::fs::symlink;

    let dir = tempdir().unwrap();
    let outside = tempdir().unwrap();
    let sentinel = outside.path().join("keep.txt");
    fs::write(&sentinel, b"keep").unwrap();
    symlink(outside.path(), dir.path().join(STAGING_DIR)).unwrap();

    let error = LocalModelInstaller::new(dir.path().to_path_buf())
        .err()
        .expect("staging symlink must fail closed");

    assert_eq!(error.kind, LocalModelInstallErrorKind::CorruptInstall);
    assert_eq!(fs::read(&sentinel).unwrap(), b"keep");
}

#[cfg(unix)]
#[test]
fn model_path_rejects_root_replaced_by_symlink_after_initialization() {
    use std::os::unix::fs::symlink;

    let parent = tempdir().unwrap();
    let outside = tempdir().unwrap();
    let root = parent.path().join("llm-root");
    let installer = LocalModelInstaller::new(root.clone()).unwrap();
    fs::remove_dir_all(&root).unwrap();
    symlink(outside.path(), &root).unwrap();

    let error = installer
        .model_path(crate::ai::local::catalog::DEFAULT_LOCAL_TEXT_MODEL_ID)
        .unwrap_err();

    assert_eq!(error.kind, LocalModelInstallErrorKind::CorruptInstall);
}

#[cfg(unix)]
#[test]
fn model_path_rejects_staging_replaced_by_symlink_after_initialization() {
    use std::os::unix::fs::symlink;

    let dir = tempdir().unwrap();
    let outside = tempdir().unwrap();
    let installer = LocalModelInstaller::new(dir.path().to_path_buf()).unwrap();
    let staging = dir.path().join(STAGING_DIR);
    fs::remove_dir(&staging).unwrap();
    symlink(outside.path(), &staging).unwrap();

    let error = installer
        .model_path(crate::ai::local::catalog::DEFAULT_LOCAL_TEXT_MODEL_ID)
        .unwrap_err();

    assert_eq!(error.kind, LocalModelInstallErrorKind::CorruptInstall);
}

#[cfg(unix)]
#[test]
fn promotion_rejects_model_directory_symlink_without_root_escape() {
    use std::os::unix::fs::symlink;

    let dir = tempdir().unwrap();
    let outside = tempdir().unwrap();
    let staging = dir.path().join(STAGING_DIR);
    fs::create_dir(&staging).unwrap();
    let staging_file = staging.join("verified.partial");
    fs::write(&staging_file, b"abc").unwrap();
    symlink(outside.path(), dir.path().join(TEST_ENTRY.id)).unwrap();

    let error = promote_artifact(dir.path(), &TEST_ENTRY, &staging_file).unwrap_err();

    assert_eq!(error.kind, LocalModelInstallErrorKind::CorruptInstall);
    assert!(!outside.path().join(TEST_ENTRY.revision).exists());
    assert_eq!(fs::read(&staging_file).unwrap(), b"abc");
}

#[cfg(unix)]
#[test]
fn promotion_rejects_revision_directory_symlink_without_root_escape() {
    use std::os::unix::fs::symlink;

    let dir = tempdir().unwrap();
    let outside = tempdir().unwrap();
    let staging = dir.path().join(STAGING_DIR);
    fs::create_dir(&staging).unwrap();
    let staging_file = staging.join("verified.partial");
    fs::write(&staging_file, b"abc").unwrap();
    let model_dir = dir.path().join(TEST_ENTRY.id);
    fs::create_dir(&model_dir).unwrap();
    symlink(outside.path(), model_dir.join(TEST_ENTRY.revision)).unwrap();

    let error = promote_artifact(dir.path(), &TEST_ENTRY, &staging_file).unwrap_err();

    assert_eq!(error.kind, LocalModelInstallErrorKind::CorruptInstall);
    assert!(!outside.path().join(TEST_ENTRY.artifact_filename).exists());
    assert_eq!(fs::read(&staging_file).unwrap(), b"abc");
}

#[cfg(unix)]
#[test]
fn marker_symlink_failure_removes_promoted_artifact_and_preserves_target() {
    use std::os::unix::fs::symlink;

    let dir = tempdir().unwrap();
    let outside = tempdir().unwrap();
    let outside_marker = outside.path().join("marker.txt");
    fs::write(&outside_marker, b"keep").unwrap();
    let staging = dir.path().join(STAGING_DIR);
    fs::create_dir(&staging).unwrap();
    let staging_file = staging.join("verified.partial");
    fs::write(&staging_file, b"abc").unwrap();
    let revision_dir = dir.path().join(TEST_ENTRY.id).join(TEST_ENTRY.revision);
    fs::create_dir_all(&revision_dir).unwrap();
    symlink(&outside_marker, revision_dir.join(INSTALL_MARKER)).unwrap();

    let error = promote_artifact(dir.path(), &TEST_ENTRY, &staging_file).unwrap_err();

    assert_eq!(error.kind, LocalModelInstallErrorKind::CorruptInstall);
    assert!(!revision_dir.join(TEST_ENTRY.artifact_filename).exists());
    assert_eq!(fs::read(&outside_marker).unwrap(), b"keep");
}

#[cfg(unix)]
#[test]
fn symlink_artifact_never_counts_as_installed() {
    use std::os::unix::fs::symlink;

    let dir = tempdir().unwrap();
    let installer = LocalModelInstaller::with_transport(
        dir.path().to_path_buf(),
        Arc::new(StaticBytesTransport {
            bytes: b"abc",
            wait_for_cancel_after_write: false,
        }),
    )
    .unwrap();
    let revision_dir = dir.path().join(TEST_ENTRY.id).join(TEST_ENTRY.revision);
    fs::create_dir_all(&revision_dir).unwrap();
    let real_artifact = dir.path().join("real.gguf");
    fs::write(&real_artifact, b"abc").unwrap();
    symlink(
        &real_artifact,
        revision_dir.join(TEST_ENTRY.artifact_filename),
    )
    .unwrap();
    write_test_marker(&revision_dir);

    assert!(!installer.marker_shape_is_valid(&TEST_ENTRY));
}

#[cfg(unix)]
#[test]
fn deleting_model_tree_does_not_follow_revision_symlink() {
    use std::os::unix::fs::symlink;

    let dir = tempdir().unwrap();
    let outside = tempdir().unwrap();
    let sentinel = outside.path().join("keep.txt");
    fs::write(&sentinel, b"keep").unwrap();
    let installer = LocalModelInstaller::new(dir.path().to_path_buf()).unwrap();
    let entry = local_model_entry(crate::ai::local::catalog::DEFAULT_LOCAL_TEXT_MODEL_ID).unwrap();
    let model_dir = dir.path().join(entry.id);
    fs::create_dir(&model_dir).unwrap();
    symlink(outside.path(), model_dir.join(entry.revision)).unwrap();

    installer.delete(entry.id).unwrap();

    assert_eq!(fs::read(&sentinel).unwrap(), b"keep");
    assert!(!model_dir.exists());
}
