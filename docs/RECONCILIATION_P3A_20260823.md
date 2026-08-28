# P3A Native Moonshine ASR Reconciliation — 2026-08-23

## Status

**P3A native-model acceptance gate: CLOSED.**

This reconciliation closes the remaining evidence gap for real pinned Moonshine Tiny Streaming and Small Streaming native model load/stream/transcription plus the ASR-015 measured CPU benchmark. It does not close unrelated physical microphone/output-device acceptance in P2/P3 or release signing/notarization. V1R-ASR-004 Miri-compatible hardening is separately recorded below.

## Evidence source

The authoritative acceptance execution is GitHub Actions run `32665369708`, workflow `ASR-015 Native Acceptance`, run attempt 1, completed successfully on 2026-08-23 against exact commit:

`f181a1d18ab65eecfddf875f6706d9bce5f136fb`

The single job `Tiny + Small native acceptance and CPU benchmark` completed successfully, including:

- exact source-revision verification;
- pinned Moonshine/ONNX native runtime preparation;
- pinned model-manifest Rust contract validation;
- Tiny and Small release-mode native acceptance benchmark;
- durable evidence upload.

The preserved artifact is:

- name: `asr015-supported-mac-f181a1d18ab65eecfddf875f6706d9bce5f136fb`;
- artifact ID: `9500044584`;
- recorded digest: `sha256:b8964a048667fb638fd9e4fc39dc3825a5fac68afd7458e1e5b6ca68c2c9764d`.

The artifact contains the rendered acceptance report, raw `asr015-records.jsonl`, hardware/corpus metadata, and all 12 per-run logs.

## Reference environment

- Hardware model: `VirtualMac2,1`
- CPU/chip: `Apple M1 (Virtual)`
- Physical/logical CPU count: 3 / 3
- RAM: 7168 MiB
- macOS: 15.7.7 (`24G720`)
- Architecture: arm64
- Low-power mode: unknown

This is the minimum **measured acceptance reference** established by ASR-015. No slower Mac CPU is claimed supported by this evidence. The virtual 3-vCPU reference must not be presented as a blanket consumer-hardware equivalence claim for every physical M1 configuration.

## Corpus identity

Both models used the same immutable representative corpus:

- source repository/path: `ggml-org/whisper.cpp/samples/jfk.wav`;
- source commit: `45f1593fd326b3435c04392e3151dff65967e523`;
- source Git blob: `3184d372cd2f8b804d3a540c70ec50d927b335d2`;
- source SHA-256: `59dfb9a4acb36fe2a2affc14bacbee2920ff435cb13cc314a08c13f66ba7860e`;
- derived PCM SHA-256: `5d5024881abcb527a43c9b643abed1545627960ac894584167ea510c8a442061`;
- format: 16,000 Hz mono signed 16-bit little-endian PCM;
- duration: 13.0 s total (11.0 s speech plus 2.0 s trailing silence).

Each run accepted all 130 100-ms chunks.

## Tiny Streaming measured results

Tiny completed one warm-up plus five measured production-pipeline native runs with the pinned model `moonshine-tiny-streaming-en`, revision `quantized_26_07_30`, runtime release `v0.1.3`.

Across measured runs 1–5:

- median RTF: **0.099**;
- worst RTF: **0.106**;
- median first-final latency: **4747 ms**;
- worst first-final latency: **5356 ms**;
- measured first-partial latency range: **1339–1793 ms**;
- highest sampled process RSS: **306.5 MiB**;
- average process CPU utilization range: **4.9%–6.0%**;
- bounded-ingress drops: **0**;
- typed ASR errors: **0**;
- useful final transcript emitted on every run: **yes**.

The final transcript was stable across all six Tiny runs and materially matched the JFK reference speech.

## Small Streaming measured results

Small completed one warm-up plus five measured production-pipeline native runs with the pinned model `moonshine-small-streaming-en`, revision `quantized_26_07_30`, runtime release `v0.1.3`.

Across measured runs 1–5:

- median RTF: **0.195**;
- worst RTF: **0.200**;
- median first-final latency: **5081 ms**;
- worst first-final latency: **5434 ms**;
- measured first-partial latency range: **1602–1923 ms**;
- highest sampled process RSS: **758.6 MiB**;
- average process CPU utilization range: **10.2%–11.7%**;
- bounded-ingress drops: **0**;
- typed ASR errors: **0**;
- useful final transcript emitted on every run: **yes**.

The final transcript was stable across all six Small runs and materially matched the JFK reference speech.

## Raw-record cross-check

The raw JSONL contains exactly 12 records:

- Tiny: 1 warm-up + 5 measured runs;
- Small: 1 warm-up + 5 measured runs.

For both models the measured run numbers are exactly `1,2,3,4,5`; every record reports `processed_audio_ms = 13000`, `accepted_chunks = 130`, `dropped_chunks = 0`, and `last_error = null`. Independently recomputed medians/worst values match the rendered report.

## Requirement disposition

### V1R-ASR-004 — Safe Rust Moonshine wrapper

- RAII/FFI/fake-seam implementation remains complete.
- **Miri-compatible Rust safety coverage: ACCEPTED.** The dedicated `ASR Rust Safety` workflow run `32544957896` passed on commit `83a3b0fa42ca5313a289b3334a7632aebeb22c19`, executing 15 production-module tests under Miri, including the real RAII transcriber/stream ownership and drop-ordering code through the fake ABI seam. This evidence predates ASR-015 and was missed by the original reconciliation text; the checked row in `TODO(20260818-163801).md` was correct.
- **2026-08-28 hardening:** the Miri harness is strengthened so the production `ffi.rs` C-layout transcript structs, ABI layout assertions, and `NativeMoonshineApi::copy_transcript` helper also compile under unit-test/Miri configuration without linking Moonshine. Synthetic C-layout fixtures exercise null transcript rejection, null line-array rejection, and the unsafe transcript-pointer/slice/C-string copy boundary while proving returned text/metadata are Rust-owned after backing native-style storage is released. The safety workflow pins the previously successful `nightly-2026-08-22` toolchain instead of floating on nightly.
- Miri still does **not** execute Moonshine or ONNX Runtime dylib calls. That boundary is intentionally covered by the supported-macOS native acceptance below rather than overstated as Miri coverage.
- **Real native stream lifecycle with an installed verified model: ACCEPTED.** Both Tiny and Small loaded the real pinned native runtime/model, accepted sustained streaming PCM, emitted partial/final transcription, and shut down through the benchmark lifecycle.

### V1R-ASR-015 — Diagnostics and CPU benchmark

- **Representative Tiny/Small CPU-only benchmark: ACCEPTED.**
- **Minimum measured CPU acceptance reference: DEFINED** as the GitHub macOS-15 arm64 environment presenting `Apple M1 (Virtual)` / `VirtualMac2,1`, 3 vCPUs, 7 GiB RAM.
- This is intentionally a measured-reference statement, not a claim that slower hardware or every physical M1 Mac has been independently benchmarked.

### Gate P3A

- **Tiny Streaming real-model native load/stream/transcription: ACCEPTED.**
- **Small Streaming real-model native load/stream/transcription: ACCEPTED.**
- Cloud/local mode switch remains explicit and lifecycle-safe from existing deterministic coverage.
- No local failure can silently enable cloud audio from existing privacy/failure coverage.

Therefore **Gate P3A is closed**.

## Scope boundaries

This P3A closure does not replace:

- P2 physical built-in/USB microphone, negotiated device-format, permission, and output-device acceptance;
- P3 physical interruption-to-audible-stop latency acceptance;
- P1 real macOS Keychain restart acceptance;
- P6 intentional human voice audition;
- P13 signed/notarized physical release acceptance.

The benchmark uses fixed representative PCM through the production local-ASR pipeline. Physical microphone/device behavior remains deliberately owned by P2/P3 rather than being inferred from this benchmark.

## Next acceptance work

With P3A closed, the next independent physical acceptance target is the remaining real-macOS security/privacy work, starting with P1 Keychain persistence/restart acceptance. Subsequent remaining human/device gates include the P6 voice audition, P2/P3 physical audio acceptance, and P13 release acceptance.
