# Talking Moose V1 Voice Decision

**Decision date:** 2026-08-22

## Default voice

Talking Moose V1 keeps **Fenrir** as the default Gemini voice.

Google's current Gemini TTS catalog labels Fenrir **Excitable**. That is a deliberate fit for the retro-cartoon Moose when combined with the application's dry, lower-register performance direction: the voice supplies expressive energy while the prompt controls pace, register, and deadpan delivery. The choice is an original Talking Moose performance direction, not an attempt to reproduce any existing cartoon character or performer.

The backend owns the supported 30-voice catalog and validates every selected voice. Unsupported identifiers normalize to the current default.

## Audition method

Every voice preset uses the same fixed audition corpus. The corpus deliberately contains:

- a greeting;
- a dry/sarcastic line;
- an annoyed-but-comedic line;
- a very short explanation;
- a longer explanation.

Using identical text for every candidate removes the previous bias where different voices received scripts tailored to their labels. The existing fake synthesizer keeps this audition path available to ordinary tests without a live Google request.

## Provider configuration

The authoritative production TTS model and voice catalog live in Rust under `src-tauri/src/ai/google/config.rs`. Standalone V1 TTS uses `gemini-2.5-flash-preview-tts`; persisted legacy `en-US-Standard-B` settings are accepted as a migration alias and normalized at the provider boundary. Invalid voice IDs are likewise normalized before a request is sent.

## Rate and pitch

Gemini TTS is steered through natural-language performance direction. Talking Moose maps the existing rate and pitch settings to bounded pace/register instructions while preserving the exact user-visible line as the line to recite. Provider failures return typed safe errors and do not silently switch to another production speech engine.

## Post-processing decision

V1 does **not** add local pitch shifting, EQ, compression, or other character-voice DSP after Gemini synthesis. This avoids unnecessary latency, clipping risk, and a second voice-shaping path. If real-device auditions later reveal a concrete intelligibility or loudness problem, a bounded DSP stage can be reconsidered with measured acceptance criteria.

## Non-imitation rule

The selected voice and performance direction must remain an **original Talking Moose voice**. Do not instruct Gemini or any future voice processor to imitate Bullwinkle, Bill Scott, or another identifiable performer.
