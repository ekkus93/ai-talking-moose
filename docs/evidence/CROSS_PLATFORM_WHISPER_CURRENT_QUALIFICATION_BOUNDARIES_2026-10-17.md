# Cross-platform Whisper — current qualification boundaries (2026-10-17)

This record separates the latest **exact-head** automated negative-path test from earlier macOS native and diagnostic app-build evidence. It is not a Moonshine removal authorization or a physical acceptance report.

## Whisper Small × Google text × standalone TTS, missing credentials

- Source: `683ba568fc15a7158c413bc87a189a07ac141819`, commit `test(whisper): reject missing Google text key without TTS fallback`.
- Exact-source GitHub Actions CI: run `39047745343`, completed **success** (2026-10-17 16:10 UTC).
- Successful jobs: Rust tests `117439810568`; Rust quality `117439810515`; generated backend contract `117439810567`; scope classification `117439781421`.
- Test: `missing_google_text_key_in_whisper_non_live_talk_never_starts_tts` in `src-tauri/src/app/state/tests/talk_routing.rs`.
- With Whisper Small selected and no saved Google key, both Local and Google standalone TTS selections must produce a typed `Auth` error for the selected Google **text** provider. The test rejects disclosure of the synthetic input text and subsequent audio or successful completion events. It does not contact Gemini Live or prove real TTS playback.
- Frontend quality, macOS Tauri bundle, canonical `npm run check:all`, dependency audit and release gates were **skipped** in this exact-head CI scope; they are not passing evidence for this SHA.

## Prior diagnostic unsigned macOS app smoke and static link

- Source: `b738a3ce598aa57f0784ce3e9fed0d2dcba32ab6`, which predates the Google-key test above.
- `Whisper macOS Pre-Cutover App Smoke` run `39046290246`: completed **success** for arm64 job `117435630974` and x86_64 job `117435631273`. Both jobs passed diagnostic unsigned `.app` construction and the executable's pinned native Whisper source/linkage verification.
- `Whisper macOS Native Compile` run `39046290264`: completed **success** for arm64 job `117435638434` and Intel job `117435638620`.
- `ASR-015 Native Acceptance` run `39046290263` on that same source: **failure** in `Prepare pinned Moonshine native runtime` (job `117435562773`). This is a legacy Moonshine preparation failure, not evidence of a failed Whisper inference test.

## Remaining gates

Neither a diagnostic unsigned app build nor the missing-key unit test establishes real app launch, clean-machine model install, enforced macOS offline egress, microphone capture, spoken playback, signing/notarization, physical P13 or current-head complete release qualification. Existing two-architecture real-model CPU corpus evidence remains documented separately at `999a04e3aefaa9b93f30ad0c53b188009f9f228c`, run `38886320925`. Keep the WSP-200/300/400/500/600/700 open requirements open. Do not remove Moonshine until all applicable cutover gates pass. Preserve the independently owned KittenTTS ONNX Runtime, its models, and its offline/no-fallback guarantees.
