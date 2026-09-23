# WWR-800 — Conversation-path Wake gate coverage

Date: 2026-09-22

## Scope

The Wake Word lifecycle and source/security workflows previously watched the dedicated Wake modules and `src-tauri/src/commands/conversation/**`, but the normal command-ASR implementation also lives in `src-tauri/src/conversation/session.rs` and `src-tauri/src/conversation/session/**`.

That left a path-filter hole: a future Wake→normal-command-ASR integration change in the conversation session layer could receive ordinary CI without automatically exercising the Wake lifecycle/security gates.

## Remediation

Both Wake-specific workflows now include:

- `src-tauri/src/conversation/session.rs`
- `src-tauri/src/conversation/session/**`
- `src-tauri/src/commands/conversation/**`

for both pull-request and master-push triggers.

This is deliberately broader than the eventual Wake handoff patch so future changes to the normal command-ASR lifecycle cannot silently bypass the Wake-specific gates merely because they are implemented on the conversation side of the boundary.

## Acceptance boundary

This change closes only the path-filter coverage hole. It does **not** claim that WWR-800 as a whole is complete, and it does not treat a skipped specialized workflow as a pass. Corpus, native Linux/macOS KWS, packaging, lifecycle acceptance, performance reporting, and final exact-head merge-eligibility policy remain subject to their own objective qualification requirements.
