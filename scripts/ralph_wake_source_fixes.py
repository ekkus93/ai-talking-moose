#!/usr/bin/env python3
from pathlib import Path

acceptance = Path("src-tauri/src/bin/wake_word_acceptance.rs")
text = acceptance.read_text(encoding="utf-8")
old = '''    Ok(data
        .chunks_exact(2)
        .map(|chunk| i16::from_le_bytes([chunk[0], chunk[1]]))
        .collect())'''
new = '''    Ok(data
        .as_chunks::<2>()
        .0
        .iter()
        .map(|chunk| i16::from_le_bytes(*chunk))
        .collect())'''
if old not in text:
    raise SystemExit("wake acceptance PCM conversion shape drifted")
acceptance.write_text(text.replace(old, new, 1), encoding="utf-8")

runtime = Path("src-tauri/src/audio/wake_word/runtime.rs")
text = runtime.read_text(encoding="utf-8")
needle = "vec![1_u8, 0_u8].repeat(1600)"
count = text.count(needle)
if count < 1:
    raise SystemExit("wake runtime Clippy fixture shape drifted")
runtime.write_text(text.replace(needle, "[1_u8, 0_u8].repeat(1600)"), encoding="utf-8")

character = Path("src-tauri/src/commands/character.rs")
text = character.read_text(encoding="utf-8")
old = '''        let (pcm_tx, _pcm_rx) = tokio::sync::mpsc::channel(1);
        capture.lock().start(None, 16_000, pcm_tx, None).unwrap();
        playback.seed_buffer_for_tests(&[0.25, -0.25, 0.5], 0.5);'''
new = '''        playback.seed_buffer_for_tests(&[0.25, -0.25, 0.5], 0.5);'''
if old not in text:
    raise SystemExit("character interaction capture setup drifted")
text = text.replace(old, new, 1)
marker = '''        .expect("interaction setup state should succeed through IPC");

        InteractionTestFixture {'''
replacement = '''        .expect("interaction setup state should succeed through IPC");

        // Build/setup may reconcile inactive wake capture. Arm the explicit mock
        // microphone only after the presentation state is established so these
        // tests exercise mute/dismiss teardown from an actually active capture.
        let (pcm_tx, _pcm_rx) = tokio::sync::mpsc::channel(1);
        capture.lock().start(None, 16_000, pcm_tx, None).unwrap();

        InteractionTestFixture {'''
if marker not in text:
    raise SystemExit("character interaction fixture return marker drifted")
character.write_text(text.replace(marker, replacement, 1), encoding="utf-8")

policy = Path("scripts/check_local_tts_packaging_policy.py")
text = policy.read_text(encoding="utf-8")
old = '''    resources = bundle.get("resources", [])
    if resources != ["native/macos/notices/"]:
        fail("ordinary Tauri resources must remain notice-only; Local TTS artifacts are installer-owned")'''
new = '''    resources = bundle.get("resources", [])
    allowed_resources = {"native/macos/notices/", "resources/wake_word/"}
    if len(resources) != len(allowed_resources) or set(resources) != allowed_resources:
        fail(
            "ordinary Tauri resources may contain notices plus the independently pinned "
            "Wake Word subtree; Local TTS artifacts remain installer-owned"
        )'''
if old not in text:
    raise SystemExit("Local TTS bundle resource guard drifted")
policy.write_text(text.replace(old, new, 1), encoding="utf-8")
