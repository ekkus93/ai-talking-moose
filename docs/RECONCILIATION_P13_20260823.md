# P13 macOS release reconciliation — 2026-08-23

**Status:** source/workflow implementation staged; signed/notarized artifacts and physical release acceptance remain open until a real release tag and supported-Mac evidence exist.

## V1R-130 — macOS metadata

Source state after this reconciliation:

- product name is `Talking Moose AI`;
- bundle identifier is `com.talkingmoose.ai`, matching the existing Keychain/TCC identity already used by prior physical protocols;
- V1 version is consistently `0.1.0` across npm, Cargo, and Tauri metadata;
- placeholder icon blobs are removed; release builds deterministically generate original project Moose artwork in PNG, ICNS, and ICO formats from `scripts/generate_app_icons.py`;
- V1 now declares macOS 13.4 as the minimum on both Intel and Apple Silicon, matching the Mach-O deployment target of the pinned ONNX Runtime 1.23.2 dylibs; CI and release verification enforce that same floor on every shipped Mach-O;
- direct Developer ID distribution has been reviewed as non-sandboxed and requiring no custom V1 entitlements; microphone access remains a TCC/usage-description capability rather than an App Sandbox entitlement.

`python3 scripts/validate_release_metadata.py` is the fail-closed metadata/version/icon gate.

## V1R-131 — signing/notarization

Implemented source/workflow behavior:

- `.github/workflows/release.yml` runs only for semantic `v*.*.*` tags and requires Developer ID certificate + Apple notarization credentials;
- Tauri performs Developer ID signing, notarization, and stapling during `app,dmg` release builds;
- `scripts/verify_macos_release.sh` requires Developer ID authority, hardened runtime, secure timestamp, Gatekeeper acceptance, and valid stapled tickets for app and DMG;
- `docs/MACOS_RELEASE.md` documents secrets and independent verification commands;
- no credential is stored in source.

Still open: an actual Developer ID-signed/notarized run using the owner's Apple credentials.

## V1R-132 — distribution artifacts

Implemented source/workflow behavior:

- architecture-specific `.app.zip` and `.dmg` outputs for arm64 and x86_64;
- combined `SHA256SUMS.txt` plus per-architecture checksum manifests;
- checked-in `docs/releases/v0.1.0.md` release notes;
- project `LICENSE`, native Moonshine/ONNX notices, and resolved production npm/Rust dependency license texts are staged into the application notice resource tree;
- the tagged workflow creates a **draft** GitHub Release only after both architectures verify; unsigned ordinary-CI smoke bundles are never published as release artifacts.

Still open: real tagged release artifacts and final legal/notice review of the generated payload before public publication.

## V1R-133 — upgrade compatibility

Existing deterministic implementation/tests already cover the source-level requirements:

- explicit SQLite schema versioning, ordered migrations, representative legacy-schema preservation, idempotent reopen, and failed-migration rollback;
- plaintext `google_api_key` migration writes/verifies the secure backend before deleting SQLite and preserves the legacy row if secure migration fails;
- settings deserialization distinguishes new profiles from legacy profiles: new profiles default to Moonshine Tiny, while profiles created before an ASR selector stay on Gemini Live audio because that preserves their prior cloud-microphone behavior;
- current profiles preserve an explicit Tiny/Small/Gemini selection;
- legacy settings migration is pure settings normalization and does not invoke the Moonshine model installer, so upgrade does not silently download a local model.

Still open: physical upgrade acceptance on a representative pre-ASR-selector macOS profile using the packaged release candidate, including Keychain persistence and confirmation that no unexpected model download/cloud-routing change occurs.

## V1R-134 — release smoke

`scripts/run_p13_macos_release_acceptance.sh` now records the exact commit/version, macOS/hardware identity, and explicit PASS/FAIL/SKIP evidence for:

- final metadata/signing/distribution;
- legacy upgrade behavior;
- first-run/onboarding/Keychain/microphone permission;
- Tiny/Small local ASR and explicit Gemini Live routing;
- audio/conversation/barge-in/mute/dismiss;
- privacy, ambient, memory/transcript/data reset behavior;
- degraded provider/audio/model/install paths;
- clean-machine packaged runtime and no-development-secret checks.

No V1R-134 physical row is marked complete by this source change.

## Gate decision

P13 remains **OPEN** until:

1. all prerequisite physical gates still required from P1/P2/P3/P3A/P6 are PASS on the release candidate;
2. a `v0.1.0` (or later version-consistent) tag produces both signed/notarized architectures successfully;
3. the generated draft release passes the complete P13 physical acceptance script on supported clean Mac hardware;
4. the generated license/notice payload receives final release review;
5. the draft GitHub Release is published only after those acceptance conditions are met.
