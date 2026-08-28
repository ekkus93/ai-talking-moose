# Remaining Work Reconciliation — Non-owned-Mac Path — 2026-08-28

## Purpose

This reconciliation answers one narrow question after the P14–P20 remediation closure and the final P1 source repairs:

> What mandatory Talking Moose V1 work remains that can be advanced without access to a supported physical Mac owned by the project developer?

This document does not convert manual/physical acceptance into automated evidence. It classifies the remaining work so stale unchecked rows in `TODO(20260818-163801).md` are not mistaken for current source defects.

## Audit basis

The sweep reviewed the legacy feature tracker, every `RECONCILIATION_*.md` overlay, the completed P14–P20 remediation tracker, current standalone-speech cancellation code/tests, the P6 physical voice protocol, the P13 release workflow, and the latest accepted full CI evidence.

Relevant current evidence includes:

- PR #9 / V1R-013 provider-error hardening: head `3cc5ad4d7450e373b1d5c7483a889946a5e17f5b`, CI `33199001645`, merged as `6a631f0d05dc42ada94569c7a09dd9fccb541fb2`;
- PR #10 / V1R-014 diagnostic-capture teardown: head `0d523a6bebaf504a58126f689ce5fde4e03e3239`, CI `33200290753`, merged as `2194a5ee072f9fc835c3365388bc16bfa2e41437`;
- P13 dependency-license collector correction: `bb5e9f436b5430583e964f3ba124cefbd8b1e40b`, CI `32754273420`;
- P14–P20 remediation: `TODO(20260824-142233).md` is closed; its final reconciliation is `RECONCILIATION_FINAL_20260828.md`.

## Source/test backlog

**No mandatory source-code defect remains in the reconciled V1 feature/remediation scope.**

The old monolithic TODO still contains numerous unchecked source/test rows, but later reconciliation overlays and remediation commits supersede those stale entries. In particular:

- P0 source/CI quality is complete.
- P1 source-level security/privacy repair is complete; only real Keychain/process-restart evidence remains.
- P2/P3 source/test gaps are closed; remaining rows are supported-Mac physical audio/TCC/latency evidence.
- P3A is closed by real native Tiny/Small acceptance and benchmark evidence.
- P5 and P7–P12 source/test implementation is complete; older "pending CI" wording in several overlays is superseded by later full-repository CI on the descendant source tree.
- P13 deterministic release engineering is complete; release execution and physical smoke remain separate gates.
- P14–P20 remediation is complete.

### V1R-065 cancellation reconciliation

The stale P6 overlay previously listed "explicit cancellation acceptance" as a source/test gap. Current production behavior closes that gap:

- `invoke_standalone_speech` uses `synthesize_and_queue_cancellable`;
- `StandaloneSpeechController::cancel` cancels the current token and flushes playback;
- Start, Barge-in, Mute, Dismiss, and the explicit standalone-cancel command route through the same controller;
- `explicit_cancellation_aborts_in_flight_synthesis_and_flushes_playback` exercises cancellation while production cancellable synthesis is suspended and proves no queued/playing audio remains;
- `ambient_barge_in_flushes_standalone_audio_and_returns_to_idle` separately exercises the command/IPC interruption boundary with buffered playback.

Those tests are part of the Rust suite that passed later full CI, including run `33200290753`.

This does **not** close the separate P6 supported-Mac physical protocol in `MACOS_P6_VOICE_ACCEPTANCE.md`. That protocol intentionally requires human listening for voice selection and real audible cancellation behavior.

## Mandatory work that does not require owning a physical Mac

### 1. P13 signed/notarized tagged release execution

This is the main mandatory gate that can be advanced without a developer-owned Mac.

`.github/workflows/release.yml` builds on GitHub-hosted `macos-15` arm64 and `macos-15-intel` runners. A semantic `v*.*.*` tag triggers:

- the source quality gate;
- Developer ID signing;
- Apple notarization and stapling;
- signed `.app.zip` and `.dmg` generation for both architectures;
- signature/notarization/provenance verification;
- checksums; and
- creation of a **draft** GitHub Release.

This requires the repository's Apple release secrets to be configured: `APPLE_CERTIFICATE`, `APPLE_CERTIFICATE_PASSWORD`, `APPLE_ID`, `APPLE_PASSWORD`, and `APPLE_TEAM_ID`.

The workflow must not be triggered merely as a probe if those credentials are unavailable. A successful tagged run closes the release-execution portion of V1R-131/V1R-132, but it does not replace clean-machine physical smoke.

### 2. Final dependency-notice/legal review of the generated release payload

The collector now fails closed for dependencies without usable licensing evidence, and CI validates the generated inventory. Some dependencies legitimately contribute declaration-only license metadata rather than a copied standalone license file. Before a public release, the generated notice payload still requires final human review.

That review does not require a physical Mac; it can be performed from the tagged workflow artifacts/draft release once the signed run exists.

## V1R-ASR-004 Miri hardening reconciliation

The initial residual-work sweep incorrectly said no repository evidence existed for Miri. That was stale: commit `83a3b0fa42ca5313a289b3334a7632aebeb22c19` added the dependency-free production-module Miri harness, and dedicated `ASR Rust Safety` workflow run `32544957896` passed all 15 tests under Miri. The monolithic TODO's checked V1R-ASR-004 Miri row was therefore correct.

The 2026-08-28 hardening pass goes further instead of merely correcting that documentation error. The production Moonshine FFI transcript structs/layout assertions and `NativeMoonshineApi::copy_transcript` helper are made available to the unit-test/Miri configuration without linking the native library. New synthetic C-layout tests directly exercise the audited unsafe pointer-copy boundary, including null-pointer fail-closed behavior and copying C-string/metadata into Rust-owned data. The dedicated workflow is also pinned to the known-good `nightly-2026-08-22` toolchain used by the earlier accepted run.

Native Moonshine/ONNX dylib calls remain outside Miri's practical scope and continue to be covered by ASR-015's real supported-macOS Tiny/Small native acceptance. Once the strengthened ASR Rust Safety workflow passes on this change, there is no remaining V1R-ASR-004 sanitizer/Miri-compatible hardening item.

## Deferred until supported physical Mac access

The following remain deliberately unverified:

- **P1:** real Keychain persistence across a process restart, real SQLite absence check, and related secure-store acceptance;
- **P2:** physical output/input device behavior, real sample-rate/format behavior, TCC permission flows, device disconnects, diagnostics UI, and related audio acceptance;
- **P3:** real audible barge-in-to-playback-stop latency and stale-audio confirmation;
- **P6:** intentional listening comparison across the complete supported Google voice catalog plus the documented real audible cancellation checks;
- **P13/V1R-133:** packaged legacy-profile upgrade acceptance;
- **P13/V1R-134:** clean-machine packaged release smoke and the broader physical acceptance matrix.

These are not source failures. They remain release-risk evidence that must be collected later if V1 is to be declared fully physically accepted.

## Current priority

With physical-Mac work deferred, there is no mandatory implementation task to code next inside the reconciled V1 scope.

The next actionable V1 gate is therefore **P13 signed/notarized tagged release execution**, but only if the Apple release credentials are already available and the project owner intends to create the `v0.1.0` release candidate. The workflow creates a draft release; it does not publish the release publicly.

Because Apple release credentials are currently deferred, acceptance of the strengthened V1R-ASR-004 Miri workflow leaves no remaining mandatory or optional V1 implementation task. Further V1 progress then waits on the deferred physical-Mac and release-execution prerequisites.
