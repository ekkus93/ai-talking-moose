#!/usr/bin/env python3
from __future__ import annotations

import argparse
import hashlib
import json
import pathlib
import struct
import urllib.request
import wave

SOURCE_COMMIT = "45f1593fd326b3435c04392e3151dff65967e523"
SOURCE_BLOB_SHA1 = "3184d372cd2f8b804d3a540c70ec50d927b335d2"
SOURCE_URL = (
    "https://raw.githubusercontent.com/ggml-org/whisper.cpp/"
    f"{SOURCE_COMMIT}/samples/jfk.wav"
)
EXPECTED_SAMPLE_RATE_HZ = 16_000
EXPECTED_CHANNELS = 1
EXPECTED_SAMPLE_WIDTH_BYTES = 2
EXPECTED_SOURCE_FRAMES = 176_000
TRAILING_SILENCE_MS = 2_000
CHUNK_MS = 100


def git_blob_sha1(data: bytes) -> str:
    header = f"blob {len(data)}\0".encode()
    return hashlib.sha1(header + data).hexdigest()  # noqa: S324 - Git object identity.


def download_source() -> bytes:
    request = urllib.request.Request(
        SOURCE_URL,
        headers={"User-Agent": "talking-moose-asr015-acceptance/1"},
    )
    with urllib.request.urlopen(request, timeout=60) as response:
        if response.status != 200:
            raise SystemExit(f"benchmark corpus download returned HTTP {response.status}")
        return response.read()


def wav_pcm(data: bytes) -> bytes:
    import io

    with wave.open(io.BytesIO(data), "rb") as reader:
        if reader.getnchannels() != EXPECTED_CHANNELS:
            raise SystemExit("benchmark WAV must be mono")
        if reader.getsampwidth() != EXPECTED_SAMPLE_WIDTH_BYTES:
            raise SystemExit("benchmark WAV must be signed 16-bit PCM")
        if reader.getframerate() != EXPECTED_SAMPLE_RATE_HZ:
            raise SystemExit("benchmark WAV must be 16 kHz")
        if reader.getnframes() != EXPECTED_SOURCE_FRAMES:
            raise SystemExit(
                f"benchmark WAV frame count changed: {reader.getnframes()} != {EXPECTED_SOURCE_FRAMES}"
            )
        if reader.getcomptype() != "NONE":
            raise SystemExit("benchmark WAV must be uncompressed PCM")
        return reader.readframes(reader.getnframes())


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("pcm_output", type=pathlib.Path)
    parser.add_argument("metadata_output", type=pathlib.Path)
    args = parser.parse_args()

    source = download_source()
    actual_blob = git_blob_sha1(source)
    if actual_blob != SOURCE_BLOB_SHA1:
        raise SystemExit(
            f"benchmark source Git blob mismatch: expected {SOURCE_BLOB_SHA1}, got {actual_blob}"
        )

    pcm = wav_pcm(source)
    silence_frames = EXPECTED_SAMPLE_RATE_HZ * TRAILING_SILENCE_MS // 1_000
    pcm += struct.pack("<h", 0) * silence_frames
    bytes_per_chunk = (
        EXPECTED_SAMPLE_RATE_HZ
        * EXPECTED_CHANNELS
        * EXPECTED_SAMPLE_WIDTH_BYTES
        * CHUNK_MS
        // 1_000
    )
    if len(pcm) % bytes_per_chunk:
        pcm += b"\0" * (bytes_per_chunk - len(pcm) % bytes_per_chunk)

    args.pcm_output.parent.mkdir(parents=True, exist_ok=True)
    args.metadata_output.parent.mkdir(parents=True, exist_ok=True)
    args.pcm_output.write_bytes(pcm)

    frames = len(pcm) // (EXPECTED_CHANNELS * EXPECTED_SAMPLE_WIDTH_BYTES)
    metadata = {
        "source_repository": "ggml-org/whisper.cpp",
        "source_path": "samples/jfk.wav",
        "source_commit": SOURCE_COMMIT,
        "source_git_blob_sha1": SOURCE_BLOB_SHA1,
        "source_sha256": hashlib.sha256(source).hexdigest(),
        "source_bytes": len(source),
        "source_frames": EXPECTED_SOURCE_FRAMES,
        "source_duration_ms": EXPECTED_SOURCE_FRAMES * 1_000 // EXPECTED_SAMPLE_RATE_HZ,
        "trailing_silence_ms": TRAILING_SILENCE_MS,
        "sample_rate_hz": EXPECTED_SAMPLE_RATE_HZ,
        "channels": EXPECTED_CHANNELS,
        "sample_width_bytes": EXPECTED_SAMPLE_WIDTH_BYTES,
        "corpus_frames": frames,
        "corpus_duration_ms": frames * 1_000 // EXPECTED_SAMPLE_RATE_HZ,
        "corpus_bytes": len(pcm),
        "corpus_sha256": hashlib.sha256(pcm).hexdigest(),
    }
    args.metadata_output.write_text(json.dumps(metadata, indent=2) + "\n", encoding="utf-8")
    print(json.dumps(metadata, sort_keys=True))
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
