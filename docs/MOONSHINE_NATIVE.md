# Moonshine Native Runtime and Model Provenance

Status: **V1 pinned dependency baseline**  
Recorded: 2026-08-18

Talking Moose V1 uses the official Moonshine native runtime through its C API for local speech recognition. This document is the source-of-truth provenance record for the runtime and the two approved English streaming model payloads. Production builds and model downloads must not resolve a floating upstream branch or a mutable "latest" URL.

## 1. Runtime pin

| Item | V1 pin |
| --- | --- |
| Upstream | `moonshine-ai/moonshine` |
| Release | `v0.1.3` |
| Source commit | `db88bffd14574212b6094a2e230d4f328029c31b` |
| Moonshine version | `0.1.3` |
| C API header version | `30000` (`3.0.0`) |
| C header blob | `fe36c81db4a13ca6b71f2f79dbf635fb1c0c807b` |
| Model catalog blob | `253f783132fe27d592bb641a485e1472b250dda5` |
| Model integrity metadata blob | `0a215c9ae67d5835f82e450ac29ff1fc7f880013` |
| ONNX Runtime version in the pinned native tree | `1.23.2` on macOS |

V1 shall build/package the native runtime from the pinned Moonshine source revision rather than resolving `main`, `latest`, or another moving reference. A future runtime upgrade is an explicit dependency update with a model/runtime compatibility review.

The C API constants required by V1 at this pin are:

- `MOONSHINE_MODEL_ARCH_TINY_STREAMING = 2`
- `MOONSHINE_MODEL_ARCH_SMALL_STREAMING = 4`

The FFI implementation must pass the pinned `MOONSHINE_HEADER_VERSION` and verify runtime/header compatibility as specified by the upstream API.

## 2. Model policy

Talking Moose V1 uses the English native streaming model payloads selected by Moonshine `v0.1.3`. The upstream catalog deliberately places each quantized revision in a dated directory rather than overwriting an existing payload. V1 pins `quantized_26_07_30`.

Word-level timestamps are **not required for V1**. Therefore the application model manager shall install the seven files listed below and shall not download `decoder_kv_with_attention.ort` unless a later feature explicitly enables word timestamps.

### 2.1 Moonshine Tiny Streaming — V1 default

Stable Talking Moose model ID: `moonshine-tiny-streaming-en`  
Moonshine architecture: `MOONSHINE_MODEL_ARCH_TINY_STREAMING`  
Native payload revision: `quantized_26_07_30`  
Native base path: `https://download.moonshine.ai/model/tiny-streaming-en/quantized_26_07_30/`  
Source model: `UsefulSensors/moonshine-streaming-tiny`  
Source model commit: `f8e9dfd8c562c257c151a907b7b7f2fe8ff8511a`

| Required file | Bytes | Upstream CRC32C (base64) |
| --- | ---: | --- |
| `adapter.ort` | 1,319,664 | `kwQ+Bw==` |
| `cross_kv.ort` | 1,287,544 | `76wzFQ==` |
| `decoder_kv.ort` | 32,583,720 | `KJjeNw==` |
| `encoder.ort` | 7,675,440 | `UjAIpQ==` |
| `frontend.ort` | 8,324,920 | `AW9qsg==` |
| `streaming_config.json` | 509 | `HGL0Ug==` |
| `tokenizer.bin` | 249,974 | `B7s10Q==` |

Expected installed payload size, excluding filesystem metadata: **51,441,771 bytes** (~49.1 MiB).

Optional upstream word-timestamp file, not part of the V1 manifest:

- `decoder_kv_with_attention.ort`: 32,515,016 bytes, CRC32C `zFeWyQ==`.

### 2.2 Moonshine Small Streaming

Stable Talking Moose model ID: `moonshine-small-streaming-en`  
Moonshine architecture: `MOONSHINE_MODEL_ARCH_SMALL_STREAMING`  
Native payload revision: `quantized_26_07_30`  
Native base path: `https://download.moonshine.ai/model/small-streaming-en/quantized_26_07_30/`  
Source model: `UsefulSensors/moonshine-streaming-small`  
Source model commit: `2c036506f23a09c18df5a50057599ba6d9280999`

| Required file | Bytes | Upstream CRC32C (base64) |
| --- | ---: | --- |
| `adapter.ort` | 2,870,368 | `XlWjTg==` |
| `cross_kv.ort` | 5,356,536 | `5ySK8g==` |
| `decoder_kv.ort` | 81,878,600 | `Rn/VUA==` |
| `encoder.ort` | 44,148,576 | `41D2jQ==` |
| `frontend.ort` | 30,984,520 | `m6NCLA==` |
| `streaming_config.json` | 512 | `dPbFiw==` |
| `tokenizer.bin` | 249,974 | `B7s10Q==` |

Expected installed payload size, excluding filesystem metadata: **165,489,086 bytes** (~157.8 MiB).

Optional upstream word-timestamp file, not part of the V1 manifest:

- `decoder_kv_with_attention.ort`: 81,766,608 bytes, CRC32C `IzJXVA==`.

## 3. Integrity rules

The upstream `v0.1.3` catalog exposes a byte count and CRC32C value for each native model component. The Talking Moose model manager shall treat those values as the minimum authoritative integrity metadata for this pinned payload.

Before V1 model downloading is considered complete:

1. every download must use HTTPS;
2. every component must be downloaded to a temporary path;
3. byte count and CRC32C must match the pinned manifest before installation;
4. a failed verification must never replace an existing known-good model;
5. the verified directory must be promoted atomically where the platform/filesystem allows it;
6. the installed revision must be recorded as `quantized_26_07_30`;
7. partial files are never reported as installed;
8. a production manifest update requires code review and a deliberate provenance change.

A later hardening task may additionally record application-owned SHA-256 values for the downloaded payloads. That is additive; it must not weaken or replace the pinned upstream checks.

## 4. Licensing and redistribution baseline

- Moonshine Voice first-party runtime code: **MIT**.
- Moonshine English Tiny Streaming model: **MIT**.
- Moonshine English Small Streaming model: **MIT**.
- ONNX Runtime: **MIT**.
- Moonshine's pinned source tree includes third-party components with their own licenses. The release packaging task must preserve their required notices. The current inventory is tracked in `docs/THIRD_PARTY_NOTICES.md`.

The current V1 model scope is English only. Do not assume Moonshine models for other languages carry the same license; evaluate each additional model separately before adding it to the product.

## 5. Update procedure

A Moonshine runtime or model update is never automatic. An update PR must:

1. select a new immutable runtime commit/tag;
2. record the new C API/header version and relevant blob revisions;
3. regenerate and review the native model manifest;
4. verify license/redistribution terms again;
5. run ABI/layout tests for the Rust FFI;
6. run Tiny and Small streaming regression/latency tests;
7. run model migration/download tests;
8. update this document and `THIRD_PARTY_NOTICES.md` in the same change.

Until such a review lands, V1 remains pinned to the values above.
