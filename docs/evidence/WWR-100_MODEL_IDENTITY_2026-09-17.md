# WWR-100 — selected KWS model identity evidence

The selected V1 model is `sherpa-onnx-kws-zipformer-gigaspeech-3.3M-2024-01-01`. Upstream sherpa documentation identifies this exact release asset and the fp32 encoder/decoder/joiner plus `tokens.txt` and `bpe.model` consumed by the KWS command. The same documentation identifies the model as trained on the GigaSpeech XL subset and links the icefall training work.

Deterministic freezer run `35288533232` on PR-head `af3532ac0584b2af6ae0d7719598bf543adab81c` independently downloaded the selected release archive, rejected unsafe extraction forms, hashed the archive and exact five consumed files, and generated the `HEY MOOSE` BPE representation using pinned SentencePiece 0.2.1.

Frozen identities:

- archive: 17,626,723 bytes, SHA-256 `f170013b4716e41b62b9bfd809687c207cef798ef9bc6534d524e17af9b6561a`
- encoder: 12,174,219 bytes, SHA-256 `063fbc1aeae8a9b574607a331a00e60371846ef9eaa3c1d9ea48176665dfc693`
- decoder: 1,063,189 bytes, SHA-256 `f61ebd3eed3773a44d088d53dfae92dbb6aec4839f4dcaee2d402414741663a3`
- joiner: 642,462 bytes, SHA-256 `0d7a37e749d8055223029318d6ffae82db1dae2d315d0892a68ba5dad17c1d2d`
- `tokens.txt`: 5,006 bytes, SHA-256 `fd2ded4050a55d2b1578870ba8697d02371980217806b7558bd0a5cc60f3ba53`
- `bpe.model`: 244,837 bytes, SHA-256 `c8a2a0129c4ab8e463164c142f82d25649661b122c8cd0b7aab5c9e80b90ad24`
- generated `HEY MOOSE` representation: `▁HE Y ▁MO O SE`; canonical newline-terminated artifact is 19 bytes, SHA-256 `3b1ad407b63b5e89edd8e253b9c104ac85d74a2a862b0d07ac4a0d4ee27770a6`.

## Provenance and license

The sherpa-onnx documentation identifies the GigaSpeech KWS model and also names pkufool's ModelScope repository as an alternate distribution. That ModelScope model card identifies the model license as Apache License 2.0 and describes the same GigaSpeech XL training provenance. This model-license evidence is recorded separately from the sherpa-onnx runtime's Apache-2.0 license so runtime licensing is not used as a proxy for model licensing.

The authoritative machine-readable identities are `wake-word-artifacts.json`; Rust mirrors them in `src-tauri/src/asr/wake_word_sherpa_manifest.rs` and has a regression test requiring all production identities and the keyword representation to agree with the JSON manifest.
