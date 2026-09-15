# Wake Word V1 Microphone Routing Design

**Scope:** WW-400 / WW-410 implementation contract

## Decision

Wake Word V1 uses **one authoritative microphone capture stream**. Wake KWS and command ASR are consumers of a single chronological canonical-PCM route; they are not independent owners that may open the input device concurrently.

This is the required design because the existing application already has an authoritative `AudioCapture` object in `AppState`. Adding a second long-lived wake capture would create device contention, nondeterministic disconnect/reconnect behavior, and sample gaps at wake-to-command transfer. The wake implementation must therefore extend routing around the existing capture owner rather than instantiate a competing capture owner.

## Canonical audio route

The capture owner produces the application's canonical microphone PCM. Wake Word V1 consumes 16 kHz mono signed-16-bit PCM. Resampling/channel normalization must happen once in the authoritative capture/routing path where practical; KWS and the wake pre-roll buffer must receive the same ordered samples.

While wake mode is enabled and the conversation is eligible for wake listening, each canonical chunk is routed in this order:

1. append the chunk to `WakeWordRuntimeManager`'s bounded `PcmRingBuffer`;
2. feed the same chronological chunk to `SherpaKwsEngine`;
3. if no trigger occurs, continue the same route;
4. if the engine reports the canonical wake phrase, atomically accept one runtime trigger and capture one chronological pre-roll snapshot.

No wake path may create a second `AudioCapture` merely to simplify KWS integration.

## Wake-to-command ownership transfer

An accepted wake trigger changes routing state before command activation so repeated positive KWS frames cannot create duplicate commands. The runtime then transfers the one-shot pre-roll snapshot to command ASR.

The command-ASR input sequence is:

1. the captured chronological pre-roll snapshot, including the wake phrase;
2. every canonical live sample after the snapshot boundary, in original order;
3. no intentionally duplicated sample range and no intentional gap.

Live capture must remain owned by the authoritative capture path while command ASR initializes. Samples arriving during initialization must be retained through a bounded handoff mechanism rather than discarded. This is necessary for utterances such as `Hey Moose, tell me the time`, where the first command word can begin immediately after the keyword.

V1 does **not** strip `Hey, Moose` acoustically. Downstream ASR is allowed to transcribe the wake phrase together with the prompt.

If command ASR startup fails after a trigger, stale pre-roll/handoff audio is cleared and the wake runtime returns to a recoverable state when policy permits. Manual interaction remains available even if wake initialization or handoff fails.

## Command-to-wake ownership transfer

Wake activation stays suspended throughout command ASR, Thinking, and especially Talking according to the conversation-lifecycle policy. On entry to Talking, retained wake audio is cleared. It remains suspended for the complete TTS playback interval; V1 has no barge-in.

After successful TTS completion, TTS cancellation, or a recoverable TTS failure, stale wake audio is cleared again before wake listening resumes. The same authoritative capture owner is reused; the implementation must not accumulate capture streams across repeated cycles.

## Device failure and cancellation

Microphone unavailable/disconnect errors must terminate or suspend the current route without spin loops. A reconnect/retry may reopen only through the authoritative capture owner. Wake state must report a sanitized error and preserve the manual interaction path where possible.

Cancellation must be able to stop wake load/listen/handoff work at defined boundaries. Disable and application shutdown must clear buffered wake audio, stop KWS work, and release native/runtime resources. Shutdown is idempotent.

## Concurrency invariants

The implementation and acceptance tests must enforce all of the following:

- at most one authoritative microphone capture stream is open for the wake/ASR route;
- at most one wake runtime owns KWS state;
- one accepted wake event creates at most one command activation;
- KWS and pre-roll observe the same chronological canonical samples;
- command ASR receives pre-roll before post-trigger live samples;
- samples are not intentionally duplicated across the pre-roll/live boundary;
- wake activation cannot occur while Moose is Talking;
- disable/error/shutdown clears stale wake audio;
- repeated wake → ASR → Thinking → Talking → wake cycles do not multiply capture streams.

## Diagnostics and privacy

Routing diagnostics may expose device state, route state, sample rate/channels, bounded queue/ring capacities, dropped-chunk counters, transition counts, and sanitized errors. They must not expose raw PCM, ring-buffer contents, utterance text, credentials, or unnecessary filesystem paths.

## Acceptance implications

WW-400/WW-410 are not complete merely because this design is documented. Source integration must prove the one-stream invariant and continuous handoff with deterministic tests. At minimum, acceptance must cover repeated cycles, a wake-plus-immediate-command fixture, pre-roll/live ordering, ASR-startup failure recovery, device-unavailable behavior, and shutdown during listening/handoff.