# Wake Word V1 artifact preparation policy

Wake Word V1 uses `wake-word-artifacts.json` as the sole production identity boundary. The preparation path is deliberately split from identity selection so no network response, HTTP cache, or extraction result can become trusted merely because a download succeeded.

## Preparation sequence

`scripts/prepare-wake-word-artifacts.py` accepts only archive URLs and identities already frozen in the committed manifest. For the requested platform it:

1. selects only `all` and exact-platform archive records;
2. requires a frozen HTTPS URL, filename, positive byte size, and SHA-256;
3. downloads only when the archive is not already present in the supplied download directory;
4. verifies exact archive byte size and SHA-256 before extraction;
5. rejects archive members with absolute paths or `..` traversal;
6. extracts only the declared `tar.bz2` or `zip` format;
7. invokes `scripts/verify-wake-word-artifacts.py` after extraction so every selected production file is independently size/SHA verified.

A cached archive therefore receives exactly the same cryptographic verification as a fresh download. A corrupt or replaced cache cannot substitute for identity evidence.

## Platform layout

The manifest will freeze archives for `linux-x86_64` and `macos-arm64` plus the platform-independent English GigaSpeech KWS model archive. Extracted model/runtime paths remain deterministic and are themselves represented in the manifest's `artifacts` list. Additional release architectures are intentionally not accepted until real runtime/package qualification exists for them.

## Remaining identity work

The preparation mechanism is complete, but production preparation remains intentionally impossible while the manifest has no frozen archive records. WW-100/WW-110 must not be marked complete until exact upstream archive filenames, byte sizes, SHA-256 values, extracted-file identities, architecture evidence, and licensing/provenance are committed. This fail-closed state is intentional.

The selected model remains `sherpa-onnx-kws-zipformer-gigaspeech-3.3M-2024-01-01`; upstream documents it as English-only and 16-kHz KWS. The selected initial runtime remains sherpa-onnx `v1.13.8`. No Python sidecar is introduced.
