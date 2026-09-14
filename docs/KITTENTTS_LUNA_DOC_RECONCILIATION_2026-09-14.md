# AI Talking Moose — KittenTTS Luna Documentation Reconciliation

**Date:** 2026-09-14
**Scope:** Documentation index for the PR #115 Luna default closeout reconciliation
**Base master:** `96ff21e17b02aa0e3b5d8d783bd09400297eaaa4`
**Selected V1 Local default:** `Luna`
**Decision basis:** owner-approved ASR proxy, not subjective human listening

This note records why the older KittenTTS closeout documents were updated after PR #115.

PR #114 added the automated ASR voice-audition proxy. PR #115 then accepted the objective ASR-proxy recommendation and changed the shipped Local KittenTTS default from `Bella` to `Luna`.

The reconciliation updates these older authority documents so they no longer contradict the current `master` state:

- `docs/KITTENTTS_CLOSEOUT_REMEDIATION_TODO_2026-09-12.md`
- `docs/KITTENTTS_LEGACY_TODO_RECONCILIATION_2026-09-12.md`
- `docs/KITTENTTS_CLOSEOUT_EVIDENCE_2026-09-12.md`

The intended status is:

- `KCR-330` / `KTT-805` is closed for V1.
- The selected V1 Local KittenTTS default is `Luna`.
- The evidence basis is owner-approved ASR proxy evidence.
- No subjective human listening claim is made.
- Future subjective listening can still override `Luna`, but that is a new decision, not remaining V1 closeout work.

Validation evidence from PR #115 remains authoritative:

| Context | SHA | Workflow | Run | Result |
| --- | --- | --- | ---: | :---: |
| PR #115 head | `4a8bf77e4803b7898c717a3cbe145f9df9600c75` | CI | `34853091162` | PASS |
| PR #115 head | `4a8bf77e4803b7898c717a3cbe145f9df9600c75` | KittenTTS production CPU acceptance | `34853091150` | PASS |
| PR #115 head | `4a8bf77e4803b7898c717a3cbe145f9df9600c75` | KittenTTS ASR intelligibility smoke | `34853091234` | PASS |
| PR #115 head | `4a8bf77e4803b7898c717a3cbe145f9df9600c75` | KittenTTS ASR voice audition | `34853091230` | PASS |
| PR #115 merged master | `96ff21e17b02aa0e3b5d8d783bd09400297eaaa4` | CI | `34854193904` | PASS |
| PR #115 merged master | `96ff21e17b02aa0e3b5d8d783bd09400297eaaa4` | KittenTTS production CPU acceptance | `34854193873` | PASS |
| PR #115 merged master | `96ff21e17b02aa0e3b5d8d783bd09400297eaaa4` | KittenTTS ASR intelligibility smoke | `34854193869` | PASS |
| PR #115 merged master | `96ff21e17b02aa0e3b5d8d783bd09400297eaaa4` | KittenTTS ASR voice audition | `34854193968` | PASS |
