# WSP-200 macOS real-model acceptance — Dispatch limitation

**Observed:** 2026-10-15
**Repository:** `ekkus93/ai-talking-moose`
**Current head at record:** `6eefba7658888100e0587aa10ef332839247c944`
**Ralph Bridge:** `0.6.2`, build `0acea4b23362e2ccecab8f6c9db4418b162426cf`, authentication mode `app`.

## Qualified prerequisite, not real-model proof

- GitHub Actions `Whisper macOS Native Compile` run `38821907488` on exact head `6cc3fe302baad4d8d5a579c7bffd65770542131c` **passed** on both `macos-15` arm64 and `macos-15-intel` x86_64.
- Both native acceptance CLI build jobs compiled the pinned Whisper CMake static archives, verified the final architecture and invoked the real FFI `whisper_version` probe. This confirms native linkage/probe only; **no 487,601,967-byte model inference or packaged application smoke is claimed**.
- The latest ordinary repository `CI` run `38823450607` on `6eefba7658888100e0587aa10ef332839247c944` passed its selected gates. This is not WSP-200 real-weight evidence.

## Concrete execution boundary

The existing workflow `.github/workflows/whisper-macos-real-cpu-acceptance.yml` is intentionally `workflow_dispatch` only, to keep heavy model downloads out of ordinary CI. It tests both Mac architectures, explicitly installs the pinned model through the production installer, invokes native transcription of the real WAV corpus, then deletes the model and uploads JSON evidence without weights. It does **not** claim enforced macOS egress denial or a physical microphone audition.

Through Ralph Bridge `dispatch_validation`, calls with profile `whisper-macos-real-cpu-acceptance` and then `whisper-macos-real-cpu` were both rejected with `INVALID_ARGUMENT` / `Error executing tool dispatch_validation`. A separate `run_validation` with profile `rust` was also rejected. Ralph Bridge still successfully handled repository reads and exact-head commits, so it is not a general connectivity outage. The API provides no detailed reason for these profile errors; do **not** assert which operator setting or GitHub App permission is missing.

**Result:** No successfully dispatched real-model Mac workflow, no SHA-bound transcript/metrics and no WSP-200 qualification. This requires either:
- Operator configuration of an approved Ralph Bridge `dispatch_validation` profile bound to `whisper-macos-real-cpu-acceptance.yml` with sufficient GitHub Actions dispatch permissions; or
- An authorized human to launch the workflow once in GitHub Actions on current `master` via **Actions → Whisper macOS Real CPU Acceptance → Run workflow → master** and provide its run ID for Ralph Bridge verification.

Do not create a push-triggered workaround that downloads 488 MB on every ordinary commit or weakens the integrity/network policy. When a run is available, inspect **both** arm64 and Intel jobs, exact source SHA, model revision/size/hash, nonempty genuine transcript, runtime metrics, and uploaded evidence; address failures normally. The macOS *network-denied* and *physical microphone* gates remain separate and must not be inferred from a successful fixed-corpus CI run.

**Safety gate:** Keep Moonshine installed and selected by default on macOS until WSP-200 has genuine supported-architecture transcription evidence. Do not remove the independent KittenTTS ONNX Runtime or the separate wake-word sherpa-onnx runtime.
