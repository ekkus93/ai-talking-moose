# Third-Party Dependency Notice Inventory

Status: **V1 release notice inventory**
Recorded: 2026-09-12

This file records licenses and immutable source identities for native components that Talking Moose expects to redistribute or directly depend on. Release packaging must include the actual license/notice texts required by those components; this inventory is not a substitute for those texts.

## Local Whisper.cpp runtime and model

| Component | Pin / identity | License | Notes |
| --- | --- | --- | --- |
| whisper.cpp source | `ggml-org/whisper.cpp`; vendored commit `60c0be6ac8fa71b1a2ae2dd938a31a34a508e774` | MIT | Built through `src-tauri/build.rs` on supported Linux and macOS local-ASR targets; macOS release preparation verifies this checkout and stages its exact upstream MIT notice in the Tauri resource path. |
| `ggml-small.bin` weights | Hugging Face asset `ggml-small.bin` in `ggerganov/whisper.cpp`, source commit `5359861c739e955e79d9a303bcbc70fb988958b1`: `https://huggingface.co/ggerganov/whisper.cpp/resolve/5359861c739e955e79d9a303bcbc70fb988958b1/ggml-small.bin?download=true`; 487601967 bytes; SHA-256 `1be3a9b2063867b937e64e2ec7483364a79917e157fa98c5d94b5c1fffea987b` | MIT | Downloaded only after explicit user action; not bundled. The model repository metadata records the artifact license as MIT; exact SHA-256 and byte size are enforced by the production installer. |

## Local LLM native runtime

| Component | Pin / identity | License | Notes |
| --- | --- | --- | --- |
| `llama-cpp-2` Rust binding | `utilityai/llama-cpp-rs` `0.1.154`, commit `bed81ad4ab1a6c904b11d425608e50f976d8ea62` | MIT OR Apache-2.0; redistributed under MIT | High-level Rust API linked into the application; upstream MIT text is bundled. |
| `llama-cpp-sys-2` Rust/sys binding | `utilityai/llama-cpp-rs` `0.1.154`, same release commit | MIT OR Apache-2.0; redistributed under MIT | Builds the vendored native llama.cpp sources; no system llama.cpp dependency is required. |
| llama.cpp / ggml native source | submodule commit `5f55650a78f92aff4d48d671423e888fac0469ff` in `llama-cpp-sys-2` 0.1.154 | MIT | Statically linked CPU inference runtime. The pinned MIT text is bundled as `LocalLlmRuntime/LLAMA_CPP_LICENSE`. |

GGUF catalog models are downloaded only after an explicit user install action and are not redistributed in the application bundle. Their source/revision/license metadata is maintained separately in `docs/LOCAL_LLM_MODEL_LICENSES.md`.

## Local TTS runtime and downloaded assets

The Local TTS licensing boundary is split deliberately between dependencies that are part of the application binary and artifacts that the installer downloads from upstream after explicit user action.

### Shipped Local TTS dependencies

| Component | Pin / identity | License evidence | Notes |
| --- | --- | --- | --- |
| `ort` Rust crate | exact `2.0.0-rc.13`; default features disabled; `std`, `api-23`, `load-dynamic` only | Generated from the resolved Cargo package by `scripts/collect_release_licenses.py` | Linked application dependency. The ONNX Runtime dynamic library itself is not bundled for Local TTS; it is downloaded and verified separately. |
| `piper-plus-g2p` Rust crate | exact `0.4.0`; default features disabled; `english` only | Generated from the resolved Cargo package by `scripts/collect_release_licenses.py`; upstream identifies the crate as MIT | Linked application dependency. No eSpeak/eSpeak-ng dependency or GPL payload is approved. |

### Installer-downloaded Local TTS assets

| Component | Pin / identity | License | Notes |
| --- | --- | --- | --- |
| KittenTTS Mini 0.8 ONNX model | `KittenML/kitten-tts-mini-0.8` revision `c02725660cea441db4c383af69f1f26f5cd00947` | Apache-2.0 repository/model-card metadata | Downloaded only after explicit Local TTS install; not bundled. |
| KittenTTS `voices.npz` embeddings | same immutable KittenTTS revision | Apache-2.0 repository/model-card metadata at the artifact-containing revision | Downloaded only after explicit Local TTS install; not bundled. |
| CMUdict JSON | `ayutaz/piper-plus` revision `244ffeb44108347a514ebfc0c2f773d938c9613b` | BSD-style (CMU), explicitly identified in upstream `piper-plus-g2p` third-party notices | Downloaded only after explicit Local TTS install; not compiled into the app. |
| ONNX Runtime CPU archive | Microsoft ONNX Runtime `v1.23.2`, platform-specific Linux x86_64/macOS arm64/macOS x86_64 archives | MIT, upstream `LICENSE` at tag `v1.23.2` | Downloaded only after explicit Local TTS install; exact archive bytes/SHA-256 are catalog-pinned. |

Exact artifact filenames, URLs, sizes, hashes, and evidence locations are recorded in `docs/LOCAL_TTS_ASSET_LICENSES.md` and enforced by `scripts/check_local_tts_packaging_policy.py`. These downloaded assets are intentionally **not** copied into the generated application-bundle dependency notice tree, because doing so would misstate what the application actually redistributes.

## macOS secure credential dependencies

| Component | V1 version | License |
| --- | ---: | --- |
| `security-framework` Rust crate | 3.7.0 | MIT OR Apache-2.0 |
| `security-framework-sys` Rust crate | 2.17.0 | MIT OR Apache-2.0 |

These crates are thin Rust bindings around Apple's Security framework and are used only on macOS to store the Google API key in Keychain.

## Release requirements

Before producing a signed/notarized V1 distribution:

- run `scripts/prepare_macos_notices.sh` to stage the Talking Moose project license and pinned Whisper notice;
- the same preparation step verifies the pinned whisper.cpp checkout and stages its upstream `LICENSE` text at `Resources/native/macos/notices/WhisperRuntime/WHISPER_CPP_LICENSE`;
- run `scripts/collect_release_licenses.py` after `npm ci` and Cargo dependency resolution to collect production npm and resolved Rust license/notice texts plus a generated dependency inventory, including `llama-cpp-2`, `llama-cpp-sys-2`, `ort`, and `piper-plus-g2p`;
- run `scripts/check_local_tts_packaging_policy.py` so Local TTS model/runtime assets remain externally installed, immutable/checksum-pinned, model-weight-free in the app bundle, and free of unapproved eSpeak/GPL payloads;
- include the generated inventories and license/notice texts under the application bundle's `Resources/native/macos/notices` path, including the staged Whisper source license and checked-in `LocalLlmRuntime/LLAMA_CPP_LICENSE` native notice and `LocalLlmRuntime/LLAMA_CPP_RS_LICENSE_MIT` binding notice;
- keep installer-downloaded Local TTS artifact evidence in `docs/LOCAL_TTS_ASSET_LICENSES.md` rather than representing those external assets as bundled application resources;
- verify that local ASR model downloads remain limited to the pinned Whisper Small manifest;
- verify that the application bundle does not accidentally contain developer model caches, GGUF weights, Local TTS ONNX/voice/dictionary/runtime archives, or credentials.
