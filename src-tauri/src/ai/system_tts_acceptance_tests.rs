use super::system_tts_request::validate_system_tts_request;
use super::tts_catalog::TtsVoiceDescriptor;
use super::types::{ProviderErrorKind, TtsRequest};

#[test]
fn saved_native_voice_does_not_fall_back_when_os_catalog_changes() {
    let installed = vec![TtsVoiceDescriptor {
        id: "com.apple.voice.available".into(),
        display_name: "Available".into(),
        style: Some("en-US".into()),
    }];
    let mut request = TtsRequest {
        text: "Speak this sentence.".into(),
        voice_name: Some("com.apple.voice.available".into()),
        speaking_rate: Some(1.0),
        pitch: None,
    };
    assert!(validate_system_tts_request(&request, &installed).is_ok());
    request.voice_name = Some("com.apple.voice.removed".into());
    let error = validate_system_tts_request(&request, &installed).unwrap_err();
    assert_eq!(error.kind, ProviderErrorKind::Model);
    assert!(!error.retryable);
    request.voice_name = Some("system_default".into());
    assert!(validate_system_tts_request(&request, &[]).is_ok());
}

#[test]
fn invisible_saved_system_voice_identifier_is_not_silently_substituted() {
    let hidden = format!("com.apple.voice.{}spoof", char::from_u32(0x200b).unwrap());
    let installed = vec![TtsVoiceDescriptor {
        id: hidden.clone(),
        display_name: "Hidden voice".into(),
        style: None,
    }];
    let request = TtsRequest {
        text: "Speak this sentence.".into(),
        voice_name: Some(hidden),
        speaking_rate: None,
        pitch: None,
    };
    let error = validate_system_tts_request(&request, &installed).unwrap_err();
    assert_eq!(error.kind, ProviderErrorKind::Model);
    assert!(!error.retryable);
}

#[test]
fn malformed_os_voice_metadata_cannot_make_a_saved_voice_appear_available() {
    let voice = |id: &str, name: &str| TtsVoiceDescriptor {
        id: id.into(),
        display_name: name.into(),
        style: Some("en-US".into()),
    };
    let saved_id = "com.apple.voice.selected";
    let request = TtsRequest {
        text: "Speak this sentence.".into(),
        voice_name: Some(saved_id.into()),
        speaking_rate: None,
        pitch: None,
    };
    let catalog = vec![
        voice(
            saved_id,
            &format!("Spoofed{} Name", char::from_u32(0x200b).unwrap()),
        ),
        voice("com.apple.voice.other", "Valid Other Voice"),
    ];
    let error = validate_system_tts_request(&request, &catalog).unwrap_err();
    assert_eq!(error.kind, ProviderErrorKind::Model);
    assert!(!error.retryable);
    let mut recovered = catalog;
    recovered.push(voice(saved_id, "Selected Voice"));
    assert!(validate_system_tts_request(&request, &recovered).is_ok());
}
