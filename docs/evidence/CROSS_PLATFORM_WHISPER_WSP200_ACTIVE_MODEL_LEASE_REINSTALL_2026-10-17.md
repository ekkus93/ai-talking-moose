# WSP-200 — Active verified-model lease and concurrent reinstall

**Recorded:** 2026-10-17 UTC
**Qualified source:** `b738a3ce598aa57f0784ce3e9fed0d2dcba32ab6`
**Exact-head CI:** [39046290326](https://github.com/ekkus93/ai-talking-moose/actions/runs/39046290326) — **success**.

## Focused change

The test `active_verified_model_lease_serializes_reinstall_without_replacing_weights` was added to `src-tauri/src/asr/whisper/installer/tests.rs`. It first installs a tiny synthetic model using the production `WhisperModelInstaller` and a deterministic fake download transport. It holds a `WhisperVerifiedModelLease` on the installed model while starting another install, checks that the operation has not completed during the lease, confirms the model bytes and marker remain intact, then releases the lease. The queued installer returns `AlreadyInstalled` without a second download; the original bytes and marker remain unchanged.

This extends the existing active-lease/delete and reinstall-idempotence regressions without changing production model code. No Whisper model weights are downloaded or committed.

## Qualification

The initial test-only source commit `fc584350a52403712f056ce554ccd40553b33a15` failed Rust quality on one rustfmt line-wrap. A narrow formatting correction landed at `b738a3ce598aa57f0784ce3e9fed0d2dcba32ab6`. Exact-head main CI run `39046290326` subsequently passed `Rust tests`, `Rust quality` (Clippy/rustfmt), `Generated backend contract`, and scope classification. Skipped frontend and full packaging jobs are not counted as passed.

## Acceptance boundaries

This exercises deterministic fixture bytes and lock ownership, not a physical microphone, genuine Whisper model loading during inference, full model update/cancellation matrix, enforced macOS egress-denial, signed/notarized packaging, real playback or human observation. WSP-200, WSP-400, WSP-500, WSP-600 and WSP-700 remain subject to their named independent gates. Preserve Moonshine until qualified cutover and preserve KittenTTS's separate ONNX Runtime.
