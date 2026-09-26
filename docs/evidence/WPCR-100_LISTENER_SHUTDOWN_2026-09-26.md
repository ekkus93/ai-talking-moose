# WPCR-100 Listener Shutdown Evidence

**Date:** 2026-09-26
**Remediation item:** WPCR-100 — native listener control plane shutdown distinction.
**Merged master:** `dcc855cd16a89e37a6894ef217ba0532fc4f068b`

## Change summary

`src-tauri/src/app/wake_word_local_listener_thread.rs` now gives an intentional shutdown request priority over a simultaneous mock/real capture-close observation. The listener loop uses a biased shutdown branch and, if capture routing reports an error while a shutdown is concurrently pending, resolves the listener as `WakeLocalListenerEvent::Stopped` instead of recording a Wake capture error.

The listener-thread unit test now queues shutdown immediately after spawning the dedicated listener, then requires the privacy-safe event sequence `Started` followed by `Stopped` and requires the runtime phase to end as `Disabled`. The test continues to use a deliberately non-`Send` KWS engine fixture, proving the native KWS session remains local to the listener thread boundary and does not cross into the multithreaded Tauri executor.

## Evidence

- Exact PR head: `6da507f2f1c55d09539df6dbf854da91ba5cfa74`
- Exact-head ordinary CI: run `36235140401` passed.
- Exact-head Wake source-security audit: run `36235140426` passed.
- Merged master: `dcc855cd16a89e37a6894ef217ba0532fc4f068b`
- Exact-master ordinary CI: run `36235465363` passed.
- Exact-master Wake source-security audit: run `36235465362` passed.

## Scope boundary

This evidence closes the WPCR-100 requirement that intentional listener shutdown is distinct from capture failure and is tested without real audio hardware. It does not claim complete WPCR-100, WPCR-110, WPCR-120, WPCR-200, or final WPCR-950/960 closeout.
