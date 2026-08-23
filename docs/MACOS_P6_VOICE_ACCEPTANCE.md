# P6 macOS Character / Voice Acceptance

Status: **Ready for physical listening acceptance**

This procedure closes the two remaining P6 acceptance items without treating an automated waveform or provider label as a substitute for human listening:

1. an intentional comparison across every currently supported Gemini TTS voice; and
2. explicit standalone-TTS cancellation, including cancellation while synthesis is still in flight and while generated audio is already playing.

The source-level character, prompt-budget, personality-policy, fixed-corpus, TTS hardening, and no-DSP decisions are already reconciled in `RECONCILIATION_P0_P1_P6_20260822.md`.

## Preconditions

- Run a current `master` build on macOS.
- Configure a real Google API key through Talking Moose Settings.
- Select the actual output device you intend to use.
- Keep the default speaking rate/pitch initially (`0.95`, `-1.5`) so voices are compared under the same performance direction.
- Open Settings and click **P6 Voice Acceptance**. Its acceptance-only selector is populated from the authoritative Rust Gemini voice catalog, so the listening pass is not limited to the normal preset shortlist.

## Fixed audition corpus

Every voice must receive exactly the same corpus:

> Hello, I'm Moose. Oh good, another button. Professionally disappointed. Short version: it works. Longer version: I explain things while looking bewildered.

The corpus is deliberately short enough to fit the bounded ten-second standalone playback queue while still covering greeting, dry/sarcastic delivery, annoyed comedy, a short explanation, and a longer explanatory phrase.

Do not rewrite the corpus per voice. The purpose is an apples-to-apples comparison.

## Voice comparison

Audition all voices shown in the selector. For each voice, record a 1–5 score for:

| Criterion            | What to listen for                                             |
| -------------------- | -------------------------------------------------------------- |
| Character fit        | Feels plausible for the original retro Talking Moose character |
| Dry/deadpan delivery | Sarcasm lands without becoming melodramatic                    |
| Intelligibility      | Words remain clear at the default rate/register                |
| Long-line stability  | Longer phrase stays natural and understandable                 |
| Fatigue              | Voice would remain tolerable during repeated desktop use       |

Reject a voice regardless of score if it sounds too close to an identifiable existing performer/character or encourages an imitation-oriented direction. The product voice must remain an original Talking Moose performance.

Fenrir remains the provisional V1 default until this listening comparison says otherwise. Do not change the default merely because another voice has a more attractive provider style label.

## Cancellation acceptance

Perform both cases with at least Fenrir and the slowest-feeling voice from the listening pass.

### A. Cancel while synthesis is in flight

1. Click **Audition "<voice>" Voice Sample**.
2. Immediately click **Stop Sample**, before speech begins if network/provider latency permits.
3. Pass only if no stale audio starts later after cancellation.
4. Repeat several times, including rapid audition → stop → audition of another voice.

### B. Cancel during playback

1. Start a voice audition and let speech become audible.
2. Click **Stop Sample** mid-utterance.
3. Pass only if audible playback stops promptly, the playback queue is flushed, and the cancelled tail never resumes.
4. Start another audition and verify the new voice begins cleanly without stale audio from the previous sample.

Also verify Mute and Dismiss during a standalone sample do not allow an in-flight TTS response to produce audio afterward.

## Acceptance record

Record:

- date;
- tested Talking Moose commit;
- Mac model / macOS version;
- output device;
- voices auditioned (must match the complete selector catalog);
- shortlist/final choice and rationale;
- cancellation cases A/B pass/fail;
- Mute/Dismiss stale-audio check pass/fail;
- any clipping, truncation, latency, or intelligibility issue.

P6 may be marked fully accepted only after the real listening comparison and cancellation checks are recorded. Automated unit/UI tests verify the mechanism but do not choose a voice by listening.
