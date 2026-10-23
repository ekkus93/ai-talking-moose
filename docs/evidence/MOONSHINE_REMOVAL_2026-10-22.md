# Moonshine Removal — Owner Override (2026-10-22)

The owner explicitly instructed: “I'll override that. Remove Moonshine.” This supersedes the original WSP-200/300/400 prerequisite for deleting Moonshine. It does not mark the outstanding performance, physical-device, or release acceptance checks passed.

## Implementation

- Removed the Moonshine native wrapper, FFI, model manifests, installer, engines, factories, app state, conversation request fields, runtime probes, architecture variants, and dedicated legacy benchmarks/Miri harness.
- Removed Moonshine build/link configuration, native provenance manifest, preparation/validation scripts, Tauri frameworks, legacy acceptance workflow, and generated notice/runtime staging from the shipping path.
- Retained only `whisper_small` and explicit `gemini_live_audio` as current ASR modes. Both historical Moonshine selectors still migrate when reading persisted settings. New IPC writes reject those strings during decoding; migration never downloads weights or changes selected text/TTS providers.
- Updated browser previews, contract representatives, frontend types, Wake Word policy/disclosures, release documentation, and provider-neutral regression fixtures to use Whisper. The production Whisper queue remains 56 chunks.
- Replaced native preparation with `scripts/prepare_macos_notices.sh`. Bundle verification now requires static Whisper's actual native probe and license notice and rejects legacy libraries/resources and embedded ASR weights.
- Ported the opt-in KittenTTS ASR roundtrip/voice audition to Whisper Small. The independent KittenTTS `ort` dependency, runtime installer, pinned archives, model, voices, and dictionary are retained.

## Remaining Moonshine References

Retained references have these purposes:

1. Historical settings strings and migration/IPC rejection tests.
2. Negative assertions that retired UI choices, libraries, and resources are absent.
3. Cleanup of stale generated staging resources and Git ignore entries preventing their accidental inclusion.
4. The manual Local LLM P13 workflow's fixed historical baseline: its isolated old checkout must use that revision's own preparation scripts. The current bundle uses Whisper.
5. Archived documentation, dated plans, incident records, historical voice audition results, and evidence. Moonshine-specific guides are explicitly labeled historical. The original migration plan records the owner override.
6. The reference-inventory script, which intentionally searches for obsolete names.
7. `AGENTS.md` and `memory.md`: existing contributor instructions/historical handoff retained. The earlier instruction to leave an existing `AGENTS.md` unmodified still applies; its old ASR architecture description is superseded by the current implementation and owner instruction.

## Verification

Frontend lint/typecheck/format, all 195 Vitest tests, and the frontend build passed with `VITEST_MAX_WORKERS=2`. An initial concurrent frontend run had one browser-preview timeout; the complete rerun passed.

Whisper provenance, macOS acceptance-workflow policy, release metadata, Local LLM packaging policy, Local TTS packaging policy, Wake Word documentation audit, dependency-license collection, 44 script policy tests, 29 repository Python tests, workflow syntax validation, and affected shell syntax checks passed. Local LLM and KittenTTS license dependencies remain present in the generated inventory.

`CARGO_BUILD_JOBS=2 CARGO_INCREMENTAL=0 npm run check:rust` passed Rustfmt, Clippy with all targets/features, 1,071 unit tests, and every integration/helper target. The generated backend contract was regenerated and its drift/shape gate passed. Failure and stress matrix discovery passed for 25 and eight scenarios respectively. Native macOS build/bundle results and real KittenTTS-to-Whisper voice acceptance must be recorded separately; earlier Whisper acceptance is bound to its original SHA.

The measured Intel RTF above 1 remains an open performance limitation. Physical microphone/playback, sustained thermal behavior, and signed/notarized release acceptance remain open.

## Source and CI follow-up

Removal implementation is committed as `f9d7310cf1a4c38d7c85180211eb48b2136d0a36`. Its Wake Word source security, required-gates, and documentation audits passed. The privacy audit exposed pre-existing paths to symbols moved into `wake_word_engine/types.rs` and `wake_word_acceptance/performance.rs`; the follow-up updates the audit to inspect current modules and recognize `tests.rs` as test code. All original sanitizer/privacy assertions remain enforced, and the corrected audit passes locally. macOS native/app and KittenTTS acceptance runs are still queued or running at this observation.
