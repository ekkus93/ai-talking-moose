# macOS P2/P3 physical acceptance

**Status:** required supported-Mac release evidence; not satisfied by sandbox or ordinary CI

This protocol closes the deliberately physical remainder of P2 (production audio) and P3 (conversation lifecycle). It does **not** replace the separate P1 Keychain restart acceptance, P3A Tiny/Small native-model + ASR-015 benchmark acceptance, or P6 voice-audition acceptance.

Run the acceptance against the exact candidate commit that is intended to proceed to P13. Use a real supported Mac, the packaged application where the check concerns bundle/runtime behavior, and real audio hardware. Do not mark a row complete from mock audio, Tauri test runtime, Linux behavior, or a GitHub-hosted bundle smoke job.

The application binary now embeds its full Git build commit. The acceptance runner fails closed unless the packaged `.app` reports the same commit as the clean source checkout, so a stale or accidentally substituted bundle cannot be certified under the current source SHA. The runner also records the bundle identifier/version and executable SHA-256, then verifies that executable hash again after the physical run.

## Build and evidence runner

Build the exact clean checkout you intend to test. When building locally, pin the embedded provenance explicitly:

```bash
export P2_P3_EXPECTED_COMMIT="$(git rev-parse HEAD)"
export TALKING_MOOSE_BUILD_COMMIT="$P2_P3_EXPECTED_COMMIT"

# Prepare the pinned native runtime if it is not already staged for this Mac.
bash scripts/prepare_moonshine_macos.sh
python3 scripts/generate_app_icons.py
npm run tauri -- build --bundles app
```

The default bundle location is `src-tauri/target/release/bundle/macos/Talking Moose AI.app`. If the candidate bundle is elsewhere, set `TALKING_MOOSE_APP_PATH` to that `.app`. Then run:

```bash
P2_P3_EXPECTED_COMMIT="$(git rev-parse HEAD)" \
  TALKING_MOOSE_APP_PATH="src-tauri/target/release/bundle/macos/Talking Moose AI.app" \
  bash scripts/run_p2_p3_macos_physical_acceptance.sh
```

The runner rejects tracked or untracked changes to source-controlled inputs while allowing only the repository’s known dependency/generated/build trees (`node_modules`, `dist`, Tauri target output, generated icons, and staged macOS native runtime). It validates the `.app` bundle identifier/version/build commit, records the executable hash, captures the macOS audio-device inventory, and launches that exact validated bundle with `open -n` before walking the operator through each manual check. A run is accepted only when every required check is explicitly recorded `PASS`, every result has nonblank evidence notes, and the executable remains byte-identical through the run. `SKIP`, blank evidence, provenance mismatch, bundle mutation, or any failure keeps the gate open.

For the clean microphone-permission sequence, resetting the application permission is destructive to the current TCC decision. The runner only prints the command; it never runs `tccutil reset` automatically.

## Required P2 evidence

### V1R-020 — Rust is the sole production speech owner

- Trigger normal conversational output plus at least one standalone/canned or ambient speech path.
- Confirm there is exactly one audible utterance for each response: no browser `speechSynthesis` duplicate, echo, or overlapping second speech engine.
- Stop/dismiss the Moose and confirm queued speech does not continue from a second owner.

### V1R-021 — actual output-device rate

Exercise real output devices/configurations that negotiate **44.1 kHz** and **48 kHz**. For each:

- confirm Diagnostics reports the actual selected device and negotiated rate;
- play the generated output test tone and normal speech;
- confirm pitch/speed are correct and audio is not truncated or stretched.

If one rate cannot be exercised with the available hardware, the row remains open rather than being inferred from unit tests.

### V1R-022 — practical CPAL output format

For each real output configuration exercised above, record the sample format and channel count shown by Diagnostics and verify successful generated-tone and speech playback. A runtime format that the application advertises as supported must not fail or silently substitute a different selected device.

### V1R-023 — bounded playback and interruption

- Produce enough speech to create visible playback queue depth.
- Under repeated/long output, verify queue depth never exceeds the reported hard limit and the UI remains responsive.
- While speech is playing, perform barge-in.
- Confirm audible playback stops promptly, the mouth/output level returns to idle, queue depth is cleared, and **no pre-interruption audio resumes later**.
- Record any dropped-playback counter increase observed during the overload exercise.

The hard bounded/drop policy and stale-callback suppression are covered by deterministic tests; this step verifies the real CoreAudio path does not violate those invariants.

### V1R-024 — truthful microphone startup/failure

Exercise all practical real-Mac failure paths:

- denied microphone permission;
- stale explicitly selected microphone (select a USB device, disconnect it, then start);
- device disconnect during active capture where the hardware permits it.

Confirm each path reports a truthful error, does not silently change devices, and leaves microphone activity/capture diagnostics inactive after failure.

A Mac with a permanent built-in microphone may not be able to demonstrate a literal host-wide "no input device" state. The stale-selected-device/disconnect checks are the required physical analogue; the no-default-device branch remains deterministic-test covered.

### V1R-025 — built-in and USB microphone

Using both a built-in microphone and a real USB microphone:

- select the intended input explicitly;
- run the local microphone diagnostic and a normal conversation;
- confirm actual device, sample rate, sample format, channel count, and live input level are truthful;
- confirm speech is intelligible and channel conversion does not produce obvious missing-channel, phase-cancellation, or speed/pitch faults;
- disconnect the USB microphone after the successful case and verify the configured device is not silently replaced.

### V1R-026 — clean-profile permission flow

Perform this sequence on a clean/reset TCC state for bundle identifier `com.talkingmoose.ai`:

1. Before requesting access, confirm the app reports `not_requested` and normal conversation start does not trigger an accidental permission prompt.
2. Use the explicit Settings permission action and confirm macOS presents the microphone request.
3. Deny access; confirm the app reports denied/restricted state and does not repeatedly re-prompt.
4. Open macOS System Settings, grant microphone access, return to the app, and use Refresh.
5. Confirm state becomes granted and a microphone diagnostic/conversation can capture audio.

### V1R-027 — diagnostics UI/device acceptance

On the real Mac, verify the Diagnostics UI exposes and correctly updates:

- configured and actual input/output device;
- negotiated input/output rate, format, and channels;
- microphone active state and input level;
- output level;
- playback queue depth/limit;
- dropped input chunks and playback samples;
- last typed input/output error;
- local microphone test;
- generated output-tone test;
- refusal to steal audio ownership while a conversation is active.

The user must be able to diagnose a missing/wrong device from this UI without reading developer logs.

## Required P3 evidence

### V1R-037 — real barge-in latency and stale-audio closure

While the Moose is audibly speaking through a real output device:

1. record the interaction with a method that lets the operator estimate the interval between the barge-in action and audible playback stop (screen/audio recording is acceptable);
2. perform barge-in several times, including once with a visibly non-empty playback queue;
3. record the observed stop latency in milliseconds for the representative run;
4. confirm the stop is perceptibly quick;
5. continue the conversation long enough to prove stale pre-interruption speech never resumes.

The current specification deliberately defines no hard millisecond SLA before real measurements exist. This acceptance therefore records the measured value rather than inventing a threshold.

## Gate decision

P2/P3 physical acceptance is complete only when the generated report contains `PASS` for every required check above and identifies the exact tested commit, macOS version, Mac model/CPU, and real audio devices used.

Physical evidence does not by itself close the separate P1, P3A, P6, or P13 gates.
