# ELSV macOS System Voices — physical acceptance record

**Date:** 2026-10-15  
**Status:** Human audition and real-app playback observation **not yet performed**.  
**Scope:** `docs/EXTERNAL_LLM_SYSTEM_VOICES_TODO_2026-10-13.md` ELSV-500 / ELSV-900.

## Existing automated evidence (not a listening substitute)

The macOS arm64 System TTS acceptance workflow passed at exact SHA `f1cc46af09ae874c4c7c250e94af2c0a045abdf3`: [run 38797713849](https://github.com/ekkus93/ai-talking-moose/actions/runs/38797713849). Its **system-tts-audition** artifact contains generated WAVs and a machine-readable report. Non-silent PCM, selected-voice checks and cancellation do not prove human intelligibility, physical output playback or speech-bubble timing. Newer commits require their own exact-head CI.

## Physical Mac audition

1. Record Mac model, architecture, macOS version, exact app SHA (`git rev-parse HEAD`), build method, test date, output device and operator.
2. Open **Settings → Voice & Audio**, refresh the System voice catalog, select **System Default**, and click **Audition**. Verify audible, intelligible speech and no overlapping auditions.
3. Select an installed English OS voice and audition it at rates 1.0 and 2.0. Confirm exact selected voice, audible rate difference, no truncation and no Google-only pitch processing.
4. Select a local ASR mode (Whisper Small or Moonshine) and System TTS for a non-Live Talk turn. Record when audible speech begins/ends and when the speech bubble appears/disappears; the bubble must remain until playback ends.
5. Interrupt a long utterance and replace the turn. Confirm the old audio stops and does not restart and no stale bubble survives. Retest after changing the selected text provider.
6. Select an unavailable saved OS voice and refresh. Confirm the UI reports unavailable and synthesis refuses to substitute System Default, Google or KittenTTS.
7. If Gemini Live is configured separately, change standalone System voice while Live is active and confirm no Live interruption or voice change.
8. Qualify Intel macOS separately if claiming it: the prior synthesis acceptance run exercised arm64 only.

## Evidence fields

| Field | Observed result |
| --- | --- |
| Exact SHA / macOS version / architecture / app build | PENDING |
| Output device / human operator | PENDING |
| Default and selected voice audible and intelligible | PENDING |
| Speaking rate 1.0 versus 2.0 and no truncation | PENDING |
| Audible start/end and bubble disappearance timestamps | PENDING |
| Cancel/replacement and stale audio/bubble check | PENDING |
| Unavailable saved voice fails closed | PENDING |
| Gemini Live isolated from standalone provider change | PENDING |
| Intel macOS build or explicit deferral | PENDING |

Do not check ELSV-500's human audition or ELSV-900's physical acceptance item until real observations replace these placeholders. Do not commit credentials, audio artifacts, private paths or transcripts.
