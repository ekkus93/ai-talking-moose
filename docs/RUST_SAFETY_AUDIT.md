# Rust Lint and Thread-Safety Audit

**Scope:** `src-tauri/src/**` and `src-tauri/build.rs`  
**Audit date:** 2026-08-22

This audit records the V1R-004 lint-suppression and manual thread-safety review. It is intentionally narrower than the Moonshine FFI safety audit: V1R-004 is concerned with compiler/lint suppressions and manual `Send`/`Sync` assertions in ordinary application code.

## Lint suppressions

A repository scan of the production Rust tree finds no `#[allow(...)]` or `#[expect(...)]` attributes. There is therefore no production lint suppression currently masking a Clippy or compiler warning, and no suppression rationale to maintain.

The CI Rust gate remains authoritative for warning enforcement and runs Clippy with `-D warnings`.

## Manual thread-safety assertions

Exactly two manual thread-safety assertions remain:

- `src-tauri/src/audio/capture.rs` — `unsafe impl Send for SafeStream`
- `src-tauri/src/audio/playback.rs` — `unsafe impl Send for SafeStream`

Both wrap CPAL 0.15 `Stream`. CPAL 0.15 keeps its cross-platform stream wrapper non-`Send` because some backends, notably Android AAudio, cannot safely move a stream between threads. Talking Moose V1 targets desktop macOS and Linux, and the wrappers are compiled only for those targets. The stream values are held behind application ownership/mutex boundaries rather than being concurrently accessed without synchronization.

The assertions are therefore deliberate compatibility shims, not blanket assertions applied to arbitrary audio state.

## Ownership and shutdown coverage

The retained stream wrappers are exercised through deterministic ownership/shutdown coverage:

- capture tests cover start/stop state and runtime stream-error state clearing;
- playback tests cover `stop()` flushing queued samples and resetting negotiated output state;
- conversation lifecycle tests cover idempotent shutdown, provider close/error cleanup, application shutdown, mute/dismiss teardown, and stale-generation protection while audio resources are owned by the lifecycle manager.

These tests guard the ownership assumptions that make the current CPAL 0.15 wrappers acceptable on the supported desktop targets.

## Removal plan

The preferred end state is to remove both manual `unsafe impl Send` assertions rather than preserve them indefinitely. Newer CPAL releases provide stronger native `Send`/`Sync` behavior on desktop backends, so the removal path is a controlled CPAL upgrade followed by deletion of both `SafeStream` shims.

That dependency upgrade must not be performed as a blind manifest edit. It requires a regenerated lockfile plus the full Rust format/Clippy/test/build matrix on macOS and Linux. Until that validated migration is performed, the two target-gated, documented shims remain explicit technical debt.
