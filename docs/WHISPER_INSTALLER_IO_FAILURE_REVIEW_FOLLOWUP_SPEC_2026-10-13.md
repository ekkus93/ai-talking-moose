# Whisper Installer I/O Failure Review Follow-up — Specification

**Date:** 2026-10-13<br>
**Status:** Proposed<br>
**Companion TODO:** `docs/WHISPER_INSTALLER_IO_FAILURE_REVIEW_FOLLOWUP_TODO_2026-10-13.md`<br>
**Original work:** `docs/WHISPER_INSTALLER_IO_FAILURE_TESTS_SPEC_2026-10-13.md` and `docs/WHISPER_INSTALLER_IO_FAILURE_TESTS_TODO_2026-10-13.md`

## Purpose

Close the concrete test and documentation gaps found in the code review of the Whisper installer staging I/O failure tests. The production installer path was found to be sound; this work improves the precision of the test evidence and completion record.

## Requirements

### 1. Deterministic partial-write fixture

In `src-tauri/src/asr/whisper/installer/tests/io_failures.rs`, make the fake writer reliably commit the intended three-byte prefix before returning a successful short-write count. Use a complete write of that prefix inside the fake and propagate any actual file I/O error. Keep the next fake write call as the injected failure so the test still exercises `write_all` after a partial write. Assert the prefix and call count as before. Do not use disk exhaustion, permissions, sleeps, or platform-specific devices.

### 2. Prove failed-chunk verifier accounting

Retain the existing byte-counter and progress assertions. Add an assertion that detects whether a failed chunk entered the SHA-256 state, using a test-only observation or a focused verifier test that does not change production interfaces. The expected digest must be independent of the implementation under test. An installer must still abort after a write error; the test must not imply that continuing an installation after such an error is supported.

### 3. Prove finalization event ordering

Capture progress callbacks in both flush and sync failure cases. Assert that completed download bytes may be reported, but no `Verifying` phase or successful install outcome is reported after finalization fails. Retain the I/O error, call-order, staging cleanup, and no-promotion assertions. If useful, use mismatched expected verification data in a separate focused case to confirm that finalization failure takes precedence over size/hash validation without changing production verification order.

### 4. Correct verification evidence

Correct the integration-test count in the original TODO. The recorded full Rust gate had 11 passing integration tests across its integration binaries (3 + 4 + 1 + 2 + 1), plus 930 passing library unit tests and three ignored library tests. Ensure whitespace validation covers new files as well as tracked edits; `git diff --check` alone does not inspect untracked files. Record the commands and outcomes actually observed after implementation.

## Boundaries

- Keep the staging file factory private and keep production `std::fs::File`, exclusive creation, flush, sync, integrity checks, and installer cleanup behavior unchanged.
- Do not change public IPC, model pins, marker format, transport policy, or user-facing error messages.
- Keep ordinary tests independent of model weights, network downloads, and audio hardware.
- Run the Rust quality gate with Cargo limited to two jobs to preserve machine responsiveness.

## Acceptance criteria

1. The partial-write fake reliably writes the intended prefix, then fails on the next `write_all` call.
2. Tests directly establish that a failed chunk does not advance successful byte, SHA-256, or progress accounting.
3. Flush and sync tests establish that no `Verifying` progress or ready model state follows finalization failure.
4. The original TODO's verification count is accurate, and whitespace checking includes all new files.
5. Focused Whisper installer tests and `CARGO_BUILD_JOBS=2 npm run check:rust` pass.
