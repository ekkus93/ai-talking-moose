# Wake Word V1 — sherpa-onnx artifact selection

**Date:** 2026-09-15
**Status:** WW-100 selection decision; immutable byte-level manifest still required before WW-100 acceptance

## Selected KWS model family

Use `sherpa-onnx-kws-zipformer-gigaspeech-3.3M-2024-01-01` for Wake Word V1.

Rationale:

- It is the sherpa-onnx KWS model documented specifically for English.
- It is an open-vocabulary/custom-keyword Zipformer KWS model, so `Hey, Moose` can be configured without training a Moose-specific neural model.
- Upstream documents both fp32 and int8 encoder/decoder/joiner variants. V1 should begin with the int8 model set for continuous CPU use, subject to real-corpus acceptance proving adequate recall/false-trigger behavior.
- The model is trained on the GigaSpeech XL subset (10,000 hours) according to upstream sherpa-onnx documentation.

Selected production model files:

- `encoder-epoch-12-avg-2-chunk-16-left-64.int8.onnx`
- `decoder-epoch-12-avg-2-chunk-16-left-64.int8.onnx`
- `joiner-epoch-12-avg-2-chunk-16-left-64.int8.onnx`
- `tokens.txt`
- `bpe.model`

The production artifact-preparation work must pin and verify the containing upstream `kws-models` release asset by exact byte size and SHA-256 before this selection is accepted for release.

## Audio contract

Wake Word V1 retains the project canonical 16 kHz mono signed-16-bit PCM contract. Upstream's file decoder requires mono 16-bit samples and can handle input sampling rates other than 16 kHz; keeping the project's wake path at 16 kHz avoids needless resampling and matches the already-merged 32,000-sample/two-second pre-roll definition.

## Keyword preparation

The GigaSpeech KWS model uses BPE keyword tokens. Upstream's documented preparation flow is equivalent to:

```text
sherpa-onnx-cli text2token \
  --tokens <model>/tokens.txt \
  --tokens-type bpe \
  --bpe-model <model>/bpe.model \
  keywords_raw.txt keywords.txt
```

For V1 the source phrase is fixed to `HEY MOOSE`. The generated token sequence must be committed or deterministically generated from the pinned `tokens.txt` and `bpe.model`; runtime startup must not depend on Python, pip, or a network request.

Sherpa KWS supports per-keyword boosting score and trigger threshold. Those values remain explicit V1 configuration and must be frozen only after corpus qualification; lower threshold / higher boost makes triggering easier and therefore changes the false-accept tradeoff.

## Native runtime selection

Pin sherpa-onnx runtime version `v1.13.8` for the first Wake Word V1 implementation candidate. Upstream released it on 2026-09-10 and publishes prebuilt native artifacts for the required desktop architectures, including Linux x64 and macOS arm64. sherpa-onnx supports a Rust API, so no Python sidecar is required for production inference.

The repository's deterministic runtime manifest must record the exact chosen native archive filenames, byte sizes, SHA-256 values, architecture, extraction layout, and license before WW-100/WW-110 can be marked complete. CI caches are never identity evidence: every prepared artifact must be verified against the committed manifest.

## Licensing/provenance boundary

sherpa-onnx code is Apache-2.0. Model licensing must be reviewed independently and recorded with the exact selected model artifact; upstream explicitly warns that supported models can have licenses different from sherpa-onnx itself. No production packaging should proceed until that model-license evidence is committed.

## Fail-closed requirements

Artifact preparation/runtime loading must reject:

- a SHA-256 mismatch;
- a byte-size mismatch where size is frozen;
- an unsupported OS/architecture;
- a missing required model/tokenizer file;
- a native library whose inspected architecture does not match the target.

None of those failures may switch ASR providers, enable cloud processing, or fall back to full-time ASR.

## Remaining WW-100 evidence

This decision intentionally does **not** mark WW-100 complete. Before acceptance, the implementation still needs exact byte identities for the model archive and selected Linux x86_64/macOS arm64 native artifacts, model-license provenance, deterministic preparation/verification scripts, and CI evidence proving mismatch rejection.
