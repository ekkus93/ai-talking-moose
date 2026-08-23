# Moonshine CPU Benchmark and ASR Diagnostics

Status: **ASR-015 supported-Mac acceptance passed**
Recorded: 2026-08-23

## Purpose

ASR-015 must measure local Moonshine behavior instead of inventing a minimum CPU requirement. Talking Moose therefore exposes the same bounded-pipeline measurements used by the application and provides an opt-in, hardware-dependent benchmark path for Moonshine Tiny Streaming and Small Streaming.

The pinned Moonshine runtime intentionally ships ONNX Runtime's CPU execution provider only on every platform; its upstream execution-provider design explicitly states that shipped builds do not contain CoreML or NNAPI providers. The macOS acceptance measurements are therefore CPU-only rather than hidden Apple Neural Engine/CoreML results.

Ordinary tests do **not** download a model, load the native Moonshine runtime, use a microphone, or contact Google.

## User-facing diagnostics

The ASR settings diagnostics surface reports:

- selected ASR engine, model ID/revision, and install state;
- local streaming state;
- input sample rate and available logical CPU threads;
- inference queue depth/capacity and authoritative microphone dropped-chunk count;
- last typed local ASR error;
- time from the first processed audio chunk to the first useful partial and first final transcript;
- latest native Moonshine transcription latency;
- cumulative processed-audio duration, inference wall time, and real-time factor (RTF);
- process CPU time and average process CPU utilization from native-engine readiness through the local streaming session (model-load CPU is excluded);
- process RSS immediately before native model load, current RSS, and the highest RSS sample observed during the local-ASR session.

The most recent completed local-ASR session snapshot is retained after teardown for the selected local mode, so timing/RTF/memory data and the last typed ASR error do not disappear as soon as the conversation stops. Retained data is explicitly marked as a snapshot; its resident-memory value is the session-final RSS sampled immediately before model teardown, not a claim about live RSS after the model has been unloaded.

Memory is deliberately labeled **process RSS**. Moonshine/ONNX Runtime does not expose a trustworthy model-exclusive resident-memory counter, so Talking Moose reports the pre-model baseline and lets the UI show current/peak RSS increases without misrepresenting them as allocations owned only by the ASR model.

RTF is `cumulative inference wall time / cumulative processed audio duration`. RTF below `1.0` means inference itself is faster than real time. Queue drops remain a separate acceptance signal because sustained recognition can still fail if the bounded eight-chunk ingress queue overloads.

## Representative corpus

The supported-Mac acceptance automation uses `ggml-org/whisper.cpp` `samples/jfk.wav` pinned at commit `45f1593fd326b3435c04392e3151dff65967e523` and Git blob `3184d372cd2f8b804d3a540c70ec50d927b335d2`. The source is the 11-second English JFK excerpt already used by whisper.cpp for transcription/benchmark examples. `scripts/prepare_asr015_corpus.py` downloads that immutable source at acceptance time, verifies the Git blob identity, validates 16 kHz mono signed-16-bit PCM, strips the WAV container, appends exactly 2 seconds of silence for streaming finalization, and records both source and derived-corpus SHA-256 values. The audio is not stored in this repository.

The supported-Mac acceptance run must use the same fixed speech corpus for Tiny and Small. The corpus must:

- be English speech representative of conversational microphone input;
- be mono, signed 16-bit little-endian PCM at exactly 16,000 Hz;
- contain whole 100 ms chunks (3,200 bytes per chunk);
- contain enough speech to exercise sustained streaming rather than only startup;
- contain sufficient trailing silence for Moonshine to emit a final transcript;
- be redistributable or kept outside the repository with its provenance recorded in the benchmark report.

Do not use silence, synthetic random data, or a different corpus per model as release evidence.

## Opt-in benchmark invocation

The hardware benchmark lives as ignored macOS tests in `src-tauri/src/asr/pipeline_benchmarks.rs`. It requires a build explicitly linked to the pinned Moonshine native runtime and pre-installed, verified model payloads.

For the low-level ignored Cargo tests, prepare the pinned runtime first on the Mac that will run the benchmark:

```bash
bash scripts/prepare_moonshine_macos.sh "$(uname -m)"
```

The ordinary packaged-runtime path is then used automatically by `build.rs`; no Homebrew path or developer checkout is required. `TALKING_MOOSE_MOONSHINE_LIB_DIR` remains an explicit development override only. Example environment and invocation for Tiny:

```bash
export TALKING_MOOSE_ASR_BENCHMARK=1
export TALKING_MOOSE_ASR_BENCHMARK_MODEL_ROOT="$HOME/Library/Application Support/Talking Moose/models"
export TALKING_MOOSE_ASR_BENCHMARK_PCM=/absolute/path/to/asr015-corpus.pcm

cargo test --release --locked --manifest-path src-tauri/Cargo.toml \
  asr015_cpu_benchmark_tiny_on_supported_mac \
  -- --ignored --nocapture
```

Run the corresponding `asr015_cpu_benchmark_small_on_supported_mac` test for Small. The benchmark feeds one 100 ms chunk every 100 ms using the same non-blocking bounded ingress behavior as production and fails if the queue drops a chunk. Set `TALKING_MOOSE_ASR_BENCHMARK_INSTALL=1` when the benchmark should install or re-verify the pinned model through the production model installer before native startup.

For reproducible project acceptance, use the opt-in `ASR-015 Native Acceptance` GitHub Actions workflow or run `scripts/run_asr015_macos_acceptance.sh "$(uname -m)"` on a supported Mac. The local runner requires a clean tracked checkout, validates and prepares the pinned native runtime itself, generates deterministic Tauri icon inputs, installs/re-verifies the pinned models through the production installer, and rejects a runtime whose Mach-O architecture does not match the host. The GitHub workflow prepares the same pinned runtime in its preceding provenance step and explicitly reuses that fresh staging directory. The automation performs one warm-up plus five measured **release-mode** runs for Tiny, then the same for Small, using one verified model-install root and the fixed corpus above. It emits machine-readable `ASR015_BENCHMARK_JSON` records and renders a complete Markdown report with every run, medians, worst cases, hardware identity, corpus identity, CPU utilization, RTF, latency, RSS, and transcript evidence. The workflow also preserves the report, raw JSONL records, hardware/corpus metadata, and per-run logs as a workflow artifact. Ordinary CI does not invoke this workflow and still downloads no model weights.

## Required report fields

Each release-evidence run must record:

- Mac model identifier and Apple/Intel CPU model;
- physical/logical core count;
- RAM;
- macOS version and power mode;
- Talking Moose commit SHA;
- Moonshine runtime release/commit and native library architecture;
- model ID/revision and verified payload size;
- corpus identity, provenance, SHA-256, and audio duration;
- first useful partial latency;
- first final latency;
- latest/native decode latency as contextual evidence;
- processed-audio duration;
- cumulative inference wall time;
- RTF;
- process CPU time and average process CPU utilization;
- pre-model, steady/current, and highest-sampled process RSS plus their increases from baseline;
- bounded-queue drops (must be zero for acceptance).

Run each model at least five times after one warm-up run. Report all runs plus median and worst observed RTF/latency/RSS; do not publish only the best run.

## Minimum supported CPU acceptance

No CPU model is declared supported by ASR-015 until representative measurements exist. The minimum supported reference CPU is chosen from measured Macs, not from model size, marketing generation, core count, or an assumed performance ratio.

For the chosen minimum supported reference CPU, both selectable local models must complete the representative sustained feed with:

1. zero bounded-ingress drops;
2. no typed ASR error;
3. a useful partial and final transcript;
4. sustained RTF strictly below `1.0` on every release-acceptance run;
5. recorded CPU and RSS values that fit the product's documented system requirements.

A slower machine may only be added to the supported set after it independently passes the same measured gate. If Small cannot satisfy the gate on the minimum Tiny-capable CPU, either Small's minimum requirement must be documented separately or Small cannot be advertised as supported on that CPU.

## Current acceptance record

GitHub Actions run `32665369708` passed the `ASR-015 Native Acceptance` workflow on 2026-08-23 at commit `f181a1d18ab65eecfddf875f6706d9bce5f136fb`. The durable artifact is `asr015-supported-mac-f181a1d18ab65eecfddf875f6706d9bce5f136fb` (artifact ID `9500044584`, recorded digest `sha256:b8964a048667fb638fd9e4fc39dc3825a5fac68afd7458e1e5b6ca68c2c9764d`).

Reference environment:

- hardware model: `VirtualMac2,1`;
- CPU/chip: `Apple M1 (Virtual)`;
- physical/logical CPU count: 3 / 3;
- RAM: 7168 MiB;
- macOS: 15.7.7 (`24G720`);
- architecture: arm64;
- corpus: 13.0 s, 16 kHz mono s16le, derived SHA-256 `5d5024881abcb527a43c9b643abed1545627960ac894584167ea510c8a442061`.

Tiny Streaming completed one warm-up and five measured native runs. Across the five measured runs, median RTF was **0.099**, worst RTF was **0.106**, median first-final latency was **4747 ms**, worst first-final latency was **5356 ms**, highest sampled process RSS was **306.5 MiB**, average process CPU utilization ranged from about **4.9% to 6.0%**, every run accepted all 130 chunks, and total drops/errors were zero.

Small Streaming completed the same one-warm-up/five-measured protocol. Median RTF was **0.195**, worst RTF was **0.200**, median first-final latency was **5081 ms**, worst first-final latency was **5434 ms**, highest sampled process RSS was **758.6 MiB**, average process CPU utilization ranged from about **10.2% to 11.7%**, every run accepted all 130 chunks, and total drops/errors were zero.

Every Tiny and Small run emitted a useful partial and final transcript from the real pinned native model/runtime. The minimum **measured acceptance reference** is therefore the GitHub macOS-15 arm64 environment presenting `Apple M1 (Virtual)` / `VirtualMac2,1` with 3 vCPUs. This evidence does **not** claim support for a slower CPU, does not establish a consumer-facing equivalence between a virtual 3-vCPU runner and every physical M1 Mac, and does not replace the separate P2/P3 physical microphone/device acceptance gates.

Detailed reconciliation and gate disposition are recorded in `docs/RECONCILIATION_P3A_20260823.md`.
