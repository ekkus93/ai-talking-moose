# ASR Unit Test Coverage TODO

**Date:** 2026-10-08
**Status:** Complete
**Scope:** New deterministic unit tests for the identified Whisper acceptance, Whisper installer, WAV parsing, and local-ASR pipeline-abort coverage gaps.

All listed work is complete. Verification results and practical test limits are recorded below. Keep tests independent of a real model download, network access, audio device, microphone permission, or host routing configuration. Preserve the existing manual real-CPU acceptance workflow for end-to-end evidence; it does not replace the failure-path unit tests in this list.

## ASRUT-100 — Whisper installer download and promotion failure coverage

**Primary files:** `src-tauri/src/asr/whisper/installer.rs`, `src-tauri/src/asr/whisper/installer/transport.rs`, and `src-tauri/src/asr/whisper/installer/tests.rs`.

- [x] Audit `WhisperModelInstaller::install` and enumerate each externally visible terminal outcome: installed, already installed, insufficient disk space, transport/HTTP failure, cancellation, size mismatch, SHA-256 mismatch, staging I/O failure, and promotion failure.
- [x] Introduce or refine a private test seam for the download transport and disk-space probe so tests can control streamed chunks, errors, and free space without network access.
- [x] Make integrity verification testable with a small fixture artifact while production continues to use the canonical manifest byte count and SHA-256. Do not weaken, replace, or override the production model identity to make fixtures pass.
- [x] Add a fake transport that records whether streaming was attempted and can emit deterministic chunks, return an error before the first chunk, or fail after partial output.
- [x] Add a fake disk-space probe and verify insufficient free space fails before transport I/O begins.
- [x] Cover a successful install using a small fixture and assert the final model and marker are promoted, metadata matches the fixture policy, and staging artifacts are removed.
- [x] Cover an already-installed verified model and assert no download is attempted and the disposition is `AlreadyInstalled`.
- [x] Cover transport failure before bytes arrive and after partial bytes arrive; assert the public error category is stable and all partial staging state is removed.
- [x] Cover cancellation before streaming, during chunk delivery, and after download completion but before verification/promotion. Assert cancellation does not produce a ready canonical install and staging state is cleaned.
- [x] Cover truncated and overlong fixture artifacts; assert both fail size verification without promotion. Separately prove an existing verified install is preserved and short-circuits a replacement transport.
- [x] Cover a same-size artifact with incorrect content; assert SHA-256 verification fails and no ready marker or canonical model is left behind.
- [x] Inject staging-file creation/write/flush failures where practical and assert errors remain sanitized and partial state is cleaned.
- [x] Inject model and marker promotion failures; assert stable promotion/I/O errors and cleanup. A verified install is preserved by the already-installed short circuit, so replacement promotion is not entered.
- [x] Assert operation-lock behavior for concurrent install attempts: duplicate work is serialized or rejected according to the existing contract, with no double promotion or staging collision.
- [x] Keep existing migration, marker compatibility, symlink, lease, delete, and retry tests intact; avoid duplicating their current assertions in new cases.

**Acceptance:** install success and every supported failure outcome are exercised without real model weights or network access. Failed/cancelled installs never appear verified or leave unowned partial artifacts, and an existing valid install is preserved without entering replacement transport or promotion.

## ASRUT-200 — WAV parser malformed-input matrix

**Primary file:** `src-tauri/src/asr/whisper/acceptance/wav.rs`.

- [x] Add a compact WAV fixture builder that emits a valid PCM16 RIFF/WAVE file and can insert, truncate, resize, or mutate individual chunks without hand-maintaining unrelated offsets.
- [x] Preserve the existing regression for an odd-sized `LIST` metadata chunk with its padding byte before `data`.
- [x] Add table-driven rejection cases for inputs shorter than a RIFF header, incorrect `RIFF`/`WAVE` identifiers, missing `fmt `, and a `fmt ` chunk that does not meet the parser's supported PCM layout.
- [x] Cover a truncated format chunk, a non-PCM format tag, unsupported bit depth, zero sample rate, and a block-alignment value inconsistent with channel count and sample width.
- [x] Cover a missing `data` chunk, a truncated chunk header, a metadata chunk whose declared extent passes the end of the file, and a `data` chunk whose declared extent passes the end of the file.
- [x] Cover a `data` chunk with an odd byte count for 16-bit samples.
- [x] Verify supported mono PCM16 boundary samples convert correctly, including minimum/maximum `i16` and zero.
- [x] Specify and test policy for RIFF declared size versus physical input length, duplicate `fmt `/`data` chunks, and trailing bytes. Reject inconsistent inputs or explicitly document and test the accepted behavior.
- [x] Verify parsing failures return stable, bounded error messages and never panic on arbitrary/truncated input.
- [x] Add a separate caller-level test for rejecting otherwise parseable non-mono corpora if that validation remains in `transcribe_for_acceptance` rather than the parser.

**Acceptance:** malformed and truncated WAV inputs fail with `Err` rather than panicking or producing samples; currently supported metadata-chunk WAVs and valid PCM16 samples continue to parse identically.

## ASRUT-300 — Acceptance report and network-denial behavior

**Primary files:** `src-tauri/src/asr/whisper/acceptance.rs` and `src-tauri/src/asr/whisper/acceptance/wav.rs`.

- [x] Separate pure report construction/serialization from live installer and native transcription work where needed, keeping test seams private to the acceptance module.
- [x] Add fixture-based tests for install report construction and JSON serialization, including schema version, model/revision/source identity, artifact filename, expected and installed bytes, disposition, license, and production-installer verification fields.
- [x] Add transcription report construction tests for transcript text/segment aggregation, duration and corpus metadata, runtime identity, pipeline metrics, and nullable resource metrics.
- [x] Cover empty/whitespace-only transcript output: assert the report's failure status and transcript fields are internally consistent and the caller receives the documented error.
- [x] Cover safe report-writing behavior for nested report directories, serialization/write errors where injectable, and stable error messages that do not expose private transcript/model data.
- [x] Extract a deterministic network-denial probe seam that accepts route and connection-probe results, without changing the production probe's fail-closed decision.
- [x] Test that a present default route rejects the denied-network condition without attempting the TCP probe.
- [x] Test that an absent route plus a failed connection probe passes the denial check.
- [x] Test that an absent route plus a successful connection probe fails the denial check.
- [x] Test that `require_network_denied = true` refuses transcription before model loading when the probe fails, and that `false` records probe status without imposing the denial requirement.
- [x] Ensure no unit test depends on `/proc/net/route`, a hard-coded external IP being reachable/unreachable, or the CI host's firewall configuration.
- [x] Keep the manual real-CPU workflow responsible for proving actual native transcription and OS-level network isolation; do not simulate those claims in unit tests.

**Acceptance:** report construction and network-denial policy have deterministic unit coverage; tests require neither a live Whisper model nor host-dependent network behavior, while the manual acceptance workflow remains the evidence for real native CPU transcription.

## ASRUT-400 — Pipeline abort drops queued work and suppresses stop-time finals

**Primary files:** `src-tauri/src/asr/pipeline/worker.rs`, `src-tauri/src/asr/pipeline.rs`, and `src-tauri/src/asr/pipeline_tests/`.

- [x] Define the expected abort contract for an inference call already in progress when abort is requested. Distinguish an update produced by that in-flight call from queued chunks and stop-time final updates.
- [x] Extend the fake engine with deterministic gates/counters for entering and releasing `push_pcm`, recording processed chunks, returning updates from `stop`, and recording that `stop` ran exactly once.
- [x] Add a regression that blocks the worker in one in-flight `push_pcm`, queues additional accepted chunks, requests abort, and then releases the worker without using timing sleeps as synchronization.
- [x] Assert queued chunks are not subsequently passed to `push_pcm` after the abort is observed.
- [x] Configure `stop` to return a final transcript and assert that abort suppresses this stop-time final from the callback and transcript state.
- [x] Assert engine cleanup still runs once, the worker joins, running state becomes false, and pending event acknowledgements do not deadlock shutdown.
- [x] Assert the documented policy for any update returned by the already-running `push_pcm` call; encode that policy explicitly so cancellation semantics cannot drift silently.
- [x] Add an abort-before-first-chunk case to cover the worker's top-of-loop abort path and verify queued audio is not processed.
- [x] Keep graceful `stop_and_join` coverage separate and unchanged: normal stop must continue draining accepted audio and delivering eligible final events.
- [x] Place the regression in the shutdown/cancellation-focused test module or a dedicated `abort.rs` module if it materially improves navigation.

**Acceptance:** deterministic abort tests prove queued work is not processed after abort, stop-time finals are not delivered, engine cleanup and worker joining still complete, and normal graceful-stop drain/final behavior remains covered independently.

## ASRUT-500 — Full test-gate verification and closeout

- [x] Run the focused Whisper installer tests and confirm no real model download occurs.
- [x] Run the focused WAV parser and acceptance report/network tests and confirm they do not require a model, audio device, or live network.
- [x] Run the focused pipeline abort tests repeatedly to check that the synchronization is deterministic and does not rely on sleeps.
- [x] Run `npm run check:rust` with the repository's documented all-targets/all-features gate.
- [x] Run `npm run check:all` when practical so frontend, static policy, IPC/generated-contract, build, and Rust gates are all checked together.
- [x] Confirm the manual Whisper real-CPU acceptance workflow remains manual and ordinary CI still downloads no Whisper weights.
- [x] Review every checkbox against a named test or gate before marking it complete; record any intentionally deferred case and its reason rather than implying it is covered.

**Acceptance:** all new unit tests are deterministic, the full Rust test/clippy/format gate passes, ordinary CI remains weight-free, and the checklist accurately records any remaining limitation.


## Closeout notes

- The installer failure suite injects staging-file creation failure. Write/flush failure injection was not added: the production sink owns a concrete `std::fs::File`, and portable deterministic write/flush failure injection would require a broader writer abstraction that is not needed for the bounded failure outcomes covered here.
- The install marker intentionally remains the canonical production manifest marker even when the private installer seam verifies a tiny fixture. Production construction still binds expected size and SHA-256 to the canonical manifest.
- The install flow does not replace a verified model: it returns `AlreadyInstalled` before transport. A dedicated regression verifies a failing replacement transport is never called and original model/marker bytes remain intact.
- No real Whisper weights, audio device, live network, or host routing state were used by these unit tests. The manual real-CPU acceptance workflow remains separate and unchanged; the workflow policy check passed.
