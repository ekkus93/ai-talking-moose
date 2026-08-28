# P13 macOS release reconciliation — 2026-08-23

**Status:** deterministic source/workflow implementation accepted; signed/notarized release execution remains an execution gate, while supported-Mac physical acceptance is explicitly deferred until suitable Mac hardware is available.

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

Still open: an actual Developer ID-signed/notarized tagged run using the owner's Apple credentials.

## V1R-132 — distribution artifacts

Implemented source/workflow behavior:

- architecture-specific `.app.zip` and `.dmg` outputs for arm64 and x86_64;
- combined `SHA256SUMS.txt` plus per-architecture checksum manifests;
- checked-in `docs/releases/v0.1.0.md` release notes;
- project `LICENSE` and native Moonshine/ONNX notices are staged into the application notice resource tree;
- production npm and macOS-reachable Rust dependencies are inventoried with copied package notice/license text when present; when a published package has no standalone notice file but declares a license in package metadata, that declaration is bundled explicitly and flagged for final release review;
- dependencies with neither packaged notice/license text nor a declared license remain a fail-closed error;
- the tagged workflow creates a **draft** GitHub Release only after both architectures verify; unsigned ordinary-CI smoke bundles are never published as release artifacts.

Still open: real tagged release artifacts and final legal/notice review of the generated payload before public publication.

## V1R-133 — upgrade compatibility

Existing deterministic implementation/tests already cover the source-level requirements:

- explicit SQLite schema versioning, ordered migrations, representative legacy-schema preservation, idempotent reopen, and failed-migration rollback;
- plaintext `google_api_key` migration writes/verifies the secure backend before deleting SQLite and preserves the legacy row if secure migration fails;
- settings deserialization distinguishes new profiles from legacy profiles: new profiles default to Moonshine Tiny, while profiles created before an ASR selector stay on Gemini Live audio because that preserves their prior cloud-microphone behavior;
- current profiles preserve an explicit Tiny/Small/Gemini selection;
- legacy settings migration is pure settings normalization and does not invoke the Moonshine model installer, so upgrade does not silently download a local model.

Physical upgrade acceptance on a representative pre-ASR-selector macOS profile remains deferred with the other supported-Mac-only evidence.

## V1R-134 — release smoke

`scripts/run_p13_macos_release_acceptance.sh` records the exact commit/version, macOS/hardware identity, and explicit PASS/FAIL/SKIP evidence for:

- final metadata/signing/distribution;
- legacy upgrade behavior;
- first-run/onboarding/Keychain/microphone permission;
- Tiny/Small local ASR and explicit Gemini Live routing;
- audio/conversation/barge-in/mute/dismiss;
- privacy, ambient, memory/transcript/data reset behavior;
- degraded provider/audio/model/install paths;
- clean-machine packaged runtime and no-development-secret checks.

No V1R-134 physical row is marked complete by source/CI work.

## 2026-08-24 current-master source-only release audit

The release-engineering audit is performed against current `master` after P12 and the P2/P3 physical-evidence provenance hardening. Because supported Mac hardware is not currently available, this pass deliberately closes only deterministic source/workflow requirements and records the physical remainder as deferred rather than PASS.

The audit found and repaired the following release-boundary gaps:

- [x] **No signing before source quality:** the tagged release workflow now has an explicit `source-gate` job. Both signed macOS builds depend on it, so frontend typecheck/lint/format/tests/build, Rust fmt/Clippy/tests, release metadata, Moonshine provenance, production dependency audits, and dependency-license collection must pass before Developer ID signing begins.
- [x] **Single signing-keychain owner:** the workflow no longer manually imports the same `.p12` that is also supplied through `APPLE_CERTIFICATE`. Tauri owns the documented ephemeral signing-keychain flow directly from `APPLE_CERTIFICATE` / `APPLE_CERTIFICATE_PASSWORD`; notarization remains authenticated with `APPLE_ID` / `APPLE_PASSWORD` / `APPLE_TEAM_ID`.
- [x] **Signed artifact provenance:** release builds explicitly set `TALKING_MOOSE_BUILD_COMMIT=$GITHUB_SHA`, and `verify_macos_release.sh` can require the packaged executable's embedded commit to equal that exact tag commit before accepting the signature/notarization checks.
- [x] **Release artifact set integrity:** the draft publisher requires exactly two `.app.zip` files, two DMGs, and two architecture checksum manifests, verifies both architecture manifests, then builds and self-verifies the combined `SHA256SUMS.txt` before creating the draft release.
- [x] **License payload fails before tagging:** ordinary CI executes `collect_release_licenses.py` on normal `master` CI rather than for the first time during a release tag. The collector filters Cargo's resolve graph for both shipped macOS targets, excludes dev-only dependency edges, copies explicit Cargo `license-file` paths and conventional notice files, and records declaration-only license metadata separately. A dependency with no usable notice/license text and no declared license still fails closed.
- [x] **Generated notices do not dirty source:** the generated dependency-notice staging tree is ignored alongside the other generated native release resources.
- [x] **Physical report provenance hardened:** the P13 physical runner requires the exact release tag and embedded build commit, verifies the downloaded DMG against the combined checksum manifest, requires nonblank evidence for every manual result, and confirms the tested executable hash did not change during the run.

The first CI execution of the license payload gate exposed that the original collector incorrectly treated every package in Cargo metadata as shipped and required a standalone notice file even when package metadata declared an SPDX license. That failure was a collector-model bug, not evidence that the listed crates were unlicensed. The corrected collector uses Cargo's target-filtered resolve graph and preserves declaration-only entries for mandatory final release review rather than silently treating them as copied license text.

Current Tauri 2 documentation still supports `APPLE_CERTIFICATE`, `APPLE_CERTIFICATE_PASSWORD`, `APPLE_ID`, `APPLE_PASSWORD`, `APPLE_TEAM_ID`, and optional `APPLE_SIGNING_IDENTITY`; no retired `altool` flow or custom sandbox entitlement is introduced by this audit.

## Deferred Mac-only acceptance ledger

The following remain intentionally **DEFERRED / NOT VERIFIED** until suitable supported Mac hardware is available:

- P1 real macOS Keychain persistence across an actual process restart and SQLite absence check;
- P2 V1R-020 through V1R-027 real production audio/device/TCC/Diagnostics acceptance;
- P3 V1R-037 real audible barge-in-stop latency and stale-audio confirmation;
- any remaining P3A native Tiny/Small physical/benchmark evidence that specifically requires supported Mac hardware;
- P6 intentional final-voice listening/cancellation physical acceptance;
- P13 V1R-133 packaged legacy-profile upgrade acceptance;
- P13 V1R-134 clean-machine packaged release smoke.

These rows are not failures, but they are also not converted to PASS by CI. They remain release-risk evidence to collect when hardware becomes available.

## Gate decision

P13 has two independent states:

1. **Source/release engineering:** accepted complete. The license-collector correction passed CI run `32754273420`, and later full repository runs, including `33199001645` and `33200290753`, continued to pass release metadata/license collection and both macOS bundle architectures on the descendant source tree.
2. **Release execution / physical acceptance:** still open. A real semantic tag must successfully produce Developer ID-signed/notarized arm64 and x86_64 artifacts, the generated notice payload still requires final release review, and supported-Mac physical evidence remains deferred.

The draft GitHub Release must remain unpublished until the project owner explicitly decides how to handle the deferred physical evidence and the signed/notarized tag execution has succeeded.
