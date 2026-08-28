# Rust Lint and Thread-Safety Audit

**Scope:** `src-tauri/src/**` and `src-tauri/build.rs`  
**Audit date:** 2026-08-28

This audit records the V1R-004 lint-suppression and manual thread-safety review. It is intentionally narrower than the Moonshine FFI safety audit: V1R-004 is concerned with compiler/lint suppressions and manual `Send`/`Sync` assertions in ordinary application code.

## Lint suppressions

A repository scan of the production Rust tree finds no `#[allow(...)]` or `#[expect(...)]` attributes. There is therefore no production lint suppression currently masking a Clippy or compiler warning, and no suppression rationale to maintain.

The CI Rust gate remains authoritative for warning enforcement and runs Clippy with `-D warnings`.

## Manual thread-safety assertions

No manual production `unsafe impl Send` or `unsafe impl Sync` assertion remains in the ordinary Rust application tree.

V1R-004 previously identified two `SafeStream` wrappers around CPAL 0.15 `Stream`, one in capture and one in playback. They existed because CPAL 0.15 kept its cross-platform stream wrapper non-`Send`, requiring the application to assert desktop-only thread-safety manually.

The project now uses CPAL 0.17.3. CPAL 0.17 makes `Stream` `Send + Sync` across its supported platform implementations, so capture and playback store `cpal::Stream` directly and both application-owned `unsafe impl Send` shims have been deleted.

## Ownership and shutdown coverage

Removing the manual wrappers does not relax the application's ownership rules. Existing deterministic regressions continue to cover:

- capture start/stop state and runtime stream-error state clearing;
- playback `stop()` flushing queued samples and resetting negotiated output state;
- conversation lifecycle idempotent shutdown, provider close/error cleanup, application shutdown, mute/dismiss teardown, and stale-generation protection while audio resources are owned by the lifecycle manager.

Those tests now exercise CPAL's native stream thread-safety contract rather than an application-level unsafe assertion.

## V1R-004 result

The preferred removal plan from the 2026-08-22 audit is complete: CPAL was upgraded in a controlled migration with a resolver-generated lockfile, both `SafeStream` shims were removed, and the ordinary Rust/macOS CI matrix is the execution gate for the dependency/API change.

V1R-004 therefore leaves no unexplained production lint suppression and no manual unsafe thread-safety assertion in the audited ordinary Rust application scope.
