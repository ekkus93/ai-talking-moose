use serde::Serialize;
use talking_moose_lib::ai::google::{
    GoogleModelDescriptor, GoogleTtsVoiceDescriptor, GOOGLE_MODELS, GOOGLE_TTS_VOICES,
};
use talking_moose_lib::app::state::AppSettings;

#[derive(Serialize)]
struct FrontendContract<'a> {
    settings: AppSettings,
    google_models: &'a [GoogleModelDescriptor],
    google_tts_voices: &'a [GoogleTtsVoiceDescriptor],
}

fn main() {
    let contract = FrontendContract {
        settings: AppSettings::default(),
        google_models: GOOGLE_MODELS,
        google_tts_voices: GOOGLE_TTS_VOICES,
    };
    println!(
        "{}",
        serde_json::to_string_pretty(&contract).expect("frontend contract must serialize")
    );
}
