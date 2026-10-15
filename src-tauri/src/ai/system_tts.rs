//! Platform facade for OS-provided speech. Only macOS (AVSpeechSynthesizer
//! buffer synthesis) is implemented. Every other platform reports System voices
//! as unsupported and fails closed; there is no subprocess or Google/Local fallback.
use crate::ai::system_tts_selection::SYSTEM_DEFAULT_VOICE_ID;
use crate::ai::traits::SpeechSynthesizer;
use crate::ai::tts_catalog::{TtsProviderDescriptor, TtsVoiceDescriptor};
use crate::ai::types::TtsProvider;

#[cfg(target_os = "macos")]
mod macos;

/// AVSpeechUtterance's documented default rate; our `speaking_rate` 1.0 maps here.
#[cfg_attr(not(target_os = "macos"), allow(dead_code))]
const AV_DEFAULT_RATE: f32 = 0.5;
#[cfg_attr(not(target_os = "macos"), allow(dead_code))]
const AV_MAX_RATE: f32 = 1.0;

pub fn is_supported() -> bool {
    cfg!(target_os = "macos")
}

/// Map the app's 0.25–4.0 speaking-rate multiplier (1.0 = normal) onto
/// AVSpeechUtterance's 0.0–1.0 scale (0.5 = normal), monotonically and clamped.
#[cfg_attr(not(target_os = "macos"), allow(dead_code))]
pub(crate) fn av_speech_rate(speaking_rate: f32) -> f32 {
    let rate = if speaking_rate.is_finite() {
        speaking_rate.clamp(0.25, 4.0)
    } else {
        1.0
    };
    if rate <= 1.0 {
        AV_DEFAULT_RATE * rate
    } else {
        AV_DEFAULT_RATE + (rate - 1.0) / 3.0 * (AV_MAX_RATE - AV_DEFAULT_RATE)
    }
}

/// Voices currently installed in the OS, already normalized and without the
/// explicit System Default entry. Empty when unsupported.
pub fn installed_voices() -> Vec<TtsVoiceDescriptor> {
    #[cfg(target_os = "macos")]
    {
        crate::ai::system_tts_voice_catalog::normalize_installed_voices(macos::installed_voices())
    }
    #[cfg(not(target_os = "macos"))]
    {
        Vec::new()
    }
}

/// The System provider catalog entry, present only where it can actually speak.
pub fn provider_descriptor() -> Option<TtsProviderDescriptor> {
    if !is_supported() {
        return None;
    }
    let mut voices = vec![TtsVoiceDescriptor {
        id: SYSTEM_DEFAULT_VOICE_ID.to_string(),
        display_name: "System Default".to_string(),
        style: None,
    }];
    voices.extend(installed_voices());
    Some(TtsProviderDescriptor {
        id: TtsProvider::System,
        display_name: "macOS System Voices".to_string(),
        is_local: true,
        models: Vec::new(),
        voices,
        // Informational: the real rate is taken from each synthesized buffer.
        sample_rate_hz: 22_050,
        supports_speaking_rate: true,
        supports_pitch: false,
        install_required: false,
        license_summary:
            "Uses voices already installed in macOS; the application distributes no speech assets."
                .to_string(),
    })
}

pub fn synthesizer() -> Box<dyn SpeechSynthesizer> {
    #[cfg(target_os = "macos")]
    {
        Box::new(macos::MacSystemSpeechSynthesizer)
    }
    #[cfg(not(target_os = "macos"))]
    {
        Box::new(crate::ai::unavailable::UnavailableSpeechSynthesizer)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rate_mapping_is_monotonic_anchored_and_bounded() {
        assert_eq!(av_speech_rate(1.0), 0.5);
        assert_eq!(av_speech_rate(4.0), 1.0);
        assert!((av_speech_rate(0.25) - 0.125).abs() < 1e-6);
        let mut previous = av_speech_rate(0.25);
        for step in 1..=75 {
            let next = av_speech_rate(0.25 + step as f32 * 0.05);
            assert!(next >= previous && (0.0..=1.0).contains(&next));
            previous = next;
        }
        assert_eq!(av_speech_rate(100.0), 1.0);
        assert_eq!(av_speech_rate(f32::NAN), 0.5);
    }

    #[cfg(not(target_os = "macos"))]
    #[test]
    fn unsupported_platforms_expose_no_system_provider_or_voices() {
        assert!(!is_supported());
        assert!(provider_descriptor().is_none());
        assert!(installed_voices().is_empty());
    }
}
