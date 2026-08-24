# macOS release process

**V1 product:** Talking Moose AI
**Bundle identifier:** `com.talkingmoose.ai`
**V1 version:** `0.1.0`
**Deployment target:** macOS 13.4 or later on both Intel and Apple Silicon.

This document describes direct Developer ID distribution outside the Mac App Store. The release gate is intentionally split into automated artifact integrity and physical clean-Mac acceptance. A tagged workflow creates a **draft** GitHub Release; it must not be published until the project owner has explicitly resolved the physical-acceptance gate. When supported Mac hardware is unavailable, those physical rows remain deferred rather than being inferred from CI.

The V1 floor is set by the pinned ONNX Runtime 1.23.2 dylibs shipped with Moonshine. Their Mach-O deployment target is macOS 13.4, so advertising an earlier application minimum would be incorrect even if the Rust/Tauri executable itself could be compiled for an older OS.

## Entitlement review

V1 is a direct-download, non-sandboxed Tauri application. It does not use App Sandbox, CloudKit, push notifications, kernel/system extensions, or other capabilities requiring a custom entitlement. Microphone access is controlled by TCC and `NSMicrophoneUsageDescription` in `src-tauri/Info.plist`.

Accordingly, `bundle.macOS.entitlements` remains `null`. Do not add `com.apple.security.app-sandbox` or permissive hardened-runtime exceptions such as `allow-jit`, `allow-unsigned-executable-memory`, or library-validation disablement merely to make notarization pass. If a future feature genuinely needs an entitlement, add the minimum entitlement and document why.

Developer ID signing must enable Apple's hardened runtime. The release verifier rejects a signature that does not report the `runtime` flag.

## Release credentials

Configure these GitHub Actions secrets before creating a release tag:

- `APPLE_CERTIFICATE` — base64-encoded Developer ID Application `.p12` certificate.
- `APPLE_CERTIFICATE_PASSWORD` — password used when exporting that certificate.
- `APPLE_ID` — Apple ID used for notarization.
- `APPLE_PASSWORD` — app-specific password for that Apple ID.
- `APPLE_TEAM_ID` — Apple Developer team ID.

The workflow deliberately lets the Tauri bundler own the ephemeral signing-keychain lifecycle from `APPLE_CERTIFICATE` / `APPLE_CERTIFICATE_PASSWORD`. Do not also import the same `.p12` manually in the workflow: duplicate identities can make signing ambiguous. `APPLE_SIGNING_IDENTITY` remains available as a Tauri override if a future certificate setup genuinely requires it, but V1 does not need to derive and inject it manually.

Never commit the certificate, app-specific password, private keys, or decoded credential files.

## Pre-tag gate

From a clean checkout:

```bash
python3 scripts/generate_app_icons.py
python3 scripts/validate_release_metadata.py
npm run check:all
python3 scripts/collect_release_licenses.py
```

Ordinary CI also executes dependency-license collection so a missing resolved notice file fails on `master`, before a release tag is created.

When real supported Mac hardware is available, the still-open physical gates from P1/P2/P3/P3A/P6 must be collected against the intended release candidate. If hardware is unavailable, record those rows as deferred; do not mark them PASS from CI.

## Creating a release candidate

The tag must exactly match the version in `package.json`, `src-tauri/Cargo.toml`, and `src-tauri/tauri.conf.json`. For V1:

```bash
git tag -a v0.1.0 -m "Talking Moose AI v0.1.0"
git push origin v0.1.0
```

`.github/workflows/release.yml` first runs a **tagged source release gate**. Signing does not begin until that gate passes frontend typecheck/lint/format/tests/build, Rust formatting/Clippy/tests, release metadata, Moonshine provenance, dependency audits, and dependency-license collection.

After the source gate, Apple Silicon and Intel artifacts build independently. For each architecture the workflow:

1. generates the deterministic icon set and validates tag/version/product/bundle metadata plus the generated icon containers;
2. prepares the pinned Moonshine + ONNX Runtime dylibs at the explicit macOS 13.4 support floor;
3. stages the project license, Moonshine/native notices, and license texts for resolved Rust/production npm dependencies;
4. sets `TALKING_MOOSE_BUILD_COMMIT` to the exact tag `GITHUB_SHA`;
5. builds the Tauri `.app` and DMG with Developer ID signing and Tauri notarization/stapling;
6. verifies nested native libraries, deployment target, embedded build commit, Developer ID authority, secure timestamp, hardened runtime, Gatekeeper assessment, stapled notarization ticket, and absence of obvious secret/database files;
7. packages the `.app` with `ditto`, retains the DMG, and computes architecture checksums.

The draft-publish job then requires exactly two `.app.zip` files, two DMGs, and two architecture checksum manifests, verifies both architecture manifests, generates and self-verifies the combined `SHA256SUMS.txt`, and only then creates the **draft** GitHub Release.

Ordinary `ci.yml` never publishes unsigned bundles. It only builds an unsigned `.app` smoke bundle on macOS. Distribution assets come exclusively from the signed/notarized release workflow.

## Automated verification commands

Given a built application and DMG on macOS, verify both signing/notarization and source provenance:

```bash
bash scripts/verify_macos_release.sh \
  "/path/to/Talking Moose AI.app" \
  "/path/to/Talking-Moose-AI_v0.1.0_macos_arm64.dmg" \
  arm64 \
  "$(git rev-parse v0.1.0^{commit})"
```

The verifier includes the equivalent of:

```bash
codesign --verify --deep --strict --verbose=2 "/path/to/Talking Moose AI.app"
codesign -dvvv "/path/to/Talking Moose AI.app"
spctl -a -t exec -vv "/path/to/Talking Moose AI.app"
xcrun stapler validate "/path/to/Talking Moose AI.app"
xcrun stapler validate "/path/to/Talking Moose AI.dmg"
spctl -a -t open --context context:primary-signature -v "/path/to/Talking Moose AI.dmg"
```

Apple's current notarization service uses `notarytool`; Tauri performs submission/stapling when the notarization environment variables above are present. Do not reintroduce the retired `altool` flow.

## Physical P13 acceptance

When a supported Mac becomes available, download the draft release's architecture artifact plus the combined `SHA256SUMS.txt`, check out the exact tag commit, and run:

```bash
P13_EXPECTED_COMMIT="$(git rev-parse v0.1.0^{commit})" \
P13_SHA256SUMS="/path/to/SHA256SUMS.txt" \
bash scripts/run_p13_macos_release_acceptance.sh \
  "/Applications/Talking Moose AI.app" \
  "/path/to/Talking-Moose-AI_v0.1.0_macos_arm64.dmg"
```

The runner requires the checkout, release tag, packaged executable's embedded build commit, and DMG checksum manifest to agree before manual evidence begins. Every PASS/FAIL/SKIP row requires nonblank evidence, and the runner re-hashes the application executable at the end so the tested binary cannot silently change during acceptance.

The generated report remains `OPEN` if any required row is `FAIL` or `SKIP`. It covers metadata, signing/notarization, distribution files, upgrade compatibility, clean first run, local/cloud ASR routing, audio/conversation behavior, privacy/data controls, degraded modes, and clean-machine packaged-native-runtime behavior.

## Publishing the draft

A draft release is not equivalent to release acceptance. Before public publication:

1. confirm the signed/notarized tagged workflow succeeded for both architectures;
2. review the generated dependency/native notice payload;
3. collect the supported-Mac physical evidence when hardware is available, or explicitly document the project's decision to ship with those rows deferred;
4. record the final tag commit and artifact hashes in the P13 reconciliation record.

Only after that explicit release decision should the draft be made public. With GitHub CLI authenticated:

```bash
gh release edit v0.1.0 --draft=false
```
