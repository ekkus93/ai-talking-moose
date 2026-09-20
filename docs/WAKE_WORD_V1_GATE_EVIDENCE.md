# Wake Word V1 Gate Evidence

This file records exact-head Wake Word gate evidence without converting partial evidence into final acceptance. Final qualification remains governed by `WAKE_WORD_V1_REMEDIATION_TODO_2026-09-17.md` and `WAKE_WORD_V1_CI_GATES.md`.

## Specialized gate inventory

| Gate | Workflow / checker | Current status | Exact evidence |
| --- | --- | --- | --- |
| Deterministic corpus manifest | `.github/workflows/wake-word-corpus.yml` / `scripts/check_wake_word_corpus_manifest.mjs` | Implemented; real-fixture calibration remains pending | master run `35482458468` passed at `1a49663add48280811f8a749e1928abcbce8cdd9` |
| Privacy-safe diagnostics/source audit | `.github/workflows/wake-word-privacy-audit.yml` / `scripts/check_wake_word_privacy_audit.mjs` | Implemented source/privacy gate; not a substitute for the complete WWR-900 audit | PR-head run `35484525190` passed at `485e262dab443f1ad5cff32e1337926572093206`; master run `35484570811` passed at `aa217eb4e6891ab8bfd46c3f73c92b47a8cd8623` |
| Lifecycle stability | `.github/workflows/wake-word-lifecycle-stability.yml` | Implemented targeted deterministic stability gate; full production audio/lifecycle acceptance remains pending | PR-head run `35485188900` passed at `241da9fa19fb6147113dfc40d42b4e3572de9127` |
| Linux x86_64 real KWS | pending | Not implemented/accepted | none |
| macOS arm64 real KWS | pending | Not implemented/accepted | none |
| Native package/architecture acceptance | pending | Not fully implemented/accepted | prerequisite manifest/package checks exist, but no final acceptance evidence is claimed here |
| Performance evidence | pending | Not implemented/accepted | none |

## Lifecycle gate scope

The lifecycle workflow runs the repository's existing `wake_word_stability` Rust tests with live API access disabled and the same Linux/Tauri build dependencies used by ordinary Rust CI. Its purpose is to keep deterministic runtime-manager stability invariants exact-head bound.

A passing lifecycle workflow does **not** prove the still-pending integrated production scenarios: repeated real wake→ASR→Thinking→Talking→wake cycles, real capture-stream ownership under native audio, native-session resource growth, representative soak behavior, or platform-specific real KWS inference.

## Qualification rules

- Ordinary CI alone is never final Wake Word V1 qualification.
- A skipped specialized workflow is not acceptance evidence.
- Evidence is bound to the exact SHA named with the run.
- Real KWS platform acceptance requires real positive and negative redistributable fixtures plus exact artifact/runtime verification.
- Final closeout must refresh this matrix with final PR-head and merged-master runs; historical passing runs cannot qualify a changed final head by themselves.
