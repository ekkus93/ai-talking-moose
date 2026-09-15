#!/usr/bin/env python3
"""Generate the ephemeral Wake Word V1 acceptance corpus on macOS.

No audio is committed to the repository. CI synthesizes speech with the pinned
macOS runner's built-in voices, converts it to canonical 16 kHz mono PCM WAV,
and creates deterministic gain/noise variants with a fixed seed.
"""
from __future__ import annotations

import argparse
import json
import math
from pathlib import Path
import platform
import random
import shutil
import struct
import subprocess
import tempfile
import wave

SAMPLE_RATE = 16_000
SEED = 0x4D4F4F53  # "MOOS"
PREFERRED_VOICES = ("Samantha", "Alex", "Ava", "Allison")


def available_english_voices() -> list[str]:
    result = subprocess.run(
        ["say", "-v", "?"], check=True, capture_output=True, text=True
    )
    voices: list[str] = []
    for line in result.stdout.splitlines():
        parts = line.split()
        if len(parts) >= 2 and parts[1].lower().startswith("en_"):
            voices.append(parts[0])
    ordered = [voice for voice in PREFERRED_VOICES if voice in voices]
    ordered.extend(voice for voice in voices if voice not in ordered)
    return ordered


def validate_wav(path: Path) -> None:
    with wave.open(str(path), "rb") as handle:
        if (
            handle.getnchannels() != 1
            or handle.getsampwidth() != 2
            or handle.getframerate() != SAMPLE_RATE
        ):
            raise SystemExit(f"non-canonical generated WAV: {path.name}")


def synthesize(output: Path, voice: str, text: str, rate: int) -> None:
    with tempfile.TemporaryDirectory(prefix="wake-corpus-") as temporary:
        aiff = Path(temporary) / "speech.aiff"
        subprocess.run(
            [
                "say",
                "-v",
                voice,
                "-r",
                str(rate),
                "-o",
                str(aiff),
                "--data-format=LEI16@16000",
                text,
            ],
            check=True,
        )
        subprocess.run(
            [
                "afconvert",
                "-f",
                "WAVE",
                "-d",
                "LEI16@16000",
                "-c",
                "1",
                str(aiff),
                str(output),
            ],
            check=True,
        )
    validate_wav(output)


def transform(source: Path, output: Path, *, gain: float = 1.0, noise: int = 0) -> None:
    rng = random.Random(SEED + sum(output.name.encode("utf-8")))
    with wave.open(str(source), "rb") as handle:
        params = handle.getparams()
        frames = handle.readframes(handle.getnframes())
    samples = list(struct.unpack(f"<{len(frames) // 2}h", frames))
    transformed: list[int] = []
    for sample in samples:
        value = int(round(sample * gain))
        if noise:
            value += rng.randint(-noise, noise)
        transformed.append(max(-32768, min(32767, value)))
    with wave.open(str(output), "wb") as handle:
        handle.setparams(params)
        handle.writeframes(struct.pack(f"<{len(transformed)}h", *transformed))
    validate_wav(output)


def rms(path: Path) -> float:
    with wave.open(str(path), "rb") as handle:
        frames = handle.readframes(handle.getnframes())
    values = struct.unpack(f"<{len(frames) // 2}h", frames)
    if not values:
        return 0.0
    return math.sqrt(sum(value * value for value in values) / len(values))


def main() -> None:
    parser = argparse.ArgumentParser()
    parser.add_argument("--output", type=Path, required=True)
    args = parser.parse_args()
    if platform.system() != "Darwin":
        raise SystemExit("Wake Word corpus generation is intentionally pinned to macOS CI.")

    output = args.output.resolve()
    if output.exists():
        shutil.rmtree(output)
    output.mkdir(parents=True)

    voices = available_english_voices()
    if len(voices) < 2:
        raise SystemExit("Wake Word acceptance requires at least two built-in English voices.")
    voice_a, voice_b = voices[:2]

    definitions = [
        ("positive_voice_a.wav", voice_a, "Hey Moose", 165, True, "positive/speaker-a"),
        ("positive_voice_b.wav", voice_b, "Hey, Moose", 180, True, "positive/speaker-b"),
        ("positive_command.wav", voice_a, "Hey Moose, tell me the time", 175, True, "positive/immediate-command"),
        ("positive_slow.wav", voice_b, "Hey Moose", 135, True, "positive/pronunciation-rate"),
        ("negative_ordinary.wav", voice_a, "Please tell me the time", 175, False, "negative/ordinary-speech"),
        ("negative_moose.wav", voice_b, "Moose", 175, False, "negative/moose-alone"),
        ("negative_bruce.wav", voice_a, "Hey Bruce", 175, False, "negative/near-miss-bruce"),
        ("negative_moosey.wav", voice_b, "Hey Moosey", 175, False, "negative/near-miss-moosey"),
        ("negative_similar.wav", voice_a, "May moves slowly", 175, False, "negative/phonetic-near-miss"),
        ("negative_sentence.wav", voice_b, "There is a moose near the lake", 175, False, "negative/moose-in-sentence"),
        ("negative_background_speech.wav", voice_a, "Welcome back to the program. Today we discuss weather, traffic, science, and music.", 190, False, "negative/background-speech"),
    ]

    cases: list[dict[str, object]] = []
    for filename, voice, text, rate, expect, category in definitions:
        path = output / filename
        synthesize(path, voice, text, rate)
        cases.append(
            {
                "path": filename,
                "expect_trigger": expect,
                "category": category,
                "voice": voice,
                "rate_wpm": rate,
                "rms": round(rms(path), 2),
            }
        )

    base = output / "positive_voice_a.wav"
    for filename, gain, noise, category in [
        ("positive_quiet.wav", 0.45, 0, "positive/low-gain"),
        ("positive_loud.wav", 1.45, 0, "positive/high-gain"),
        ("positive_noise.wav", 1.0, 350, "positive/deterministic-noise"),
    ]:
        path = output / filename
        transform(base, path, gain=gain, noise=noise)
        cases.append(
            {
                "path": filename,
                "expect_trigger": True,
                "category": category,
                "voice": voice_a,
                "rate_wpm": 165,
                "rms": round(rms(path), 2),
            }
        )

    manifest = {
        "schema_version": 1,
        "generator": "macOS say + afconvert + deterministic Python PCM transforms",
        "runner_contract": "macos-15",
        "seed": SEED,
        "sample_rate_hz": SAMPLE_RATE,
        "channels": 1,
        "sample_width_bytes": 2,
        "voices": [voice_a, voice_b],
        "cases": cases,
    }
    (output / "manifest.json").write_text(
        json.dumps(manifest, indent=2, sort_keys=True) + "\n", encoding="utf-8"
    )
    print(
        f"wake-corpus-generated cases={len(cases)} positives={sum(bool(c['expect_trigger']) for c in cases)} negatives={sum(not bool(c['expect_trigger']) for c in cases)} voices={voice_a},{voice_b}"
    )


if __name__ == "__main__":
    main()
