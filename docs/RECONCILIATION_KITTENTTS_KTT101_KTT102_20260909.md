# AI Talking Moose — KittenTTS KTT-101 / KTT-102 Reconciliation

**Tracker:** `docs/TODO(20260909-120003).md`
**Tasks:** `KTT-101 — Split standalone Google, Local, and Gemini Live voice settings`; `KTT-102 — Implement deterministic settings v3 → v4 migration`
**Implementation base:** `f69a3a1ceab3c1ef2d8621ce577f5dfcb4133a2f`
**Final implementation head:** `af92898bfdb38582e389d6bda02ca57ef7142bb7`
**Implementation PR:** #75
**Merged implementation master:** `4d72b305f7a6c7e9b0b7414975cb44c888daa4d1`
**Status:** KTT-101 / KTT-102 COMPLETE — exact-head CI, guarded merge, and exact merged-master CI accepted 2026-09-09

## Implementation

KTT-101 advances the persisted settings schema to version 4 and removes the overloaded standalone/Live TTS ownership model. The authoritative settings now carry independent fields for:

- `tts_provider`;
- `google_tts_model`;
- `google_tts_voice`;
- `local_tts_model`;
- `local_tts_voice`;
- `live_voice`;
- existing `speaking_rate`;
- existing `pitch`.

Legacy persisted `tts_model` and `tts_voice` are no longer authoritative v4 fields.

Runtime ownership is split accordingly:

- Google standalone synthesis reads `google_tts_model` and `google_tts_voice`;
- Local standalone synthesis owns `local_tts_model` and `local_tts_voice`;
- Gemini Live reads `live_voice`;
- changing `live_voice` is conversation-restart-sensitive;
- changing Google or Local standalone voices does not restart the active Live graph.

The Local provider is deliberately fail-closed until the production Kitten runtime lands: selecting `tts_provider = local` returns a local setup error through `PendingLocalSpeechSynthesizer` and cannot silently route the utterance to Google.

KTT-102 implements deterministic v3 → v4 migration. Existing v3 profiles:

- remain on `TtsProvider::Google` so upgrades preserve current standalone behavior;
- migrate `tts_model` to `google_tts_model`;
- copy legacy `tts_voice` to both `google_tts_voice` and `live_voice`;
- initialize `local_tts_model` to `KittenML/kitten-tts-mini-0.8`;
- initialize provisional `local_tts_voice` to `Bella` pending the later human voice-acceptance task;
- normalize to v4 once;
- retain the existing future-version fail-closed policy without rewriting unsupported newer settings.

Focused Rust tests cover v4 defaults, one-shot v3 migration, independent voice ownership, removal of legacy fields, restart semantics, local catalog validation, and fail-closed Local provider behavior. The Rust-generated frontend contract and TypeScript `AppSettings` shape were regenerated in the same atomic compatibility change.

The settings/contract work included here satisfies only the AppSettings compatibility portion needed by KTT-104. KTT-104 remains open until provider/model/voice metadata, shape-gate coverage, dispatcher fixtures, and any command-registration work are complete.

## Acceptance evidence

PR #75 was reduced to one clean commit on implementation base `f69a3a1ceab3c1ef2d8621ce577f5dfcb4133a2f`.

Exact PR head:

`af92898bfdb38582e389d6bda02ca57ef7142bb7`

Exact-head ordinary CI:

- run `34419447271`: PASS;
- event: `pull_request`;
- head SHA: `af92898bfdb38582e389d6bda02ca57ef7142bb7`.

PR #75 was open, non-draft, mergeable, and still pointed at that exact head immediately before merge. It was squash-merged with `expected_head_sha` guarding the validated head.

Resulting master:

`4d72b305f7a6c7e9b0b7414975cb44c888daa4d1`

Exact post-merge master CI:

- run `34420044908`: PASS;
- event: `push`;
- head SHA: `4d72b305f7a6c7e9b0b7414975cb44c888daa4d1`;
- substantive gates passed: Rust tests, Rust quality/Clippy, frontend quality, and generated backend contract verification.

Therefore every KTT-101 and KTT-102 tracker item and acceptance criterion is satisfied. KTT-103 remains the next dependency-ready implementation task, while KTT-104 remains open for its full metadata/contract scope.
