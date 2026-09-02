# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

@AGENTS.md

## Verification

`npm run check:all` is the canonical ordinary pre-conclude command (frontend: generated-tree/IPC/typecheck/lint/format/test/build; backend: `cargo fmt --check` + `cargo clippy -D warnings` + `cargo test`, followed by generated backend-contract drift/shape validation). Run it, or the relevant half while iterating, before concluding changes. See `AGENTS.md` for exact commands and the additional Local LLM packaging gate.

The Gemini live-API integration test (`test_gemini_live_asr`) is `#[ignore]`d and gated behind `TALKING_MOOSE_ALLOW_LIVE_API=1` plus a real `TALKING_MOOSE_GOOGLE_API_KEY`. Default test runs must stay offline with respect to live Google APIs.

Real Local LLM acceptance is also **not** an ordinary test. Do not add GGUF downloads to `npm run check:all` or normal CI. `.github/workflows/local-llm-real-cpu-acceptance.yml` is the explicit model-download/real-CPU evidence path; `.github/workflows/local-llm-p13-packaging-acceptance.yml` is the explicit two-architecture bundle-impact path. See `AGENTS.md` and `docs/LOCAL_LLM_ARCHITECTURE.md`.

## Privacy constraints (`docs/PRIVACY.md`)

This project is privacy-by-design; treat these as hard constraints, not suggestions:

- No silent escalation from local Moonshine ASR to cloud Gemini Live microphone upload.
- No silent fallback from Local text generation to Google or Fake.
- Selecting a Local ASR/LLM model does not authorize a download; model installation is an explicit user action.
- Fresh-profile privacy defaults remain Off for active-app observation, memory, and transcript retention.
- Fresh-profile text generation defaults to Local with SmolLM2 selected but not automatically installed.
- Logs must never contain credentials, transcripts, prompts/model output, PCM audio, memory text, or private Local-model paths/raw native errors.
- The Google API key is stored via OS keychain/secure storage, never SQLite or committed configuration.
- Local text does not imply fully local voice: Gemini Live remains the V1 realtime voice provider, and Google TTS remains cloud-backed when used.

## Phased delivery workflow

Work is tracked as numbered phases against `docs/SPEC(*).md` and `docs/TODO(*).md`, with reconciliation files under `docs/` recording evidence. Local LLM work currently spans P0–P15; P15 is the final reconciliation/gate. A task is complete only when its required implementation and acceptance evidence actually exists. Manual/physical/real-model checks stay open until run and inspected rather than being inferred from ordinary CI.

When phase work changes documentation or behavior, update the relevant TODO/reconciliation artifacts so they do not contradict the product. The `/phase-gate-check` skill may help with the audit, but a generated checklist is not evidence by itself.

## Repo conventions

- Commit messages follow Conventional Commits (`fix:`, `feat:`, `test:`, `chore:`, `docs:`, `ci:`, `style:`, `build:`), often prefixed by the phase/component in the message body or subject.
- `node_modules/` and `dist/` are intentionally ignored and must **not** be committed. `npm run check:generated-trees` fails when generated frontend trees are present in the Git index.
- `.gguf`/`.GGUF` model weights are intentionally ignored and must **not** be committed or bundled. Use the catalog + explicit production installer and the manual real-model acceptance workflow instead.
- Never weaken a fail-closed policy/checker merely to get CI green. If a packaging, privacy, provider, integrity, or generated-contract gate fails, fix the drift or explicitly revisit the requirement.
