# Wake Word V1 sherpa KWS model selection

**Recorded:** 2026-09-16
**TODO:** `WW-100` in `docs/WAKE_WORD_V1_TODO_2026-09-14.md`
**Status:** Model family selected; immutable artifact/runtime pinning remains open

## Selected model

Wake Word V1 selects the sherpa-onnx English keyword-spotting model family:

`sherpa-onnx-kws-zipformer-gigaspeech-3.3M-2024-01-01`

This is the English-only Zipformer transducer KWS model documented by the sherpa-onnx project and trained from the GigaSpeech XL subset. It is preferable to the Chinese-only WenetSpeech KWS model for the fixed English wake phrase `Hey, Moose`. The newer bilingual model is not required for the frozen V1 English-only product scope.

Upstream model documentation identifies the production inputs as the encoder, decoder, joiner, `tokens.txt`, and `bpe.model`. Both fp32 and int8 ONNX variants are published. V1 starts from the fp32 model until real acceptance establishes whether int8 is desirable.

## Frozen audio and inference configuration

The upstream KWS configuration reports:

- feature sample rate: **16,000 Hz**;
- feature dimension: **80**;
- CPU provider;
- inference threads: **1**;
- keyword score/boost: **1.0**;
- keyword threshold: **0.25**.

These values match the existing Wake Word V1 constants and the 16 kHz mono PCM pre-roll contract. Sensitivity is therefore fixed for V1 at score `1.0` and threshold `0.25`; no user-facing sensitivity setting is added until corpus acceptance demonstrates a stable mapping.

The upstream command-line examples accept single-channel 16-bit WAV input and document that input WAV sample rate need not already be 16 kHz; sherpa's feature configuration itself is 16 kHz. The application should continue to normalize the authoritative microphone stream to canonical 16 kHz mono PCM before KWS/ring-buffer routing rather than depending on file-oriented resampling behavior.

## Keyword preparation

This GigaSpeech model uses BPE keyword preparation. Upstream documents generation of the keyword token sequence with `sherpa-onnx-cli text2token`, supplying this model's `tokens.txt`, `--tokens-type bpe`, and `bpe.model`. The raw V1 keyword is the frozen canonical English phrase `HEY MOOSE`; the exact generated token sequence must be committed or deterministically generated and verified before WW-100 closes.

## Native API boundary

Wake Word V1 continues to require sherpa-onnx native integration from Rust/Tauri with no Python sidecar. Model selection does not by itself choose or qualify a particular native sherpa release. The native runtime version and per-platform libraries remain an explicit WW-100/WW-110 deliverable and must be pinned independently for Linux x86_64 and macOS arm64 before production inference is enabled.

## Provenance references

Authoritative upstream documentation:

- `https://k2-fsa.github.io/sherpa/onnx/kws/index.html`
- `https://k2-fsa.github.io/sherpa/onnx/kws/pretrained_models/index.html`

Published model release asset identity (URL is stable by model release tag/name, but the bytes are not yet accepted by this repository until independently hashed):

- `https://github.com/k2-fsa/sherpa-onnx/releases/download/kws-models/sherpa-onnx-kws-zipformer-gigaspeech-3.3M-2024-01-01.tar.bz2`

The sherpa-onnx codebase is Apache-2.0. The selected model's own license/provenance and all required notices still require repository-local verification before WW-100 acceptance; do not infer model licensing solely from the runtime license.

## Remaining WW-100 closure requirements

This selection is intentionally not presented as full WW-100 closure. Before the model/runtime inputs are production-eligible, a follow-up must:

1. download the selected model through the deterministic artifact-preparation path;
2. record exact archive byte size and SHA-256;
3. record exact byte sizes and SHA-256 values for every consumed model/tokenizer file;
4. generate and freeze the exact `HEY MOOSE` BPE keyword representation;
5. select an immutable sherpa-onnx native runtime version;
6. record exact Linux x86_64 and macOS arm64 native artifact identities, sizes, hashes, and architectures;
7. verify model/runtime licenses and add required notices;
8. implement fail-closed preparation/verification so a hash or architecture mismatch cannot reach inference.

Until those items are complete, this document freezes only the model family, canonical audio/KWS defaults, and keyword-preparation mechanism. It does not authorize unverified runtime downloads or production KWS execution.
