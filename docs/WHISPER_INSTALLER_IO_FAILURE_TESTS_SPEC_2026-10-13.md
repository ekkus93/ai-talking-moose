# Whisper Installer Staging I/O Failure Tests — Specification

**Date:** 2026-10-13<br>
**Status:** Implemented<br>
**Companion TODO:** `docs/WHISPER_INSTALLER_IO_FAILURE_TESTS_TODO_2026-10-13.md`<br>
**Related coverage audit:** `docs/ASR_UNIT_TEST_COVERAGE_TODO_2026-10-08.md`

## 1. Purpose

Add deterministic unit coverage for staging-file write and finalization failures in the Whisper model installer. The existing tests cover staging-file creation failure and installer cleanup for transport, verification, cancellation, and promotion failures. They do not inject a failure from `write_all`, `flush`, or `sync_all` because `WhisperVerifyingFileSink` currently owns a concrete `std::fs::File`.

The tests must prove that these failures stay in the sanitized installer I/O error category and that a failed install cannot leave a canonical model, ready marker, or staging directory behind.

## 2. Current behavior

`WhisperModelInstaller::install` creates a unique staging directory and an `InstallerFileSink`. Download chunks flow through `WhisperVerifyingFileSink::write_chunk`, which writes bytes before updating its byte count and SHA-256 state. `finish` flushes and synchronizes the staging file before checking size and SHA-256. Errors are mapped to stable messages such as “The Whisper model installer could not write the staging file.” The install path removes the staging directory after a sink write or finish failure.

The existing fixture transport and installer dependency seams already support deterministic download, disk-space, cancellation, and cleanup tests. This work should extend the narrow sink/file boundary as needed without changing those contracts.

## 3. Goals

- Exercise a staging write failure through the installer path using a deterministic injected writer/file operation.
- Exercise finalization failures from `flush` and `sync_all` separately if the chosen seam can distinguish them without broadening production abstractions unnecessarily.
- Prove partially written staging bytes are removed when the install fails.
- Assert stable `WhisperModelInstallErrorKind::Io` classification and path/content-free messages.
- Assert no canonical model or install marker is promoted after a write or finalization failure.
- Keep the production path backed by `std::fs::File`, with `create_new` staging-file semantics, streaming verification, and artifact identity unchanged.
- Keep the tests independent of model weights, network access, audio hardware, and operating-system-specific failure tricks.

## 4. Requirements

### 4.1 Private failure-injection seam

Introduce the smallest private seam that lets tests control staging writes and finalization. A private writer/file-operations trait, generic sink helper, or narrowly injected sink factory is acceptable. Production construction must continue to create the staging file with the current exclusive-create behavior and use real file writes, flush, and synchronization.

Do not expose the seam through IPC, public settings, or application APIs. Do not replace production integrity checks or allow a test-provided model identity to reach production construction.

### 4.2 Write failure behavior

Provide a deterministic writer that can accept a prefix of a chunk and then return an I/O error, exercising `write_all`'s partial-write loop. Where useful, also cover an immediate write error. The test must assert:

- the installer returns the sanitized staging-write I/O error;
- the injected OS error text, staging path, artifact bytes, and transcript-like content do not appear in the public message;
- the failed chunk is not counted or incorporated into the verifier's successful byte/hash state;
- progress does not report the failed chunk as fully written;
- partial staging data and the staging directory are removed;
- no canonical model or marker exists after a new-install failure.

### 4.3 Finalization failures

Inject `flush` failure and, if represented separately by the seam, `sync_all` failure. Assert they map to the documented sanitized staging-file finalization I/O error and skip hash/size success, promotion, and marker creation. The staging directory must be removed.

### 4.4 Existing-install safety

Preserve the existing verified-install fast path. A test must demonstrate that an injected writer/finalization failure is not reached when a verified installation already exists, and that the existing model and marker remain unchanged. Reuse existing coverage where it already proves the invariant rather than duplicating a broad installer test.

### 4.5 Test quality

- Use fake I/O behavior rather than full disks, permissions, `/dev/full`, platform-specific filesystems, sleeps, or real model downloads.
- Keep test-only types and constructors private to the Whisper installer module tree.
- Do not log fixture bytes, paths, or injected error details.
- Keep the existing installer success, cancellation, integrity, migration, symlink, lease, and promotion tests intact.

## 5. Non-goals

- Changing retryability or public error wording unless a test reveals an existing incorrect contract.
- Changing model size/hash pins, installer transport policy, staging layout, marker schema, or promotion behavior.
- Injecting every possible filesystem failure. Directory creation and marker/promotion errors have separate existing coverage.
- Adding real-model downloads to normal tests or CI.
- Requiring a broad filesystem abstraction for unrelated installer operations.

## 6. Acceptance criteria

1. Deterministic regressions reach production sink/install error handling for partial write failure and finalization failure.
2. The failures are classified as sanitized I/O errors and reveal no low-level error details.
3. Failed installs leave no staging artifact, canonical model, or ready marker; an existing verified install is preserved.
4. The production path still uses `std::fs::File`, exclusive staging-file creation, and the existing size/SHA-256 verification.
5. Focused Whisper installer tests and the repository Rust quality gate pass.

## 7. References

- `src-tauri/src/asr/whisper/installer/integrity.rs`
- `src-tauri/src/asr/whisper/installer/progress.rs`
- `src-tauri/src/asr/whisper/installer.rs`
- `src-tauri/src/asr/whisper/installer/tests.rs`
- `docs/ASR_UNIT_TEST_COVERAGE_TODO_2026-10-08.md`
