# WSP-300 — Whisper-only UI and diagnostics evidence (2026-10-16)

## Exact source and CI

- Repository: `ekkus93/ai-talking-moose`; code head: `ba6f871acd496db4f6fa584e32852c9f2834ea38`.
- CI: `38893061667` (GitHub Actions CI, push to master, exact code SHA above), completed **success**. The `Frontend quality` job passed. This scoped run **skipped** Rust tests, native macOS bundles, generated contract and canonical full-check jobs; those are not claimed here.
- Earlier two-architecture pinned-model native CPU transcription prerequisite: `999a04e3aefaa9b93f30ad0c53b188009f9f228c`, run `38886320925`, arm64 and Intel jobs passed. This is not packaged-app or physical-microphone acceptance.

## What was verified

- `src-tauri/src/commands/asr_models.rs` returns only the Whisper Small model descriptor for Linux and macOS and rejects attempted installation/deletion of retired Moonshine modes; model state comes from the production Whisper installer.
- `src/components/Settings/AsrSettingsPanel.tsx` offers Whisper Small local ASR and explicitly selected Gemini Live Cloud Audio, not active Moonshine choices. The Whisper installer remains an explicit user action, with cancellation and integrity/progress states.
- `src/components/Onboarding/OnboardingModal.tsx` explains local batch recognition versus cloud audio and offers an optional, explicitly initiated Whisper model download without blocking onboarding.
- `src/components/Settings/AsrDiagnosticsPanel.tsx` now identifies an active **local ASR session** rather than misleadingly claiming true streaming, and explains batch partial inference and process-level metrics without stale Moonshine language.
- `src/test/AsrSettingsPanel.accessibility.test.tsx` now uses a Whisper-only catalog fixture, asserts exactly two ASR radio selections (Whisper local and Gemini Live cloud), and retains model progress and cancel coverage. The frontend quality suite passed on the exact code SHA.
- `src/generated/backendContract.json` was synchronized to settings schema version 9; earlier generated-contract drift at `b139f470384d17f310638418681151ee0021369d` was corrected by `869d5fe50a0aea6611800036bc1682929edb234b`.

## Open acceptance boundaries

This evidence closes only the **WSP-300 UI/onboarding choice and disclosure checkbox**. It does not prove settings migration on clean Macs, actual Whisper app inference, offline egress enforcement, real microphone/wake-word handoff, provider/TTS routing, native packaging, KittenTTS parity or WSP-500 Moonshine deletion. Moonshine runtime code remains pending coordinated removal after WSP-300/400. Do not mark those gates complete on this frontend-only run.
