# WSP-200 verified-model lease/delete regression — blocked source write

**Observed:** 2026-10-21 UTC. **Repository:** `ekkus93/ai-talking-moose`. **Exact master before and after reconciliation:** `3a7dae2ed4033b434b8f639eccdf74354697b32c`.

## Proposed regression (not merged or qualified)

The existing `active_verified_model_lease_blocks_delete_until_released` test in `src-tauri/src/asr/whisper/installer/tests.rs` creates a dummy file without a verified installation marker. A proposed change would instead install the fixture through `WhisperModelInstaller`, confirm the model and marker are verified, hold the active model lease, assert deletion cannot remove the bytes or marker while the lease is held, then release the lease and assert both disappear without an additional download. This would strengthen WSP-200's verified install/delete interlock evidence.

**This proposed test was NOT committed.** No source/test/CI acceptance is claimed.

## Secret-free incident

- UTC error: `2026-10-21T10:01:31.590Z`.
- Ralph Bridge runtime: version `0.6.2`, build `0acea4b23362e2ccecab8f6c9db4418b162426cf`, instance `c75f6fc5b7794249adb6bf78a7d3605b`, auth mode `app`.
- Method: `commit_files`. Sanitized argument shape: `repo=ekkus93/ai-talking-moose`, `branch=master`, `expected_head_sha=3a7dae2ed4033b434b8f639eccdf74354697b32c`, one text upsert at `src-tauri/src/asr/whisper/installer/tests.rs`. No source body, credentials, tokens, models or transcripts included here.
- Exact tool response: `This tool call was blocked by OpenAI's safety checks. Please double check what you are sending.`
- **Classification:** platform-level OpenAI safety denial, not Ralph Bridge `INVALID_ARGUMENT`, GitHub authentication, API rate limiting or CI failure. No operation ID or accepted receipt.
- Reconciliation: a fresh Ralph Bridge `get_repo_state(master)` returned the same expected head `3a7dae2...`; `get_diff` from the expected head to master showed no changed files; exact-ref CI discovery still showed only successful documentation-only run `39579085381`. No evidence of the proposed source write landing.
- **Retry policy:** the same blocked payload was not resent, transformed or routed through another write method. Requires platform review or an independently authorized implementation; do not treat this as a fixable ordinary GitHub error.

## Earlier independent Mac CI closure (not current-head acceptance)

At prior exact source `4f54c340d5733246540af58f20112c7c6fe4a4f4`, native macOS [run 39222715385](https://github.com/ekkus93/ai-talking-moose/actions/runs/39222715385) is now **completed/success** with both ARM64 and Intel pinned Whisper static-link jobs successful. The unsigned Whisper-only app [run 39222715398](https://github.com/ekkus93/ai-talking-moose/actions/runs/39222715398) also completed **success** on both architectures. These were previously recorded as pending. They are native/app-smoke evidence at the earlier SHA, not current-head real-model Seatbelt-denied inference, physical mic/TTS, or permission to remove Moonshine. KittenTTS ONNX Runtime remains independently required.

The existing macOS real-weight manual dispatch is separately unresolved as documented in `docs/evidence/CROSS_PLATFORM_WHISPER_MANUAL_TRIGGER_POLICY_2026-10-21.md`; do not conflate the generic `INVALID_ARGUMENT` dispatch failure with this explicit platform safety denial.
