# WWR-700 — architecture documentation truthfulness evidence

Exact merged master: `99cbbbc931baba0792f5c24ca5ea2d5be04c0114`.

Merged PR: #408, `docs(wake): define authoritative V1 architecture`.

This evidence is intentionally scoped to the architecture documentation slice only. It does **not** close the full WWR-700 section and does not claim completion for WWR-300, WWR-310, WWR-400, WWR-600/610/620/630/640, WWR-800, WWR-900, WWR-950, or WWR-960.

## Documentation updated

`docs/WAKE_WORD_V1_ARCHITECTURE.md` now documents the current authoritative Wake Word V1 design boundaries:

- one `WakeWordApplicationRuntime` in `AppState`;
- one canonical `WakeWordRuntimeManager`;
- one shared application microphone owner, `AppState::audio_capture`;
- `AuthoritativeWakeCaptureOwner` as the serializer around that shared capture owner;
- fixed local phrase **Hey, Moose**;
- disabled-by-default policy;
- local/offline KWS intent after verified artifacts are prepared;
- active local microphone behavior while listening;
- wake phrase plus prompt may reach command ASR in V1 because there is no acoustic trimming;
- no V1 barge-in;
- Talking/TTS suspension intent;
- memory-only ring/pre-roll/handoff behavior;
- privacy-safe diagnostics boundaries;
- source map to the authoritative modules.

## Truthfulness boundaries preserved

The architecture document explicitly states that lifecycle wiring and real acceptance remain tracked as open work in `docs/WAKE_WORD_V1_REMEDIATION_TODO_2026-09-17.md` and must not be read as evidence that unchecked acceptance items have passed.

It also preserves the audited boundary phrase: `component tests are **not** a substitute` for native-platform qualification.

The document avoids supported-platform overclaiming by stating that Linux x86_64 and macOS arm64 remain subject to their dedicated real-KWS acceptance tasks until those sections pass.

## Exact validation

Exact PR-head validation for #408:

- ordinary CI passed on `bd29530c6be04bd91e9f732b4fa74a9c24daffcf` in run `35861437397`;
- Wake Word documentation audit passed on `bd29530c6be04bd91e9f732b4fa74a9c24daffcf` in run `35861437353`;
- P21-P23 Rust Stability Acceptance was skipped for the docs-only PR in run `35861437421` and is not used as Wake V1 acceptance evidence.

Exact merged-master validation after #408:

- ordinary CI passed on `99cbbbc931baba0792f5c24ca5ea2d5be04c0114` in run `35861502538`;
- Wake Word documentation audit passed on `99cbbbc931baba0792f5c24ca5ea2d5be04c0114` in run `35861502559`.

## Remaining WWR-700 work

The following WWR-700 work remains open until independently reconciled with objective evidence:

- full user-facing README/user docs update when the feature becomes usable;
- exact model/runtime provenance/license documentation audit across user and developer surfaces;
- supported-platform documentation based only on real Linux/macOS acceptance;
- diagnostics/troubleshooting completeness across all public docs;
- final confirmation that no current docs describe planned behavior as already functional;
- final documentation audit at WWR-950/960 closeout.
