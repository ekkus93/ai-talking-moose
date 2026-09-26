# WPCR-400 Artifact Provisioning Model

**Date:** 2026-09-25
**Remediation:** `docs/WAKE_WORD_V1_POST_CLOSEOUT_REMEDIATION_TODO_2026-09-25.md`
**Selected model:** Developer-prepared Wake artifacts

## Decision

Wake Word V1 uses the **developer-prepared** artifact provisioning model for this post-closeout remediation. The app must not imply that Wake Word works from an empty clean install. A clean app-data directory without the pinned Wake model/runtime files is expected to fail closed until the artifacts are explicitly prepared and verified.

## Source of truth

The selected provisioning model is recorded in `wake-word-artifacts.json`:

- `policy.provisioning_model = developer-prepared`
- `policy.clean_install_behavior = fail-closed-until-prepared`
- `policy.silent_network_download = false`

`src-tauri/src/app/wake_word_state.rs` resolves runtime files under app data and `NativeKwsSession` verifies the pinned model/runtime identities before listener startup. `scripts/prepare_wake_word_runtime.py` remains an explicit developer preparation helper; it is not an app startup download path. `scripts/freeze_wake_word_model_identity.py` records and re-verifies the exact consumed model identities.

## User-facing behavior

Settings now discloses that Wake model/runtime artifacts are developer-prepared and that a clean install fails closed until the pinned artifacts are prepared and verified. This prevents the Settings toggle from advertising unsupported clean-install readiness.

## Acceptance scope

This evidence closes the policy-selection and truthfulness slice for WPCR-400. It does not claim bundled/offline app-resource provisioning or an end-user installer flow. Those models remain intentionally not selected for Wake Word V1 in this remediation.
