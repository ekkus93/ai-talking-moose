# AI Talking Moose — KittenTTS KTT-100 Reconciliation

**Tracker:** `docs/TODO(20260909-120003).md`
**Task:** `KTT-100 — Add a typed standalone TTS provider`
**Implementation base:** `37b2c55f14d6e5ed54044ae962376146853b99a0`
**Final implementation head:** `81932ce0d888b50a333d1cff863e769c9d3d1b03`
**Implementation PR:** #73
**Merged implementation master:** `fda95f30563e3c0b7ebb7c9f594fb011b92ba283`
**Status:** KTT-100 COMPLETE — exact-head CI, guarded merge, and exact merged-master CI accepted 2026-09-09

## Implementation

KTT-100 adds an authoritative persisted standalone TTS provider type in `src-tauri/src/ai/types.rs`:

- `TtsProvider::Google`;
- `TtsProvider::Local`;
- deterministic snake_case serde values `google` and `local`;
- `Google` as the default, preserving current standalone TTS behavior when later settings persistence is introduced;
- no `Fake` persisted variant;
- independence from both `TextProvider` and `AsrMode`.

Focused tests prove round-trip serialization for every valid provider value and fail-closed deserialization for `fake` and an unknown provider value.

KTT-100 intentionally does not change the settings schema or runtime routing. Those responsibilities remain in KTT-101 and KTT-102 and later provider-routing tasks.

## Acceptance evidence

PR #73 was reduced to one clean commit on implementation base `37b2c55f14d6e5ed54044ae962376146853b99a0`.

Exact PR head:

`81932ce0d888b50a333d1cff863e769c9d3d1b03`

Exact-head ordinary CI:

- run `34412087596`: PASS;
- event: `pull_request`;
- head SHA: `81932ce0d888b50a333d1cff863e769c9d3d1b03`.

PR #73 was open, non-draft, mergeable, and still pointed at that exact head immediately before merge. It was squash-merged with `expected_head_sha` guarding the validated head.

Resulting master:

`fda95f30563e3c0b7ebb7c9f594fb011b92ba283`

Exact post-merge master CI:

- run `34413424816`: PASS;
- event: `push`;
- head SHA: `fda95f30563e3c0b7ebb7c9f594fb011b92ba283`.

Therefore every KTT-100 tracker item and acceptance criterion is satisfied. KTT-101 remains the next dependency-ready implementation task.
