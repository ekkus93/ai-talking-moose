//! ELSV-200: Google and OpenAI credentials are independent and never part of
//! serialized settings. Mirrors the exact calls made by the
//! `clear_google_api_key` / `clear_openai_api_key` Tauri commands.
use super::super::*;

const GOOGLE: &str = "google-key-ELSV200-aaaa";
const OPENAI: &str = "sk-openai-key-ELSV200-bbbb";

fn clear_google_like_command(state: &AppState) {
    state.secrets.clear().unwrap();
    state.db.delete_setting("google_api_key").unwrap();
}

fn clear_openai_like_command(state: &AppState) {
    state.secrets.clear_openai_api_key().unwrap();
    state.db.delete_setting("openai_api_key").unwrap();
}

fn assert_no_key_serialized(state: &AppState) {
    let settings = state.settings.read().clone();
    let json = serde_json::to_string(&settings).unwrap();
    assert!(!json.contains(GOOGLE) && !json.contains(OPENAI));
    let lower = json.to_ascii_lowercase();
    assert!(!lower.contains("api_key") && !lower.contains("apikey"));
    assert!(state.db.get_setting("app_settings").unwrap().is_none_or(|s| {
        !s.contains(GOOGLE) && !s.contains(OPENAI)
    }));
}

#[test]
fn clearing_one_provider_key_leaves_the_other_and_settings_stay_key_free() {
    let state = AppState::new_for_tests().unwrap();
    state.secrets.set_google_api_key(GOOGLE.into()).unwrap();
    state.secrets.set_openai_api_key(OPENAI.into()).unwrap();
    assert_no_key_serialized(&state);

    clear_google_like_command(&state);
    assert!(!state.secrets.has_google_api_key());
    assert_eq!(state.secrets.get_openai_api_key().as_deref(), Some(OPENAI));
    assert_no_key_serialized(&state);

    state.secrets.set_google_api_key(GOOGLE.into()).unwrap();
    clear_openai_like_command(&state);
    assert!(!state.secrets.has_openai_api_key());
    assert_eq!(state.secrets.get_google_api_key().as_deref(), Some(GOOGLE));
    assert_no_key_serialized(&state);

    // Clearing an already-empty slot is idempotent and still isolated.
    clear_openai_like_command(&state);
    assert_eq!(state.secrets.get_google_api_key().as_deref(), Some(GOOGLE));
}
