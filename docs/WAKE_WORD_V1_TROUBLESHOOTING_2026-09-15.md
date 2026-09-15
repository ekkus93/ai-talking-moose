# Wake Word V1 troubleshooting

Wake Word V1 is an opt-in local keyword-spotting path. It is disabled by default and the V1 phrase is fixed to **Hey, Moose**. Enabling it keeps the selected microphone locally active for keyword detection; idle keyword spotting does not perform full transcription and does not require a cloud service.

## Runtime states

Privacy-safe diagnostics expose the wake runtime phase, whether wake mode is enabled, the fixed `sherpa-onnx-kws` engine identity, platform/architecture, one-thread inference policy, canonical 16 kHz mono input format, fixed V1 threshold/score, bounded ring-buffer occupancy/capacity, trigger count/age, Talking suspension, and a sanitized last error. Diagnostics intentionally cannot contain raw PCM, transcripts, credentials, or filesystem paths.

`disabled` means manual interaction remains authoritative. `loading` means the local KWS runtime is being prepared. `listening` means local keyword detection is active. `triggered` means one wake event has been accepted and command handoff is in progress. `suspended_talking` prevents Moose from waking itself during TTS playback. `error` is fail-closed for wake activation while preserving manual interaction paths. `shutting_down` rejects new wake work while resources are released.

## Audio handoff

Wake listening and command ASR must not open competing microphone streams. The wake path retains at most two seconds of canonical mono PCM in memory. On an accepted trigger, the chronological pre-roll is transferred once to command ASR and live post-trigger audio follows it. V1 intentionally does **not** acoustically remove `Hey, Moose`; downstream ASR may therefore receive the wake phrase plus the command as one continuous utterance. Immediate command words after the wake phrase are preserved by the bounded handoff buffer while ASR starts.

The pre-roll and handoff buffers are memory-only. They are cleared on disable, Talking suspension, interaction completion/recovery, runtime error, and shutdown. Raw buffered audio must never be serialized or logged.

## Common failure modes

If wake mode remains `disabled`, confirm the persisted Wake Word setting is enabled; do not infer enablement from microphone activity elsewhere in the application. If the runtime enters `error`, use the sanitized diagnostic plus model/runtime verification evidence; artifact mismatch and unsupported architecture are expected to fail closed before inference. If a microphone disappears, the routing layer must release ownership deterministically rather than spin or open a second capture stream. Manual listening remains the recovery path when wake activation is unavailable.

If the Moose does not react while it is speaking, that is expected V1 behavior. Wake activation is suspended for the full Talking interval and resumes only after playback completion, cancellation, or recoverable TTS failure. Barge-in is outside Wake Word V1.

Repeated positive KWS frames for one phrase must create only one command activation. A later phrase can activate a new interaction only after the lifecycle returns to wake listening. Trigger diagnostics are counters/timing only and never include the triggering audio.

## Privacy and provider boundaries

Idle KWS is local/offline. Wake failure must not silently start continuous full ASR and must not fall back to Google, Gemini, or another cloud provider. Once a wake event intentionally starts the normal command interaction, the application's separately selected ASR/conversation provider rules apply; enabling Wake Word does not change those provider choices.

For artifact provenance, packaging, architecture verification, and licensing, see `WAKE_WORD_V1_ARCHITECTURE_2026-09-15.md` and the pinned Wake Word artifact manifests/scripts. For the authoritative microphone ownership design, see `WAKE_WORD_V1_MICROPHONE_ROUTING_2026-09-15.md`.
