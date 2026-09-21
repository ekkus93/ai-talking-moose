# WWR-800 Wake CI Gate Inventory — 2026-09-21

## Scope

This evidence records the current Wake Word CI-gate state for WWR-800 after the merged Wake runtime snapshot privacy work.

It is an inventory and gap record. It does not claim that every WWR-800 gate is complete.

## Exact base

- Repository: `ekkus93/ai-talking-moose`
- Base commit: `0bb26753d4b3f1acffa9c6f5fe90dd5c6a115040`
- Relevant merged change: PR #338, `test(wake): prove runtime snapshot omits PCM payload`
- Exact PR-head CI before merge:
  - ordinary CI `35654320341` passed on `27961b516b8d09e05c4db7ec90a2a2d71f85d64e`;
  - Wake Word lifecycle stability `35654320418` passed on `27961b516b8d09e05c4db7ec90a2a2d71f85d64e`;
  - P21-P23 Rust Stability Acceptance `35654320396` was skipped by its configured condition.

## Existing Wake-specific gate

### Wake Word lifecycle stability

Workflow file: `.github/workflows/wake-word-lifecycle-stability.yml`

The workflow currently runs on `pull_request` and `push` to `master` when these Wake lifecycle/source paths change:

- `src-tauri/src/asr/wake_word_runtime.rs`
- `src-tauri/src/asr/wake_word_stability.rs`
- `src-tauri/src/app/wake_word_composition.rs`
- `src-tauri/src/app/wake_word_state.rs`
- `src-tauri/src/app/runtime_preferences.rs`
- `src-tauri/src/commands/wake_word.rs`
- `src-tauri/Cargo.toml`
- `src-tauri/Cargo.lock`
- `.github/workflows/wake-word-lifecycle-stability.yml`

The workflow runs:

```text
cargo test --manifest-path src-tauri/Cargo.toml --all-targets --all-features wake_word_stability -- --nocapture
```

This is an exact-head gate whenever its path filters are triggered by the feature branch. PR #338 triggered and passed it on the exact PR head, and the merged `master` push started the same workflow for exact merged SHA `0bb26753d4b3f1acffa9c6f5fe90dd5c6a115040`.

## Existing non-Wake specialized gate observed on master

The push for `0bb26753d4b3f1acffa9c6f5fe90dd5c6a115040` also started `ASR-015 Native Acceptance` run `35655168046`. That workflow is native ASR acceptance rather than Wake KWS acceptance. It must not be counted as the WWR-610 Linux real Wake KWS gate or as the WWR-620 macOS arm64 Wake KWS gate.

## WWR-800 gap inventory

### Covered or partially covered

- Repeated lifecycle stability gate: partially covered by `.github/workflows/wake-word-lifecycle-stability.yml` for Wake runtime/lifecycle source paths.
- Exact-head binding for that gate: covered when path filters trigger, because GitHub Actions run on the PR head SHA and Ralph merge uses an exact-head SHA guard.

### Still open

- Deterministic Wake corpus CI/validation gate.
- Linux real sherpa KWS acceptance gate for WWR-610.
- MacOS arm64 real sherpa KWS acceptance gate for WWR-620.
- Native packaging/architecture gate specifically tied to Wake KWS final qualification.
- Performance evidence gate/report policy for WWR-630.
- Policy ensuring required Wake gates are not silently treated as passed when skipped.
- Documentation of which Wake gates require specialized runners/hardware.
- Final merge eligibility checks that require the applicable Wake-specific gates in addition to ordinary CI.

## Current conclusion

The repository now has a source-backed repeated lifecycle stability gate that can qualify Wake lifecycle/runtime changes, but WWR-800 remains incomplete because real Wake KWS corpus, platform acceptance, performance, skip-policy, and final merge-eligibility gates are not yet implemented.
