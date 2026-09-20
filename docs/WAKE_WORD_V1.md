# Wake Word V1

Wake Word V1 is the local keyword-spotting subsystem for the fixed phrase **Hey, Moose**. This document describes behavior implemented on `master` and deliberately separates implemented behavior from acceptance work that is still open.

## User behavior

Wake Word is **disabled by default**. The normal Settings UI can enable or disable it and displays the fixed phrase `Hey, Moose`; V1 does not allow arbitrary wake-phrase editing or sensitivity controls.

When Wake Word is enabled, keyword spotting is designed to run locally using the pinned sherpa-onnx KWS model/runtime. The local microphone therefore remains active while the runtime is listening. Wake detection is not continuous cloud transcription. Command ASR remains a separate subsystem and follows the user's existing ASR/provider policy after an accepted wake interaction.

V1 intentionally does **not** provide wake-word barge-in while Moose is talking. Wake activation is suspended during Talking/TTS and resumes according to the application lifecycle when the interaction completes and Wake Word is still enabled.

The Wake Word ring/pre-roll buffer is memory-only application state. Diagnostics do not expose raw PCM, transcripts, credentials, or filesystem paths. The wake phrase and immediately following prompt may be replayed to the normal command-ASR path as part of wake-to-command handoff; V1 does not acoustically trim the wake phrase.

## Authoritative architecture

Production Wake Word ownership is consolidated under the Rust Wake Word application/runtime path. There is one authoritative `WakeWordRuntimeManager`, one frozen V1 KWS policy, and one application runtime owner. Command ASR providers are not Wake Word engines and must not instantiate a second independent Wake Word runtime.

The frozen V1 KWS policy is:

- 16,000 Hz mono PCM;
- feature dimension 80;
- one inference thread;
- keyword source `HEY MOOSE`;
- score 1.0;
- threshold 0.25;
- two seconds of pre-roll.

Model and native-runtime identities are pinned by the Wake Word artifact manifests and preparation/verification tooling. Consumed artifacts are hash checked and native runtime architecture is verified before use. Model and runtime licenses are tracked separately; do not infer a model license from the sherpa runtime's Apache-2.0 license.

## Settings and diagnostics

The Settings panel exposes enable/disable control, the fixed phrase, local/offline and active-microphone disclosures, and runtime status/help text. Live setting changes are validated before persistence and applied to the application Wake Word runtime without requiring an application restart when the transition is safe.

Privacy-safe diagnostics include enabled/runtime state, model/runtime identity, platform/architecture, one-thread policy, canonical audio policy, ring capacity, threshold/score, trigger count, bounded last-trigger age, initialization duration, Talking suspension, and a sanitized last error. Raw audio is intentionally not representable in the diagnostics payload.

## Artifact provenance and licenses

The authoritative model/runtime identity and provenance records are maintained in the repository's Wake Word artifact manifests and evidence under `docs/evidence/`. Runtime preparation verifies immutable byte sizes and SHA-256 identities before native loading. Attribution/notices for the selected model and sherpa runtime must remain consistent with those frozen records.

## Corpus and CI

`docs/wake-word-corpus.json` defines the deterministic Wake Word corpus contract. `scripts/check_wake_word_corpus_manifest.mjs` validates its schema, fixed phrase/audio policy, required labels, fixture provenance/license metadata, path containment, hashes, expected outcomes, and versioned acceptance-criteria policy. `.github/workflows/wake-word-corpus.yml` runs that manifest gate for relevant pull-request/master changes.

The corpus manifest currently records `pending_real_fixture_calibration`. That is intentional: the repository must not claim positive recall, false-accept performance, or platform KWS acceptance until real redistributable positive and negative fixtures have been measured and the corresponding acceptance evidence has passed.

## Current acceptance limits

Do **not** treat component/unit tests, artifact identity verification, or the corpus-manifest gate as proof of real acoustic Wake Word performance. The remediation TODO still requires real positive/negative KWS fixtures, Linux x86_64 and macOS arm64 real KWS acceptance, integrated microphone/handoff acceptance, lifecycle stability acceptance, and a reproducible performance baseline before final Wake Word V1 closeout.

Supported-platform claims for Wake Word must therefore be limited to platforms with completed real KWS acceptance evidence. Packaging/runtime identity support alone is not an acoustic-support claim.

## Troubleshooting

If Wake Word does not enter Listening, first inspect the Settings runtime status and privacy-safe Wake Word diagnostics. Artifact/runtime identity or architecture failures are fail-closed and should surface as sanitized errors rather than falling back to cloud recognition or an unverified runtime.

If Wake Word is disabled, manual interaction remains the supported path. If a recoverable Wake Word error occurs, manual interaction must remain usable. A runtime that is suspended while Moose is talking should return to Listening only when Wake Word remains enabled; disabling during an interaction must result in Disabled rather than an unintended resume.

For implementation/qualification status, use `docs/WAKE_WORD_V1_REMEDIATION_TODO_2026-09-17.md` and the linked evidence files rather than inferring completion from this document.