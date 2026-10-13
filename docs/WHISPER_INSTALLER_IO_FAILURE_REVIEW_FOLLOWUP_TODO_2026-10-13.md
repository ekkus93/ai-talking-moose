# Whisper Installer I/O Failure Review Follow-up — TODO

**Date:** 2026-10-13<br>
**Status:** Complete<br>
**Specification:** `docs/WHISPER_INSTALLER_IO_FAILURE_REVIEW_FOLLOWUP_SPEC_2026-10-13.md`<br>
**Original TODO:** `docs/WHISPER_INSTALLER_IO_FAILURE_TESTS_TODO_2026-10-13.md`

Mark each item complete only after the named change or verification has been observed.

## WIFR-100 — Make the partial-write fixture deterministic

- [x] Change the fake writer to commit the exact intended prefix before reporting a short write, propagating any real file I/O error.
- [x] Preserve the next-call injected failure and assert both the two write calls and the exact on-disk prefix.
- [x] Keep the fake independent of permissions, disk pressure, sleeps, and platform-specific devices.

**Acceptance:** the test does not rely on a single `File::write` call writing all three prefix bytes.

## WIFR-200 — Strengthen failed-chunk accounting evidence

- [x] Retain the zero successful-byte assertion for a failed partial write.
- [x] Add a direct SHA-256-state assertion with an independently computed expected digest; use only a test-scoped observation if access is needed.
- [x] Retain the installer-path assertion that the failed chunk does not emit completed-chunk progress and staging is removed.
- [x] Keep production installer behavior fail-closed after a sink write error.

**Acceptance:** the test would fail if the verifier hashed any bytes from the failed chunk.

## WIFR-300 — Strengthen finalization evidence

- [x] Capture progress events for both injected `flush` and `sync_all` failures.
- [x] Assert neither failure emits a `Verifying` event or a successful install outcome.
- [x] Retain distinct flush/sync call counts, sanitized I/O errors, staging cleanup, and absence of a canonical model and marker.
- [x] Review the finish-path order and add a focused error-precedence assertion to prove finalization occurs before SHA validation.

**Acceptance:** a finalization failure cannot appear as successful verification or a ready model in the test evidence.

## WIFR-900 — Correct and verify the record

- [x] Correct the original TODO's integration-test count from 10 to 11 and keep the unit/ignored counts accurate.
- [x] Run the focused Whisper installer failure tests.
- [x] Run `CARGO_BUILD_JOBS=2 npm run check:rust`.
- [x] Run a whitespace check that includes newly created files as well as tracked edits, then inspect the final diff and status.
- [x] Record exact test and gate results in this TODO; update its status only when all items pass.

**Final acceptance:** the tests prove the review findings are resolved and the documentation reports only observed results.

## Completion evidence

- The focused command `CARGO_BUILD_JOBS=2 cargo test --manifest-path src-tauri/Cargo.toml --all-targets --all-features asr::whisper::installer::tests::io_failures -- --nocapture` passed: 4 passed, 0 failed.
- `CARGO_BUILD_JOBS=2 npm run check:rust` passed formatting, Clippy, and the full Rust suite: 930 library tests passed, 3 ignored; 11 integration tests passed and the live Google API test remained ignored.
- `git diff --check` passed for tracked changes. A separate whitespace check including the new Rust test and both review follow-up Markdown files passed; the original TODO count was corrected to 11 integration tests.
- `cargo fmt --manifest-path src-tauri/Cargo.toml -- --check` passed after the code changes.
