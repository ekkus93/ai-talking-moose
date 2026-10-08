# Whisper Local ASR — Model & Source License Record

Recorded: 2026-10-07
Spec: `docs/WHISPER_CPP_LOCAL_ASR_SPEC.md`
Native source commit: `60c0be6ac8fa71b1a2ae2dd938a31a34a508e774` (ggml-org/whisper.cpp, vendored at `third_party/whisper.cpp`)
Model artifact revision: `5359861c739e955e79d9a303bcbc70fb988958b1` (Hugging Face `ggerganov/whisper.cpp`)

## Vendored source

| Artifact | Location | License |
|---|---|---|
| whisper.cpp source tree | `third_party/whisper.cpp` | MIT — `third_party/whisper.cpp/LICENSE` ("Copyright (c) 2023-2026 The ggml authors") |

## Model weights

| Artifact | Source | Size (bytes) | License |
|---|---|---:|---|
| `ggml-small.bin` | https://huggingface.co/ggerganov/whisper.cpp/resolve/5359861c739e955e79d9a303bcbc70fb988958b1/ggml-small.bin?download=true | 487,601,967 | **MIT** |

The authoritative statement is the upstream model repository metadata:
`https://huggingface.co/ggerganov/whisper.cpp` (license field: `mit`),
commit `5359861c739e955e79d9a303bcbc70fb988958b1`.
The same MIT license governs the vendored whisper.cpp source tree, so the
model weights and the vendored runtime share a compatible permissive license.

SHA-256 is pinned in `src-tauri/src/asr/whisper/manifest.rs` as
`1be3a9b2063867b937e64e2ec7483364a79917e157fa98c5d94b5c1fffea987b`.
The pinned byte size matches the upstream asset `x-linked-size`. A live
SHA-256 byte-for-byte result from a fresh download is not recorded yet. The
manual real-CPU acceptance workflow installs through the production verifier
and independently hashes the installed file before recording evidence.

The macOS release preparation step verifies the pinned source checkout and
stages the exact upstream `LICENSE` text at
`src-tauri/native/macos/notices/WhisperRuntime/WHISPER_CPP_LICENSE` for the
Tauri resource bundle.
