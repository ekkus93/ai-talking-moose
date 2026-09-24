# WWR-800 — Conversation-path Wake gate coverage

Date: 2026-09-24

## Scope

The Wake Word lifecycle and source/security workflows must run for the normal command-ASR side of the Wake handoff boundary, not only for dedicated Wake modules. The command-ASR implementation includes `src-tauri/src/conversation/session.rs`, nested `src-tauri/src/conversation/session/**` modules, and `src-tauri/src/commands/conversation/**` command paths.

A future Wake-to-command-ASR integration change in the conversation session layer must not receive ordinary CI while bypassing Wake-specific lifecycle or source-security gates.

## Remediation

The Wake lifecycle stability workflow now includes these conversation-session paths for both pull-request and master-push triggers:

- `src-tauri/src/conversation/session.rs`
- `src-tauri/src/conversation/session/**`
- `src-tauri/src/commands/conversation/**`

The Wake source-security audit workflow now includes the same conversation-side boundary paths for both pull-request and master-push triggers.

## Acceptance boundary

This evidence closes the WWR-800 path-filter coverage hole only. It does not mark WWR-800 as fully complete by itself, and it does not treat skipped specialized workflows as a pass. Corpus, native Linux/macOS KWS, packaging, lifecycle acceptance, performance reporting, and final exact-head merge eligibility remain subject to their own objective qualification requirements.
