# WWR-640 — lifecycle stability exact-head evidence

Date: 2026-09-24
Base master before this evidence slice: `dc346a58536f41679529c27823e795f4a12a8c19`

## Scope

This evidence slice binds the Wake Word lifecycle stability workflow to current remediation evidence changes by adding `docs/evidence/WWR-640_*.md` to its path filters. The lifecycle workflow itself still runs the production-adjacent deterministic Rust stability suite:

```bash
cargo test --manifest-path src-tauri/Cargo.toml --all-targets --all-features wake_word_stability -- --nocapture
```

## Covered by the suite

`src-tauri/src/app/wake_word_lifecycle_stability_tests.rs` covers:

- repeated wake → interaction → wake cycles returning to Listening while ring and handoff buffers stay bounded;
- successful TTS, cancelled TTS, and recoverable TTS failure all resolving through the same terminal-interaction resume policy;
- repeated disable/enable cycles returning through Disabled → Loading → Listening without retained stale audio;
- shutdown while Listening clearing retained audio and becoming terminal;
- shutdown during triggered handoff clearing pre-roll and rejecting resume.

## Boundaries

This is deterministic integrated lifecycle evidence for the repository-supported stability suite. It does not establish WWR-630 performance baseline measurements or final WWR-950/960 closeout by itself.

The PR carrying this file must qualify with exact-head ordinary CI and Wake Word lifecycle stability. After merge, exact-master CI/lifecycle evidence must be recorded before final reconciliation.
