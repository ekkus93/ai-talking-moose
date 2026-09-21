# WWR-510 — Wake Word Error and Log Privacy Audit Evidence

**Date:** 2026-09-21
**Scope:** Audit Wake Word V1 diagnostics, production errors, and production logging surfaces for credentials, unnecessary paths, and audio content.

## Change

`scripts/check_wake_word_privacy_audit.mjs` now audits production Wake Word Rust files in addition to the existing diagnostics/docs/corpus checks. The audit:

- scans production Wake Word Rust source outside `#[cfg(test)] mod tests`;
- fails if production Wake Word code introduces direct logging macros or tracing references;
- fails if production Wake Word error/log string literals include credential/secret/API-key terms;
- fails if production Wake Word error/log string literals include transcript/raw-audio/audio-content terms;
- fails if production Wake Word error/log string literals include explicit absolute/file-path field terminology;
- continues to require sanitizer evidence for native error paths using `<path>` and `<redacted>` placeholders.

## Privacy boundary

The audit is intentionally conservative around outward-facing strings and logging surfaces. It does not forbid internal path objects required to load pinned native artifacts; it verifies that production logs are absent and outward-facing error strings do not expose credentials, transcripts, raw audio content, or file-path details.

## TODO reconciliation note

This closes WWR-510's explicit error/log audit items for credentials, unnecessary absolute paths, and audio content. The optional CPU/memory/real-inference timing portion remains open until representative acceptance environments are measured.
