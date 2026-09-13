# Talking Moose V1 Voice Selection

**Original Google voice decision:** 2026-08-22  
**KittenTTS closeout update:** 2026-09-12

Talking Moose V1 now has three separate voice ownership domains. Do not collapse them into one setting or one provider catalog.

## Voice ownership model

| Domain | Setting | Provider path | Notes |
| --- | --- | --- | --- |
| Google standalone TTS | `google_tts_voice` | Google Gemini TTS REST request | Used for typed replies, ambient remarks, canned reactions, and voice auditions when `tts_provider = google`. |
| Local standalone TTS | `local_tts_voice` | Local KittenTTS runtime | Used for the same standalone speech surfaces when `tts_provider = local`. Requires installed and verified Local TTS assets. |
| Gemini Live conversation voice | `live_voice` | Google Gemini Live WebSocket session | Used for live spoken conversations. Changing it is conversation-restart-sensitive and independent of standalone TTS voice settings. |

The provider selector chooses the standalone TTS provider. It does not change Gemini Live voice conversations. Gemini Live remains cloud-native live audio in V1.

## Google standalone default

Talking Moose V1 keeps **Fenrir** as the default Google standalone voice.

Google's Gemini TTS catalog labels Fenrir **Excitable**. That remains a deliberate fit for the retro-cartoon Moose when combined with the application's dry, lower-register performance direction: the voice supplies expressive energy while the prompt controls pace, register, and deadpan delivery. The choice is an original Talking Moose performance direction, not an attempt to reproduce any existing cartoon character or performer.

The backend owns and validates the Google voice catalog in `src-tauri/src/ai/google/config.rs`. Unsupported Google voice IDs normalize to the current default before a request is sent.

## Local KittenTTS voices

The Local KittenTTS V1 catalog is owned by the Rust Local TTS layer and currently exposes these eight voice IDs:

- `Bella`
- `Jasper`
- `Luna`
- `Bruno`
- `Rosie`
- `Hugo`
- `Kiki`
- `Leo`

`Bella` remains the initial Local TTS default until the owner completes the human voice audition gate. Unsupported Local voice IDs fail closed as setup errors. They must not normalize to a Google voice and must not trigger a Google TTS fallback.

## Settings migration

Settings version 4 split the former standalone/live voice state into provider-owned fields:

- legacy standalone Google-compatible voice values migrate to `google_tts_voice`;
- legacy live-compatible voice values migrate to `live_voice`;
- Local KittenTTS uses `local_tts_voice` and does not reinterpret a Google voice name as a Local voice;
- existing profiles without a standalone TTS provider selector remain on Google standalone TTS instead of being silently moved to Local.

This preserves user intent during upgrade and avoids mixing provider catalogs.

## Audition behavior

Voice audition is provider-aware:

- Google audition sends the fixed audition corpus to Google Gemini TTS and requires a configured Google API key.
- Local audition sends the same style of standalone speech request through the Local KittenTTS runtime and requires the selected Local model to be installed and verified.
- Local audition must fail explicitly if the model is missing, corrupt, unsupported, or unavailable. It must not fall back to Google.

The frontend blocks Local audition until Local assets are installed and verified.

## Rate and pitch

Google Gemini TTS is steered through natural-language performance direction. Talking Moose maps rate and pitch settings to bounded pace/register instructions while preserving the exact user-visible line as the line to recite.

Local KittenTTS exposes truthful rate handling through its provider/runtime path. Local pitch is not exposed as a V1 capability because the Local runtime does not implement a truthful pitch-control contract. The UI must therefore disable or mark pitch as unsupported when Local TTS is selected.

## Automated ASR evidence and human audition

The KittenTTS-to-Moonshine Tiny automated ASR smoke passed for all eight Kitten voices on macOS arm64. That proves the voices are machine-recognizable under the smoke-test phrase and configured WER/content-recall gates.

It does not decide which Local voice should be the Moose default. `KCR-330` / `KTT-805` remains open for owner audition of naturalness, comedic fit, timbre, pronunciation, artifacts, fatigue, and final default Local voice selection.

## Post-processing decision

V1 does **not** add local pitch shifting, EQ, compression, or other character-voice DSP after either Google or Local standalone synthesis. This avoids unnecessary latency, clipping risk, and a second voice-shaping path. If real-device auditions later reveal a concrete intelligibility or loudness problem, a bounded DSP stage can be reconsidered with measured acceptance criteria.

## Non-imitation rule

The selected voice and performance direction must remain an **original Talking Moose voice**. Do not instruct Google, KittenTTS, or any future voice processor to imitate Bullwinkle, Bill Scott, or another identifiable performer.
