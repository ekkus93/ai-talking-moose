# Local TTS Runtime and Downloaded-Asset License Evidence

Status: **KCR release-policy evidence**
Recorded: 2026-09-12

Talking Moose does **not** bundle KittenTTS model weights, voice embeddings, CMUdict data, or ONNX Runtime archives in the ordinary application bundle. These artifacts are downloaded from their pinned upstream locations by the Local TTS installer after explicit user action. This document records the license evidence for those externally acquired artifacts separately from the generated license inventory for dependencies that are actually linked or bundled into the application.

The production source of truth for filenames, immutable revisions, exact byte counts, SHA-256 values, and per-platform selection is `src-tauri/src/ai/local_tts/manifest/catalog.rs`. CI must fail if this document and that catalog drift apart.

## Installer-downloaded artifacts

| Artifact | Immutable source identity | License evidence | Distribution boundary |
| --- | --- | --- | --- |
| `kitten_tts_mini_v0_8.onnx` | `KittenML/kitten-tts-mini-0.8` revision `c02725660cea441db4c383af69f1f26f5cd00947` | Apache-2.0 repository/model-card metadata at the pinned revision. | Downloaded from Hugging Face after explicit install; not committed or bundled. |
| `voices.npz` | `KittenML/kitten-tts-mini-0.8` revision `c02725660cea441db4c383af69f1f26f5cd00947` | Apache-2.0 repository/model-card metadata at the same pinned revision containing the voice artifact. No separate file-level license is asserted beyond that repository-scoped evidence. | Downloaded from Hugging Face after explicit install; not committed or bundled. |
| `cmudict_data.json` | `ayutaz/piper-plus` revision `244ffeb44108347a514ebfc0c2f773d938c9613b` | `src/rust/piper-plus-g2p/THIRD_PARTY_LICENSES.md` identifies this exact asset as **BSD-style (CMU)** and reproduces the CMU Pronouncing Dictionary redistribution conditions/acknowledgment. | Downloaded after explicit install; not compiled into the app because `bundled-dicts` is disabled. |
| `onnxruntime-linux-x64-1.23.2.tgz` | Microsoft ONNX Runtime release `v1.23.2` | MIT license from upstream `LICENSE` at tag `v1.23.2`. | Downloaded for Linux x86_64 after explicit install; verified before runtime use; not bundled. |
| `onnxruntime-osx-arm64-1.23.2.tgz` | Microsoft ONNX Runtime release `v1.23.2` | MIT license from upstream `LICENSE` at tag `v1.23.2`. | Downloaded for macOS arm64 after explicit install; verified before runtime use; not bundled. |
| `onnxruntime-osx-x86_64-1.23.2.tgz` | Microsoft ONNX Runtime release `v1.23.2` | MIT license from upstream `LICENSE` at tag `v1.23.2`. | Downloaded for macOS x86_64 after explicit install; verified before runtime use; not bundled. |

### Upstream evidence locations

- KittenTTS model/voice repository metadata at the pinned revision: `https://huggingface.co/KittenML/kitten-tts-mini-0.8/blob/c02725660cea441db4c383af69f1f26f5cd00947/README.md`
- piper-plus G2P third-party evidence at the pinned revision: `https://github.com/ayutaz/piper-plus/blob/244ffeb44108347a514ebfc0c2f773d938c9613b/src/rust/piper-plus-g2p/THIRD_PARTY_LICENSES.md`
- ONNX Runtime license at the pinned release: `https://github.com/microsoft/onnxruntime/blob/v1.23.2/LICENSE`

## Linked Local TTS dependencies

The application links the Rust `ort` and `piper-plus-g2p` crates. They are **shipped dependencies**, unlike the installer-downloaded artifacts above, so their package license evidence belongs in the generated release dependency inventory produced by `scripts/collect_release_licenses.py`.

The accepted feature policy is deliberately narrow:

- `ort = 2.0.0-rc.13`, exact-pinned, default features disabled, with only `std`, `api-23`, and `load-dynamic` enabled;
- `piper-plus-g2p = 0.4.0`, exact-pinned, default features disabled, with only `english` enabled;
- `bundled-dicts` is not enabled;
- no eSpeak/eSpeak-ng feature, library, executable, dictionary, or GPL payload is approved for Local TTS V1.

The piper-plus upstream evidence explicitly describes `piper-plus-g2p` as MIT-licensed and eSpeak-ng-free. Its third-party document identifies the CMU English dictionary separately as BSD-style (CMU), which is why Talking Moose records the G2P crate and the downloaded dictionary as distinct licensing entries rather than inferring one from the other.

## Release policy

Before a release may be accepted:

1. `scripts/check_local_tts_packaging_policy.py` must prove the model/runtime payloads remain external, immutable, checksum-pinned, HTTPS-only, and complete for Linux x86_64, macOS arm64, and macOS x86_64.
2. The policy gate must prove the accepted Cargo feature set still excludes unapproved eSpeak/GPL and bundled-dictionary payloads.
3. `scripts/collect_release_licenses.py` must find license evidence for the exact shipped `ort` and `piper-plus-g2p` crate versions in the resolved macOS dependency graph.
4. `docs/THIRD_PARTY_NOTICES.md` must continue to distinguish shipped dependency notices from installer-downloaded Local TTS assets.
5. A change to any Local TTS artifact URL, revision, checksum, byte count, license label, runtime version, Cargo feature, or supported platform requires a fresh licensing/provenance review; CI must fail closed until the evidence is updated.
