# WWR-000 Remediation Baseline Evidence

**Recorded:** 2026-09-17
**Remediation baseline reviewed:** `9f5d90b4b15f16c1ef2d473e574537392c310f0b`
**Remediation-plan merge:** `5946bedcff37e1ba9558423447e50adf563e8c00`
**Exact-master ordinary CI:** `35249670511` — success

## Scope

WWR-000 freezes the starting point for the Wake Word V1 remediation campaign and verifies that the remediation-plan merge itself changed documentation only.

The reviewed implementation baseline is commit `9f5d90b4b15f16c1ef2d473e574537392c310f0b`. Commit `5946bedcff37e1ba9558423447e50adf563e8c00` is a descendant that adds only:

- `docs/WAKE_WORD_V1_REMEDIATION_SPEC_2026-09-17.md`
- `docs/WAKE_WORD_V1_REMEDIATION_TODO_2026-09-17.md`

Therefore the remediation-plan merge does not modify runtime source, settings schema, ASR provider behavior, TTS provider behavior, manual-listen behavior, `PcmRingBuffer`, or Wake Word artifact verification.

## Frozen invariants

The remediation campaign preserves the following until a later explicit remediation task changes the relevant Wake Word implementation under test:

- Wake Word remains disabled by default.
- Existing manual-listen behavior remains authoritative while Wake Word is disabled.
- Existing ASR provider selection and fallback behavior remains unchanged.
- Local/Google/Gemini TTS separation remains unchanged.
- Existing Local TTS production thread policy remains unchanged.
- V1 continues to exclude barge-in.
- V1 continues to exclude acoustic wake-phrase trimming.
- The existing generic bounded `PcmRingBuffer` behavior and tests remain part of the baseline.
- Wake Word model/runtime artifact handling remains fail-closed until immutable production identities are populated.

## Qualification

Exact merged-master ordinary CI run `35249670511` completed successfully on `5946bedcff37e1ba9558423447e50adf563e8c00`.

No source change is made by WWR-000 itself. Subsequent remediation tasks must independently prove that their source changes preserve unrelated ASR/TTS/manual behavior.
