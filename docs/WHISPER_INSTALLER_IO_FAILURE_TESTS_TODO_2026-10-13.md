# Whisper Installer Staging I/O Failure Tests — TODO

**Date:** 2026-10-13<br>
**Status:** Complete<br>
**Specification:** `docs/WHISPER_INSTALLER_IO_FAILURE_TESTS_SPEC_2026-10-13.md`<br>
**Related coverage audit:** `docs/ASR_UNIT_TEST_COVERAGE_TODO_2026-10-08.md`

This checklist covers only deterministic failure injection for staging-file writes and finalization. Keep production model pins and installer security behavior unchanged. Mark an item complete only after its named source/test or gate exists and passes.

## WIF-100 — Add a private staging I/O test seam

- [x] Inspect `WhisperVerifyingFileSink`, `InstallerFileSink`, and the installer dependency constructor to identify the narrowest seam that can inject write, flush, and synchronization failures.
- [x] Keep production construction on `std::fs::File` with exclusive-create semantics and real flush/synchronization.
- [x] Keep the seam private to `src-tauri/src/asr/whisper/installer/` and avoid changing IPC, public APIs, manifest identity, or marker formats.
- [x] Add a deterministic fake that can write a prefix and then fail, and can fail during flush and synchronization independently where the seam supports those operations.
- [x] Preserve verifier accounting semantics: update successful byte count and SHA-256 only after a full chunk write succeeds.

**Acceptance:** tests can inject filesystem I/O outcomes without host-specific paths, permissions, disk exhaustion, or changes to production artifact identity.

## WIF-200 — Cover partial and immediate staging write failures

- [x] Add an installer-path regression where the writer accepts part of a chunk and then errors during `write_all`.
- [x] Add an immediate write-error case if it exercises a distinct mapping or cleanup boundary.
- [x] Assert `WhisperModelInstallErrorKind::Io` and the stable sanitized staging-write message.
- [x] Assert the injected error text, temporary path, and fixture bytes do not appear in the returned error.
- [x] Assert a failed chunk does not advance successful byte/hash accounting or report full-chunk progress.
- [x] Assert partial staging data and the staging directory are removed and no canonical model or marker is created.

**Acceptance:** an error after a partial physical write cannot be mistaken for a completed chunk or leave an installable artifact.

## WIF-300 — Cover staging finalization failures

- [x] Add a `flush` failure regression through the installer finish path.
- [x] Add a `sync_all` failure regression if it can be distinguished without widening the seam beyond the sink boundary.
- [x] Assert stable sanitized I/O classification and no low-level error/path leakage.
- [x] Assert size/hash success, model promotion, and marker creation do not occur after finalization fails.
- [x] Assert staging cleanup runs on each finalization failure.

**Acceptance:** every covered finalization I/O failure remains fail-closed and leaves no ready model state.

## WIF-400 — Preserve install lifecycle guarantees

- [x] Verify a verified existing model still short-circuits before the injected writer is constructed or called.
- [x] Assert the existing model and marker remain byte-for-byte unchanged.
- [x] Keep current success, transport, cancellation, integrity, migration, symlink, lease, and promotion coverage intact.
- [x] Review test doubles and assertions for deterministic synchronization and absence of private-data logging.

**Acceptance:** the new seam does not weaken existing installer ownership, cleanup, integrity, or no-replacement guarantees.

## WIF-900 — Run and record verification

- [x] Run focused Whisper installer tests covering WIF-200 and WIF-300.
- [x] Run `npm run check:rust` for formatting, Clippy, and the full Rust test suite.
- [x] Run `git diff --check`.
- [x] Update this TODO with the exact tests and results; preserve the documented limitation if a separate `sync_all` failure cannot be injected without an unjustified abstraction.

**Final acceptance:** the new tests pass through the production sink/install failure handling, the full Rust gate passes, and the recorded claims match the actual tests.

## Completion evidence

- Focused command: `CARGO_BUILD_JOBS=2 cargo test --manifest-path src-tauri/Cargo.toml --all-features asr::whisper::installer::tests::io_failures -- --nocapture` — 4 passed, 0 failed.
- Focused tests: `failed_partial_write_does_not_advance_verified_byte_accounting`, `flush_and_sync_failures_prevent_promotion_and_clean_staging`, `write_failures_are_sanitized_and_remove_partial_staging_state`, and `verified_install_short_circuits_before_injected_staging_io`.
- Full gate: `CARGO_BUILD_JOBS=2 npm run check:rust` — formatting and Clippy passed; 930 unit tests passed, 3 ignored, and 11 integration tests passed across the integration test binaries.
- `git diff --check` — passed.
- No model weights or network downloads were used.
