# Third-Party Dependency Notice Inventory

Status: **V1 working notice inventory**  
Recorded: 2026-08-18

This file records licenses and immutable source identities for native components that Talking Moose expects to redistribute or directly depend on. Release packaging must include the actual license/notice texts required by those components; this inventory is not a substitute for those texts.

## Local ASR runtime and models

| Component | Pin / identity | License | Notes |
| --- | --- | --- | --- |
| Moonshine Voice | `moonshine-ai/moonshine` `v0.1.3`, commit `db88bffd14574212b6094a2e230d4f328029c31b` | MIT | First-party runtime code outside `core/third-party`. |
| Moonshine Tiny Streaming English | native `quantized_26_07_30`; source commit `f8e9dfd8c562c257c151a907b7b7f2fe8ff8511a` | MIT | V1 default local ASR model. |
| Moonshine Small Streaming English | native `quantized_26_07_30`; source commit `2c036506f23a09c18df5a50057599ba6d9280999` | MIT | Optional higher-accuracy local ASR model. |
| ONNX Runtime | Moonshine `v0.1.3` vendored runtime; macOS CMake path identifies `1.23.2` | MIT | Native inference dependency. |

Exact model component URLs, sizes, and checksums are recorded in `docs/MOONSHINE_NATIVE.md`.

## Moonshine vendored native dependencies

These identities refer to the subtrees included by the pinned Moonshine runtime. Only components that actually reach the Talking Moose release artifact need to be reproduced in the final bundled-notices payload, but the build/release audit must begin from this complete native-tree inventory.

| Component | Pinned Moonshine subtree | License / notice source |
| --- | --- | --- |
| cpp-annote | source tree under Moonshine `v0.1.3`; `src` tree `2af8b2cf4d675e7e7053351db855379b60d7bb71` | MIT; `LICENSE` blob `63148e08d3b99e9249e37d398260329b7ca979a9` |
| Eigen | tree `854e3e24b303a00db385ff10b0dfa8b4a2c2280c` | MPL-2.0 subset; `COPYING.MPL2` blob `14e2f777f6c395e7e04ab4aa306bbcc4b0c1120e` |
| doctest | tree `4ca3b4ea819d9a456eec3dd4394fcd479e75ba4d` | MIT; `LICENSE.txt` blob `5ae0eb1052c28da584bcf2fdcc6b7ef45c30952c` |
| kaldi-native-fbank | tree `4f5350e5cd33535b47b74122178bb1b278626c29` | Apache-2.0; `LICENSE` blob `d645695673349e3947e8e5ae42332d0ac3164cd7` |
| kissfft | tree `ac532e99e00422cc9e06b29d2f2185c909da04c0` | BSD-3-Clause; `COPYING` blob `6b4b622e7bd7053be8531deaf4242603bca4c85b` |
| nlohmann/json | tree `4cdff2a7534036a8d775c0531b1a3868d2d9d338` | MIT; `LICENSE.MIT` blob `1c1f7a690d815db3a79ea3d4c9138a497e2e7702` |
| utf8cpp (`utf-8`) | tree `7a47e860bac3e6db6785261abc68566d1ac3053d` | Boost Software License 1.0; `LICENSE.txt` blob `36b7cd93cdfbac762f5be4c6ce276df2ea6305c2` |
| utf8proc | tree `06bd0edcb1ec6e74aa175ffdabbb56138f6a9cbb` | MIT; `LICENSE.md` blob `f18b1f3abdcb55e82ecdd84f476e7f366b00183b` |
| ONNX Runtime subtree | tree `d31a828135464f018d708432bf0bd1b224f7ce8e` | MIT; preserve upstream ONNX Runtime notices shipped with the native runtime |

## macOS secure credential dependencies

| Component | V1 version | License |
| --- | ---: | --- |
| `security-framework` Rust crate | 3.7.0 | MIT OR Apache-2.0 |
| `security-framework-sys` Rust crate | 2.17.0 | MIT OR Apache-2.0 |

These crates are thin Rust bindings around Apple's Security framework and are used only on macOS to store the Google API key in Keychain.

## Release requirements

Before producing a signed/notarized V1 distribution:

- generate the actual third-party notice bundle from the dependency graph and pinned Moonshine tree;
- include all license texts required for redistributed native libraries;
- verify that no non-commercial Moonshine model has entered the release payload;
- verify that model downloads remain limited to the explicitly approved English Tiny/Small manifests;
- verify that the application bundle does not accidentally contain developer model caches or credentials.
