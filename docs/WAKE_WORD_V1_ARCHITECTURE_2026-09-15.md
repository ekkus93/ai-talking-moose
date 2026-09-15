# Wake Word V1 Architecture

**Status:** implementation architecture and user-facing behavior

Wake Word V1 adds an optional local keyword-spotting path for the canonical phrase **`Hey, Moose`**. The feature is deliberately narrower than speech recognition: keyword spotting decides only whether the configured wake phrase was detected. It does not continuously transcribe microphone audio.

## User-visible policy

- Wake Word is **disabled by default**. With it disabled, the existing manual-listen behavior remains authoritative.
- V1 supports one fixed phrase: **`Hey, Moose`**. Arbitrary custom wake phrases are intentionally out of scope until they have their own acoustic acceptance evidence.
- Keyword spotting is designed to run **locally/offline**. Idle wake inference must not require a cloud request or a continuously running full-ASR process.
- Enabling Wake Word means the microphone remains locally active while the application is eligible to listen for the keyword. This local microphone activity must be stated explicitly in Settings.
- After a wake trigger, downstream command ASR may receive **`Hey, Moose` plus the following prompt** as one continuous utterance. V1 does not acoustically trim the wake phrase.
- Wake activation is suspended for the complete interval in which Moose is speaking. **Barge-in is not supported in V1.** This prevents Moose's own TTS playback from intentionally acting as a new wake event.

## Runtime ownership

`WakeWordRuntimeManager` is the authoritative wake lifecycle owner. Its externally meaningful phases are disabled, loading, listening, triggered/command handoff, suspended while Talking, error, and shutdown. A single accepted wake event transitions out of listening before command activation, which provides the one-wake-event-to-one-command invariant and rejects repeated positive KWS frames from the same phrase.

The manager also owns the wake pre-roll lifecycle. Audio is accepted into pre-roll only while the runtime is actively listening. Disable, Talking suspension, interaction completion, runtime failure, and shutdown clear retained wake audio. Recoverable wake failures must not disable the application's manual interaction path.

The final microphone integration must preserve **one authoritative capture path**. Wake KWS and command ASR must not independently open competing microphone streams. Canonical microphone PCM should be routed chronologically to the bounded pre-roll buffer and KWS engine; after trigger, ownership transfers to command ASR and then returns to wake listening only after the interaction lifecycle permits it.

## PCM pre-roll and handoff

`PcmRingBuffer` is engine-independent and has a fixed capacity of two seconds of canonical 16 kHz mono PCM. Storage is preallocated and bounded. Long-running capture therefore cannot grow retained wake audio without bound.

When a wake event is accepted, `WakeWordRuntimeManager` captures one chronological snapshot of the ring buffer. That snapshot is consumable once, preventing duplicate replay. The command-ASR handoff must replay the snapshot in order and then continue with live samples without a gap, duplicate range, or sample-order inversion. This is specifically intended to preserve an immediate command such as `Hey Moose, tell me the time` while ASR initializes.

Raw PCM and ring-buffer contents are **memory-only runtime data**. They must never be serialized into settings, diagnostics, logs, crash messages, or acceptance reports.

## Wake configuration

The engine-independent V1 configuration defaults to disabled and canonicalizes the phrase to `Hey, Moose`. Case/outer whitespace variants of that phrase may normalize to the canonical representation; unrelated phrases fail validation. V1 uses fixed sherpa tuning (`threshold = 0.25`, `score = 1.0`) rather than exposing an unqualified sensitivity control.

Persisted settings integration must retain fail-closed default-on-missing behavior: installations without wake fields load with Wake Word disabled. Adding wake persistence must not reinterpret unrelated ASR, TTS, text-provider, privacy, or personality settings.

## Engine and artifact boundary

The V1 wake engine is **sherpa-onnx KWS** using pinned model, tokenizer/support, and native-runtime artifacts. Production inputs must have immutable identities and SHA-256 verification. Native libraries must also pass architecture verification before inference so a corrupt or wrong-architecture artifact fails closed.

Normal keyword inference is CPU-only and uses one inference thread unless measured acceptance evidence justifies a future policy change. KWS performs keyword detection only; it does not perform full transcription.

Artifact provenance, versions, hashes, architecture support, and third-party licensing are recorded by the repository's wake artifact manifests/preparation machinery. Cached artifacts are never trusted in place of hash verification.

## Privacy-safe diagnostics

Wake diagnostics may report configuration and lifecycle metadata needed for support, including enabled state, runtime phase, model/runtime identity, platform/architecture, inference thread count, canonical sample format, ring-buffer capacity/sample count, fixed threshold/score, trigger count, last-trigger age, initialization/performance timing, Talking suspension, and a sanitized last error.

Diagnostics must not contain:

- raw PCM or ring-buffer samples;
- utterance content or inferred transcript text from KWS;
- credentials or secrets;
- unnecessary raw filesystem paths.

Runtime errors are intentionally mapped to bounded, user-safe messages. A wake failure must not silently fall back to cloud transcription or full-time ASR.

## Conversation lifecycle

When Wake Word is enabled and the application is otherwise eligible, the intended lifecycle is:

1. initialize the pinned local KWS runtime and enter wake listening;
2. route one chronological microphone stream to KWS and the bounded pre-roll buffer;
3. accept one wake trigger and freeze one pre-roll snapshot;
4. hand pre-roll plus subsequent live PCM to the existing command-ASR path;
5. proceed through the normal Listening → Thinking → Talking interaction;
6. clear wake audio and keep wake activation suspended for all TTS playback;
7. after successful TTS completion, cancellation, or recoverable TTS failure, clear stale pre-roll again and resume wake listening when the application is eligible;
8. on disable or shutdown, stop wake activation and release wake/native/audio resources deterministically.

No V1 path permits wake activation while Moose is Talking. No V1 path intentionally starts a cloud service merely to detect the wake phrase.

## Acceptance boundaries

Accuracy claims are limited to deterministic corpus and real-platform acceptance evidence. Closeout requires positive and negative KWS fixtures, Linux x86_64 and macOS arm64 real pinned-runtime acceptance for claimed support, repeated lifecycle stability, architecture/hash verification, and performance evidence showing wake KWS is materially lighter than continuously running the project's full ASR path.

This document describes the target V1 contract. Individual implementation slices remain subject to `docs/WAKE_WORD_V1_TODO_2026-09-14.md`; a behavior described here is not considered complete until its corresponding TODO acceptance and exact-head qualification are reconciled.