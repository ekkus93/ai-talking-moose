# Moonshine CPU Benchmark and ASR Diagnostics

Status: **ASR-015 instrumentation complete; supported-Mac acceptance measurements pending**
Recorded: 2026-08-21

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

cargo test --release --manifest-path src-tauri/Cargo.toml \
  asr015_cpu_benchmark_tiny_on_supported_mac \
  -- --ignored --nocapture
```

Run the corresponding `asr015_cpu_benchmark_small_on_supported_mac` test for Small. The benchmark feeds one 100 ms chunk every 100 ms using the same non-blocking bounded ingress behavior as production and fails if the queue drops a chunk. Set `TALKING_MOOSE_ASR_BENCHMARK_INSTALL=1` when the benchmark should install or re-verify the pinned model through the production model installer before native startup.

For reproducible project acceptance, use the opt-in `ASR-015 Native Acceptance` GitHub Actions workflow or run `scripts/run_asr015_macos_acceptance.sh "$(uname -m)"` on a supported Mac. The local runner requires a clean tracked checkout, validates and prepares the pinned native runtime itself, generates deterministic Tauri icon inputs, installs/re-verifies the pinned models through the production installer, and rejects a runtime whose Mach-O architecture does not match the host. The GitHub workflow prepares the same pinned runtime in its preceding provenance step and explicitly reuses that fresh staging directory. The automation performs one warm-up plus five measured **release-mode** runs for Tiny, then the same for Small, using one verified model-install root and the fixed corpus above. It emits machine-readable `ASR015_BENCHMARK_JSON` records and renders a complete Markdown report with every run, medians, worst cases, hardware identity, corpus identity, CPU utilization, RTF, latency, RSS, and transcript evidence. Ordinary CI does not invoke this workflow and still downloads no model weights.

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

No CPU model is declared supported by ASR-015 until representative measurements exist. The minimum supported reference CPU will be chosen from measured Macs, not from model size, marketing generation, core count, or an assumed performance ratio.

For the chosen minimum supported reference CPU, both selectable local models must complete the representative sustained feed with:

1. zero bounded-ingress drops;
2. no typed ASR error;
3. a useful partial and final transcript;
4. sustained RTF strictly below `1.0` on every release-acceptance run;
5. recorded CPU and RSS values that fit the product's documented system requirements.

A slower machine may only be added to the supported set after it independently passes the same measured gate. If Small cannot satisfy the gate on the minimum Tiny-capable CPU, either Small's minimum requirement must be documented separately or Small cannot be advertised as supported on that CPU.

## Current acceptance record

The Linux sandbox used for implementation cannot supply supported-macOS CPU evidence and does not have the pinned native Moonshine runtime linked. Therefore ASR-015's benchmark-data and minimum-supported-CPU checklist items remain open until the opt-in Tiny and Small runs are executed on representative supported Mac hardware and their measurements are committed here (or in a timestamped successor benchmark report).
