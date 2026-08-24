# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

@AGENTS.md

## Verification

`npm run check:all` is the canonical pre-conclude command (frontend: typecheck/lint/format:check/test/build; backend: `cargo fmt --check` + `cargo clippy -D warnings` + `cargo test`, both against `src-tauri/Cargo.toml`). Run it (or the relevant half) before concluding changes — see `AGENTS.md` for the exact commands.

The Gemini live-API integration test (`test_gemini_live_asr`) is `#[ignore]`d and gated behind `TALKING_MOOSE_ALLOW_LIVE_API=1` plus a real `TALKING_MOOSE_GOOGLE_API_KEY`. Default test runs (including CI) must stay offline w.r.t. Google — never remove that gate or make a test contact live Google APIs by default.

## Privacy constraints (docs/PRIVACY.md)

This project is privacy-by-design; treat these as hard constraints, not suggestions:
- No silent escalation from local Moonshine ASR to cloud Gemini Live.
- Fresh-profile defaults are all Off (active-app observation, memory, transcript retention).
- Logs must never contain credentials, transcripts, PCM audio, or memory text.
- The Google API key is stored via OS keychain/secure storage, never in SQLite or a `.env` file.

## Phased delivery workflow

Work is tracked as numbered phases (P0–P13 so far) against `docs/SPEC(*).md` and `docs/TODO(*).md`, with `docs/RECONCILIATION_P<N>_<date>.md` files recording per-phase acceptance-criteria audits. A phase task is complete only when all its required implementation and acceptance items are checked off — manual/physical (e.g. macOS hardware) checks stay open until actually run, not assumed. When doing phase-related work, check the relevant TODO/SPEC doc and update or add a RECONCILIATION doc to match (see recent ones under `docs/` for the expected format). The `/phase-gate-check` skill automates the audit half of this.

## Repo conventions

- Commit messages follow Conventional Commits (`fix:`, `feat:`, `test:`, `chore:`, `docs:`, `ci:`, `style:`, `build:`), often prefixed with the phase/component (e.g. `fix: scope P13 license inventory to shipped dependencies`).
- `node_modules/` and `dist/` are committed to git despite `.gitignore` existing — this looks like an oversight, not a convention. Don't assume it's intentional; don't "fix" it unprompted either.
