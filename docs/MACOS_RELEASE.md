# macOS release process

**V1 product:** Talking Moose AI  
**Bundle identifier:** `com.talkingmoose.ai`  
**V1 version:** `0.1.0`  
**Deployment target:** macOS 10.15 or later on Intel; Apple Silicon hardware necessarily starts with macOS 11 or later.

This document describes direct Developer ID distribution outside the Mac App Store. The release gate is intentionally split into automated artifact integrity and physical clean-Mac acceptance. A tagged workflow creates a **draft** GitHub Release; it must not be published until the physical acceptance report passes.

## Entitlement review

V1 is a direct-download, non-sandboxed Tauri application. It does not use App Sandbox, CloudKit, push notifications, kernel/system extensions, or other capabilities requiring a custom entitlement. Microphone access is controlled by TCC and `NSMicrophoneUsageDescription` in `src-tauri/Info.plist`.

Accordingly, `bundle.macOS.entitlements` remains `null`. Do not add `com.apple.security.app-sandbox` or permissive hardened-runtime exceptions such as `allow-jit`, `allow-unsigned-executable-memory`, or library-validation disablement merely to make notarization pass. If a future feature genuinely needs an entitlement, add the minimum entitlement and document why.

Developer ID signing must enable Apple's hardened runtime. The release verifier rejects a signature that does not report the `runtime` flag.

## Release credentials

Configure these GitHub Actions secrets before creating a release tag. The release workflow imports the supplied `.p12` into an ephemeral build keychain, derives the `Developer ID Application` identity with `security find-identity`, and passes that identity to Tauri as `APPLE_SIGNING_IDENTITY`:

- `APPLE_CERTIFICATE` — base64-encoded Developer ID Application `.p12` certificate.
- `APPLE_CERTIFICATE_PASSWORD` — password used when exporting that certificate.
- `APPLE_ID` — Apple ID used for notarization.
- `APPLE_PASSWORD` — app-specific password for that Apple ID.
- `APPLE_TEAM_ID` — Apple Developer team ID.

Never commit the certificate, app-specific password, private keys, or decoded credential files.

## Pre-tag gate

From a clean checkout:

```bash
python3 scripts/generate_app_icons.py
python3 scripts/validate_release_metadata.py
npm run check:all
```

On a real supported Mac, all still-open physical gates from P1/P2/P3/P3A/P6 must be complete before the final release is published. P13 does not waive those gates.

## Creating a release candidate

The tag must exactly match the version in `package.json`, `src-tauri/Cargo.toml`, and `src-tauri/tauri.conf.json`. For V1:

```bash
git tag -a v0.1.0 -m "Talking Moose AI v0.1.0"
git push origin v0.1.0
```

`.github/workflows/release.yml` then builds Apple Silicon and Intel artifacts independently. For each architecture it:

1. generates the deterministic icon set and validates tag/version/product/bundle metadata plus the generated icon containers;
2. prepares the pinned Moonshine + ONNX Runtime dylibs at the explicit support floor (macOS 10.15 Intel, macOS 11.0 Apple Silicon);
3. stages the project license, Moonshine/native notices, and license texts for resolved Rust/production npm dependencies;
4. builds the Tauri `.app` and DMG with Developer ID signing and Tauri notarization/stapling;
5. verifies nested native libraries, deployment target, Developer ID authority, secure timestamp, hardened runtime, Gatekeeper assessment, stapled notarization ticket, and absence of obvious secret/database files;
6. packages the `.app` with `ditto`, retains the DMG, and computes architecture checksums;
7. combines both architectures into one **draft** GitHub Release with `SHA256SUMS.txt` and the checked-in release notes.

Ordinary `ci.yml` never publishes unsigned bundles. It only builds an unsigned `.app` smoke bundle on macOS. Distribution assets come exclusively from the signed/notarized release workflow.

## Automated verification commands

Given a built application and DMG on macOS:

```bash
bash scripts/verify_macos_release.sh \
  "/path/to/Talking Moose AI.app" \
  "/path/to/Talking-Moose-AI_v0.1.0_macos_arm64.dmg" \
  arm64
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

Download the draft release artifacts onto a supported Mac and run:

```bash
bash scripts/run_p13_macos_release_acceptance.sh \
  "/Applications/Talking Moose AI.app" \
  "/path/to/Talking-Moose-AI_v0.1.0_macos_arm64.dmg"
```

The generated report remains `OPEN` if any required row is `FAIL` or `SKIP`. It covers metadata, signing/notarization, distribution files, upgrade compatibility, clean first run, local/cloud ASR routing, audio/conversation behavior, privacy/data controls, degraded modes, and clean-machine packaged-native-runtime behavior.

## Publishing the draft

Only after the P13 report and all prerequisite physical gates are PASS should the draft be made public. With GitHub CLI authenticated:

```bash
gh release edit v0.1.0 --draft=false
```

Record the final release URL, exact tag commit, artifact hashes, Mac models/OS versions used for acceptance, and the acceptance report location in the P13 reconciliation document.
