use crate::character::state::{transition_character_state, CharacterState};
use parking_lot::RwLock;
use tauri::{Emitter, Runtime};

pub(crate) fn transition_and_emit<R: Runtime>(
    character_state: &RwLock<CharacterState>,
    app: &tauri::AppHandle<R>,
    target: CharacterState,
) -> Result<(), String> {
    transition_character_state(character_state, target)?;
    let _ = app.emit("moose://state", target);
    Ok(())
}

/// Restore the character's presentation state without showing, raising, or focusing
/// the native window. Ambient callers rely on this to remain non-focus-stealing.
pub(crate) fn show_character<R: Runtime>(
    character_state: &RwLock<CharacterState>,
    app: &tauri::AppHandle<R>,
) -> Result<(), String> {
    let current = *character_state.read();
    if current == CharacterState::Dismissed {
        transition_and_emit(character_state, app, CharacterState::Hidden)?;
    }
    if *character_state.read() == CharacterState::Hidden {
        transition_and_emit(character_state, app, CharacterState::Appearing)?;
    }
    transition_and_emit(character_state, app, CharacterState::Idle)
}

pub(crate) fn clear_speech_bubble<R: Runtime>(app: &tauri::AppHandle<R>) {
    let _ = app.emit("moose://speech-bubble", "");
}
