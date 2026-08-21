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
| Asset archive | `moonshine-ai/moonshine-voice-assets` |
| Asset archive commit | `35d84fc0eb2d7451da9973c990e8a77066abb105` |
| Asset SHA-256 inventory | `TRANSFER_REPORT.tsv` at the pinned archive commit |
| ONNX Runtime version in the pinned native tree | `1.23.2` on macOS |

V1 shall build/package the native runtime from the pinned Moonshine source revision rather than resolving `main`, `latest`, or another moving reference. A future runtime upgrade is an explicit dependency update with a model/runtime compatibility review.

The C API constants required by V1 at this pin are:

- `MOONSHINE_MODEL_ARCH_TINY_STREAMING = 2`
- `MOONSHINE_MODEL_ARCH_SMALL_STREAMING = 4`

The FFI implementation must pass the pinned `MOONSHINE_HEADER_VERSION` and verify runtime/header compatibility as specified by the upstream API.

## 2. Model policy

Talking Moose V1 uses the English native streaming model payloads selected by Moonshine `v0.1.3`. The upstream catalog deliberately places each quantized revision in a dated directory rather than overwriting an existing payload. V1 pins `quantized_26_07_30`. The official `moonshine-ai/moonshine-voice-assets` archival mirror preserves the exact CDN paths and records an independently computed SHA-256 for each copied file; V1 pins archive commit `35d84fc0eb2d7451da9973c990e8a77066abb105` for those SHA-256 values.

Word-level timestamps are **not required for V1**. Therefore the application model manager shall install the seven files listed below and shall not download `decoder_kv_with_attention.ort` unless a later feature explicitly enables word timestamps.

### 2.1 Moonshine Tiny Streaming — V1 default

Stable Talking Moose model ID: `moonshine-tiny-streaming-en`  
Moonshine architecture: `MOONSHINE_MODEL_ARCH_TINY_STREAMING`  
Native payload revision: `quantized_26_07_30`  
Native base path: `https://download.moonshine.ai/model/tiny-streaming-en/quantized_26_07_30/`  
Source model: `UsefulSensors/moonshine-streaming-tiny`  
Source model commit: `f8e9dfd8c562c257c151a907b7b7f2fe8ff8511a`

| Required file | Bytes | SHA-256 | Upstream CRC32C (base64) |
| --- | ---: | --- | --- |
| `adapter.ort` | 1,319,664 | `22ecc949e146c49667fda28d102d4e30749a107dc88a396292aa8f277ef1347c` | `kwQ+Bw==` |
| `cross_kv.ort` | 1,287,544 | `143a36667b8d05fd9d04e8c337b7ee121f37ef299aea6b3d82bdb3d3401950b4` | `76wzFQ==` |
| `decoder_kv.ort` | 32,583,720 | `8852553f312adb6c9aa4d17418015049b30f412209ee569d336548c0044627de` | `KJjeNw==` |
| `encoder.ort` | 7,675,440 | `a8414e1a5dedf9f2093d7680601dd8a9b0433e7020260eafe0e370ead91134ca` | `UjAIpQ==` |
| `frontend.ort` | 8,324,920 | `271a563251f11e6311949530f8025ed4d345c5d69d4ac1efa74093779927d636` | `AW9qsg==` |
| `streaming_config.json` | 509 | `74fe5ddebd63b17caf59e8a3b18c17547ff7bce1642050edbb1c3962674f8950` | `HGL0Ug==` |
| `tokenizer.bin` | 249,974 | `6884b35fd6377d4c4d32336a0bc152f36b64d1e45b6503683cdc238250a8472d` | `B7s10Q==` |

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

| Required file | Bytes | SHA-256 | Upstream CRC32C (base64) |
| --- | ---: | --- | --- |
| `adapter.ort` | 2,870,368 | `c665f742364febad597cc9ac1e0b341ffbee0e24a1466e2f3bde95e6e4771762` | `XlWjTg==` |
| `cross_kv.ort` | 5,356,536 | `e2d3417144e9514055ebfefe8dcc4c0a55a55adcb8530435844c75c53e352bf6` | `5ySK8g==` |
| `decoder_kv.ort` | 81,878,600 | `1a05465b1dd955858dfcbee039c0020fb5dd982b0f5094c34e61735d518d771b` | `Rn/VUA==` |
| `encoder.ort` | 44,148,576 | `2d4d973e91e8aca08c51e7e7efa28a46ab265b63d809d5294d18b86bcd85b993` | `41D2jQ==` |
| `frontend.ort` | 30,984,520 | `d1be8145bc9ef3e8625bb7bcb7a5b2930eeb828c91d6c1897bc215d7de6f2b93` | `m6NCLA==` |
| `streaming_config.json` | 512 | `26f02b6afb22d60871a5efd85c3d38e569cc0ddb6c5eb6e93d3260152ae8a47a` | `dPbFiw==` |
| `tokenizer.bin` | 249,974 | `6884b35fd6377d4c4d32336a0bc152f36b64d1e45b6503683cdc238250a8472d` | `B7s10Q==` |

Expected installed payload size, excluding filesystem metadata: **165,489,086 bytes** (~157.8 MiB).

Optional upstream word-timestamp file, not part of the V1 manifest:

- `decoder_kv_with_attention.ort`: 81,766,608 bytes, CRC32C `IzJXVA==`.

## 3. Integrity rules

The upstream `v0.1.3` catalog exposes a byte count and CRC32C value for each native model component. The official Moonshine asset archive independently records SHA-256 for the exact CDN copies. Talking Moose pins all three values: SHA-256 is the application-owned cryptographic integrity check, while byte count and upstream CRC32C remain secondary cross-checks.

Before V1 model downloading is considered complete:

1. every download must use HTTPS;
2. every component must be downloaded to a temporary path;
3. SHA-256 must match the pinned manifest before installation;
4. byte count and upstream CRC32C must also match the pinned manifest;
5. a failed verification must never replace an existing known-good model;
6. the verified directory must be promoted atomically where the platform/filesystem allows it;
7. the installed revision must be recorded as `quantized_26_07_30`;
8. partial files are never reported as installed;
9. a production manifest update requires code review and a deliberate provenance change.

The SHA-256 inventory is pinned to the official asset archive commit above. A future manifest update must re-verify the CDN catalog, archive transfer report, source-model provenance, and licenses together; changing only a URL or checksum is not sufficient.

## 4. Licensing and redistribution baseline

- Moonshine Voice first-party runtime code: **MIT**.
- Moonshine English Tiny Streaming model: **MIT**.
- Moonshine English Small Streaming model: **MIT**.
- ONNX Runtime: **MIT**.
- Moonshine's pinned source tree includes third-party components with their own licenses. The release packaging task must preserve their required notices. The current inventory is tracked in `docs/THIRD_PARTY_NOTICES.md`.

The current V1 model scope is English only. Do not assume Moonshine models for other languages carry the same license; evaluate each additional model separately before adding it to the product.

## 5. macOS application packaging contract

Talking Moose packages Moonshine as application-owned native code rather than relying on Homebrew, `/usr/local`, a developer checkout, or shell startup files. The checked-in machine-readable source of truth is `src-tauri/native/moonshine-runtime.json`.

V1 deliberately supports both macOS CPU architectures that Moonshine supports:

| macOS architecture | Rust target | Pinned ONNX Runtime 1.23.2 dylib SHA-256 | Bytes |
| --- | --- | --- | ---: |
| Apple Silicon `arm64` | `aarch64-apple-darwin` | `480790a978a48ad3e06ce86b6025e037bd70221637f5c104a8dde19617364cf4` | 27,623,992 |
| Intel `x86_64` | `x86_64-apple-darwin` | `8c9c78de65ea3786f987c0d980e9c1b13a3a5fbc6b3e2965ba05b450e6e4c054` | 39,742,608 |

`scripts/prepare_moonshine_macos.sh` performs the reproducible native preparation step on the matching host architecture. It:

1. checks out exactly Moonshine commit `db88bffd14574212b6094a2e230d4f328029c31b`;
2. materializes only the architecture-specific ONNX Runtime dylib from Git LFS;
3. verifies that dylib against the pinned byte count and SHA-256 above;
4. builds the `moonshine` CMake target for the host architecture with deployment target 10.15;
5. stages `libmoonshine.dylib` and `libonnxruntime.1.23.2.dylib` under `src-tauri/native/macos/`;
6. normalizes their install names to `@rpath` and rejects Homebrew/developer-machine load paths; and
7. copies the pinned runtime provenance plus license/notice files from the source tree into the generated notice staging directory.

The generated dylibs are not committed. `src-tauri/tauri.conf.json` declares both as `bundle.macOS.frameworks`, so Tauri copies them to `Talking Moose AI.app/Contents/Frameworks` and supplies the executable Frameworks rpath. Tauri's framework/dylib bundle path is also the path covered by its macOS nested-code signing when signing is configured. The release signing/notarization credential workflow remains owned by V1R-131; ASR-016 ensures Moonshine and ONNX Runtime participate in that signing graph instead of living outside the app bundle.

`bundle.resources` also includes the generated native notice directory. `scripts/verify_macos_bundle.sh` validates the produced `.app` by checking architecture, `@rpath` load commands, absence of developer-machine paths, the bundled notice set, nested signatures whenever the app is signed, and an actual executable smoke mode (`--moonshine-native-smoke-test`) launched with a minimal environment. The smoke mode calls `moonshine_get_version()` from the bundled dylib before Tauri starts, so it proves the installed application can load the native runtime without a model, Homebrew, or a developer library path.

CI runs this preparation/build/verification path separately on current GitHub-hosted Apple Silicon and Intel macOS runners. Untagged commits retain no bundle artifact; tagged commits keep architecture-labelled application/DMG artifacts only, preserving the repository's existing release-asset policy.

## 6. Update procedure

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
