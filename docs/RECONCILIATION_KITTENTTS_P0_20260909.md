# AI Talking Moose — KittenTTS P0 Reconciliation

**Tracker:** `docs/TODO(20260909-120003).md`
**Specification:** `docs/SPEC(20260909-120003).md`
**Implementation branch base:** `e0fc7338433bc1a5b8133339a12cdd03ae3daa0b`
**Source architecture baseline recorded by the tracker:** `80027a70815bfa8db6c9faa03b3a8e7501f2fe4d`
**Validated implementation head before history collapse:** `61a8c5675cf60bc64fce27ca3edbb7095f6d8840`
**Authoritative P0 evidence run:** `34407263250`
**Status:** P0 COMPLETE — clean-head validation, guarded merge, and exact merged-master CI accepted 2026-09-09

## 1. KTT-001 architecture freeze

The Local TTS project preserves the production speech architecture rather than introducing a parallel playback stack.

Confirmed boundaries:

1. `src-tauri/src/ai/traits.rs::SpeechSynthesizer` is the provider-neutral standalone synthesis abstraction.
2. `src-tauri/src/commands/speech.rs::invoke_standalone_speech()` is the authoritative standalone speech path used by typed replies, ambient remarks, character reactions, and voice auditions.
3. `AudioStreamData` remains the provider boundary for synthesized PCM.
4. `src-tauri/src/audio/playback.rs::AudioPlayback` and CPAL remain the authoritative playback path, including device selection, resampling, gain, bounded queueing, output-level diagnostics, and mouth-shape generation.
5. `StandaloneSpeechController` remains the authoritative preemption/cancellation owner for standalone speech.
6. Gemini Live native audio is separate from standalone TTS. Live `AudioData` events are enqueued directly to `AudioPlayback`; KittenTTS must not route Gemini Live through `SpeechSynthesizer`.
7. Baseline settings version is 3.
8. Baseline `tts_voice` is overloaded between Google standalone TTS and Gemini Live and must be split before a Kitten voice ID becomes persistable.
9. Baseline `tts_model` means the Google standalone TTS model.
10. The repository license is Apache-2.0.

No P0 work adds a second playback stack or changes Gemini Live behavior.

## 2. KTT-002 model and corpus freeze

Selected V1 model family:

`KittenML/kitten-tts-mini-0.8`

Recorded upstream properties:

- license metadata: Apache-2.0;
- architecture: ONNX / StyleTTS 2 family;
- parameter count: 80M;
- published approximate model size: 79 MB;
- source sample rate: 24,000 Hz;
- voices: Bella, Jasper, Luna, Bruno, Rosie, Hugo, Kiki, Leo;
- model file: `kitten_tts_mini_v0_8.onnx`;
- voice embedding file: `voices.npz`.

The deterministic reference corpus is version-controlled at `tests/fixtures/kittentts_p0_reference_corpus.json` and contains 25 cases spanning ordinary Moose text, punctuation, contractions, numbers, dates/times, currency, abbreviations/initialisms, possessives, proper names, units, hyphenation, and OOV words.

The successful P0 run resolved the upstream model to immutable revision:

`c02725660cea441db4c383af69f1f26f5cd00947`

Observed artifacts at that revision:

| Artifact | Bytes | SHA-256 |
| --- | ---: | --- |
| `config.json` | 470 | `6b160bc9b19e24ecb21e84bc14f8a7da21fdf47ec72d42450bc5cf514b61804a` |
| `kitten_tts_mini_v0_8.onnx` | 78,268,016 | `0f5bbae4fc4800c98dbc544a87ecfa79510de2fb8222db30d12e5bfe9177df91` |
| `voices.npz` | 3,278,902 | `40ad2638952b77b7b2f30127e2608e169fc69dd256b53bd8aaa3409a33193c42` |

These values are P0 evidence and are suitable inputs to the later KTT-200 production catalog. KTT-200 remains responsible for defining the complete immutable production manifest and notice provenance.

## 3. Reference oracle and production boundary

The official KittenTTS v0.8 Python path performs Kitten text preprocessing/normalization, then uses `phonemizer` with eSpeak NG to produce stressed en-US IPA before Kitten token mapping and ONNX inference.

P0 uses that Python/eSpeak path only as a **reference oracle in CI**. It is not part of the selected production candidate and must not be bundled, linked, copied into application resources, or downloaded by the application-managed Local TTS runtime.

## 4. KTT-003 licensing result

### Rejected production paths

P0 rejects:

- official Kitten Python + `phonemizer` / `espeakng_loader` as a shipped runtime;
- current sherpa-onnx Kitten packages that require/carry eSpeak data;
- Piper/Piper1-GPL as the Local TTS engine;
- `kittentts-rs` as the selected production carrier after its broader convenience dependency graph failed the project's strict permissive-only selection policy;
- any `kittentts-rs` configuration that enables eSpeak;
- any implicit support-asset download outside the future Local TTS catalog;
- any Local→Google fallback when Local synthesis fails.

### Selected permissive production candidate

P0 selects a narrow application-owned Rust adapter rather than `kittentts-rs`:

- model: KittenTTS Mini 0.8;
- inference adapter: `tools/kittentts-p0`, owning Kitten tensor construction, NPZ voice loading, token framing, and bounded ONNX Runtime thread configuration;
- Rust ONNX binding: `ort = 2.0.0-rc.13`, API-23 compatibility;
- ONNX Runtime: Microsoft ONNX Runtime 1.23.2, CPU, supplied as explicitly pinned external runtime artifacts;
- G2P: `piper-plus-g2p = 0.4.0`, `default-features = false`, English only;
- G2P upstream audit revision: `244ffeb44108347a514ebfc0c2f773d938c9613b`;
- CMUdict-derived data: 3,748,042 bytes, SHA-256 `f78ed3bfa40146b70b9de78a7c67ef3be6b6c11671bb2a050442f232252a3f01`;
- Kitten input framing: exact application-owned `[0, …, 10, 0]` framing around the mapped IPA token sequence;
- playback: unchanged `AudioPlayback` / CPAL.

The candidate Cargo graph is machine-audited with `tools/kittentts-p0/license_gate.py`. The gate evaluates SPDX `AND`/`OR` alternatives instead of rejecting an entire dependency merely because one optional branch is copyleft. It still rejects dependencies for which no permissive license choice exists. The workflow separately fails if the production candidate graph contains eSpeak, `phonemizer`, or `kittentts`.

The exact selected candidate graph passed that gate on Linux x86_64, macOS arm64, and macOS x86_64 in run `34407263250`.

## 5. Pinned ONNX Runtime artifacts

The P0 matrix does not rely on Pyke's binary downloader or Homebrew. It downloads Microsoft ONNX Runtime 1.23.2 release artifacts and verifies exact SHA-256 values before compilation/inference:

| Platform | Artifact | SHA-256 |
| --- | --- | --- |
| Linux x86_64 | `onnxruntime-linux-x64-1.23.2.tgz` | `1fa4dcaef22f6f7d5cd81b28c2800414350c10116f5fdd46a2160082551c5f9b` |
| macOS arm64 | `onnxruntime-osx-arm64-1.23.2.tgz` | `b4d513ab2b26f088c66891dbbc1408166708773d7cc4163de7bdca0e9bbb7856` |
| macOS x86_64 | `onnxruntime-osx-x86_64-1.23.2.tgz` | `d10359e16347b57d9959f7e80a225a5b4a66ed7d7e007274a15cae86836485a6` |

The workflow records ONNX Runtime's MIT license. The later production catalog must carry the final runtime artifacts and notices explicitly rather than making CI's download procedure an implicit runtime contract.

## 6. KTT-004 compatibility result

The selected eSpeak-free frontend is **not byte-for-byte equivalent** to the official eSpeak reference, and P0 does not claim otherwise.

Across the 25-case frozen corpus:

- exact model-input matches: 1 / 25;
- aggregate normalized token edit distance: `0.17243589743589743`;
- representative cases synthesized by the real Kitten model per thread configuration: 8.

Largest measured divergences include:

| Case | Normalized token edit distance |
| --- | ---: |
| `oov_01` | 0.8276 |
| `initialism_01` | 0.5000 |
| `proper_02` | 0.3966 |
| `hyphen_01` | 0.3455 |
| `unit_01` | 0.2692 |
| `possessive_01` | 0.2500 |
| `percent_01` | 0.2329 |
| `currency_01` | 0.1364 |
| `currency_scale_01` | 0.1316 |
| `ordinary_03` | 0.1311 |

This satisfies the specification's feasibility requirement by comparing against the official reference and making meaningful drift explicit rather than assuming compatibility. It does **not** authorize treating the selected G2P as pronunciation-identical to eSpeak. OOV words, initialisms, proper names, hyphenation, and units require focused behavioral coverage during production frontend integration. The production Rust normalizer must preserve the relevant Kitten normalization behavior before G2P.

## 7. CPU performance evidence

All three required targets built the direct Rust adapter, loaded the real Kitten Mini model on CPU, synthesized non-empty finite audio, and uploaded comparison evidence in run `34407263250`.

The 2-thread configuration is the P0-selected conservative default because it gave the best synthesis RTF of the tested 1/2/4-thread configurations on all three required platforms.

| Platform | 2-thread cold load | 2-thread mean RTF | 2-thread max RTF | RSS after load | RSS after synthesis |
| --- | ---: | ---: | ---: | ---: | ---: |
| Linux x86_64 | 417.2 ms | 0.6510 | 0.6539 | 331.3 MiB | 340.9 MiB |
| macOS arm64 | 607.9 ms | 0.4560 | 0.4963 | 385.7 MiB | 445.3 MiB |
| macOS x86_64 | 1254.2 ms | 1.1938 | 1.5632 | 328.7 MiB | 334.2 MiB |

Thread-count observations:

- Linux x86_64: 2 threads outperformed both 1 and 4 threads for synthesis RTF.
- macOS arm64: 2 threads delivered the best synthesis RTF; 4 threads consumed more memory and synthesized more slowly despite a somewhat lower cold-load time.
- macOS x86_64: 2 threads was clearly best of the tested configurations, but representative worst-case synthesis remained slower than real time (`max RTF 1.5632`, mean `1.1938`). Intel macOS is therefore build/runtime feasible but carries a documented performance limitation.

P0 does not claim that Intel macOS is real-time for every utterance. The requirement proved here is CPU feasibility and cross-platform build/runtime viability; later acceptance must preserve truthful performance expectations.

## 8. Authoritative evidence

Authoritative specialized run:

- workflow run: `34407263250`;
- validated head: `61a8c5675cf60bc64fce27ca3edbb7095f6d8840` before history cleanup;
- `Freeze P0 Cargo dependency graph`: PASS;
- `CPU probe (linux-x86_64)`: PASS;
- `CPU probe (macos-arm64)`: PASS;
- `CPU probe (macos-x86_64)`: PASS.

Evidence artifacts:

| Artifact | Artifact ID | ZIP SHA-256 |
| --- | ---: | --- |
| `kittentts-p0-linux-x86_64` | 10125888509 | `5c422f591f32091e6c603ecb1c65c9ccb305fbce692d20567e1e9a7fce11cd05` |
| `kittentts-p0-macos-arm64` | 10125895901 | `93a414d6aac46458b50520eca0cfd1790f0043cd2491717b6c3ec5ef11c4dd42` |
| `kittentts-p0-macos-x86_64` | 10126033123 | `f7c9eb75a229949f3a205adc10d965ffe998a07b91b05cc052d855c5ab9a98b5` |

GitHub reduced the requested artifact retention to the repository maximum of seven days. Durable evidence is therefore the checked-in harness, exact revisions/hashes recorded here, and the successful workflow/run metadata rather than indefinite artifact availability.

## 9. P0 disposition

The implementation evidence supports the following P0 selection:

1. **Model:** KittenTTS Mini 0.8.
2. **Runtime:** direct application-owned Rust ONNX adapter + `ort 2.0.0-rc.13` + pinned Microsoft ONNX Runtime 1.23.2 CPU artifacts.
3. **G2P:** `piper-plus-g2p 0.4.0` English-only with pinned CMUdict-derived data.
4. **Default inference threads:** 2.
5. **Reference oracle:** official Kitten Python/eSpeak path in CI only; never shipped.
6. **Licensing:** selected production candidate remains permissive; eSpeak/phonemizer/copyleft-only dependencies are excluded from the candidate graph.
7. **Compatibility caveat:** deterministic token drift is material for some OOV/initialism/proper-name cases and must remain visible in production frontend tests.
8. **Performance caveat:** macOS Intel is feasible but can be slower than real time.

KTT-001 through KTT-004 are accepted and closed. The tracker P0 section is checked only after the clean-head validation, guarded merge, and exact merged-master CI evidence below were all successful.

## 10. Final P0 closure evidence — 2026-09-09

The iterative P0 branch was collapsed to one clean commit on accepted planning master `e0fc7338433bc1a5b8133339a12cdd03ae3daa0b`.

Final clean implementation head:

`0e5d9fdbe9c006fa505f3e3e13db6d8c7a7c1b2d`

Exact-head gates on that commit:

- ordinary repository CI `34409269416`: PASS;
- specialized KittenTTS P0 feasibility run `34409217053`: PASS;
- specialized matrix included Linux x86_64, macOS arm64, and macOS x86_64 real-model CPU probes plus the permissive-license gate.

PR #71 (`test: prove KittenTTS P0 feasibility`) was non-draft, mergeable, and still pointed exactly at `0e5d9fdbe9c006fa505f3e3e13db6d8c7a7c1b2d` immediately before merge. It was squash-merged with `expected_head_sha` guarding that exact head.

Resulting implementation master:

`bb76b73782063ba7da4548de76108fc93b82bee0`

Exact post-merge master CI:

- run `34410662180`: PASS;
- event: push;
- head SHA: `bb76b73782063ba7da4548de76108fc93b82bee0`.

P0 is therefore procedurally and technically accepted. The next tracker phase is P1, beginning with KTT-100. The G2P drift and Intel macOS performance caveats recorded above remain design constraints for later implementation; closing P0 does not erase them.
