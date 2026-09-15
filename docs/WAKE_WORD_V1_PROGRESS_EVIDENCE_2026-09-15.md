# Wake Word V1 progress evidence — 2026-09-15

This file records qualified evidence for completed Wake Word V1 work without claiming completion of later tasks.

## WW-000 — scope and implementation baseline

- Approved companion specification: `docs/WAKE_WORD_V1_SPEC_2026-09-14.md`.
- Implementation-baseline record: `docs/WAKE_WORD_V1_IMPLEMENTATION_BASELINE_2026-09-15.md`.
- Exact implementation base recorded before the first Wake Word V1 source change: `595016a09f33fee094045f5966bbddb2ea8eaa08`, a verified descendant of the TODO baseline `e638daa706e63f8bbb36b514322679c1c39e19a6`.
- Baseline evidence merged through PR #125; merge commit `1aff26dd1024e84e9498cbbfc3210cbe487ee224`.
- Exact PR-head CI run `34955186997` passed.
- Frozen V1 constraints remain those in the approved spec: sherpa-onnx KWS, `Hey, Moose`, disabled-by-default wake mode, existing manual/ASR/TTS provider behavior preserved, Local TTS production thread policy unchanged, and no V1 barge-in, AEC, or acoustic wake-phrase trimming.

## WW-200 — generic `PcmRingBuffer`

- Implemented in `src-tauri/src/audio/pcm_ring_buffer.rs` and exported from `src-tauri/src/audio/mod.rs`.
- The implementation is sherpa-independent and uses fixed preallocated `i16` storage.
- Nominal Wake Word V1 capacity is 2 seconds of 16 kHz mono PCM (32,000 samples).
- Writes remain bounded, including oversized writes; snapshots are chronological after wraparound; clear/reset is supported and zeroes retained storage.
- Ownership is deliberately external: the type does not add internal synchronization or engine/runtime coupling.
- Unit coverage includes empty, partial, exact-capacity, single/repeated wraparound, oversized writes, clear, exact chronological order, and long repeated bounded writes.
- No diagnostic/logging path exposes sample contents.
- Exact corrected PR head: `61dc7b59938d944dd3d8c6f0a9c7791a97b8f271`.
- Exact-head ordinary CI run `34956745123` passed.
- Merged through PR #126 using the guarded allowed merge path.
- Exact resulting master SHA: `26d0087859567a9e0a32bd95cabab80ac1fe7c8c`.

## Next implementation item

WW-100/WW-110 remain open. Their completion requires immutable sherpa-onnx model/runtime/tokenizer identities, hashes, provenance/licenses, deterministic preparation/verification, and supported-platform packaging evidence. No WW-100/WW-110 checkbox should be treated as complete until those exact artifacts are committed and qualified.
