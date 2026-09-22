#!/usr/bin/env python3
"""Generate the deterministic Wake Word V1 acceptance corpus from text recipes."""
from __future__ import annotations

import argparse
import array
import hashlib
import json
import math
import random
import shutil
import struct
import subprocess
import sys
import tempfile
import wave
from pathlib import Path

MANIFEST = Path(__file__).resolve().parents[1] / "docs" / "wake-word-corpus.json"


class CorpusGenerationError(RuntimeError):
    pass


def load_manifest() -> dict:
    try:
        manifest = json.loads(MANIFEST.read_text(encoding="utf-8"))
    except Exception as exc:
        raise CorpusGenerationError("invalid Wake Word corpus manifest") from exc
    if manifest.get("schema_version") != 2:
        raise CorpusGenerationError("Wake Word corpus manifest schema must be 2")
    return manifest


def tool_version(binary: str) -> str:
    try:
        result = subprocess.run(
            [binary, "--version"],
            check=True,
            capture_output=True,
            text=True,
            timeout=10,
        )
    except Exception as exc:
        raise CorpusGenerationError(
            "required deterministic speech generator is unavailable"
        ) from exc
    return (result.stdout or result.stderr).splitlines()[0].strip()


def synthesize(binary: str, fixture: dict, wav_path: Path) -> None:
    try:
        subprocess.run(
            [
                binary,
                "-v",
                fixture["voice"],
                "-s",
                str(fixture["speed_wpm"]),
                "-w",
                str(wav_path),
                fixture["text"],
            ],
            check=True,
            capture_output=True,
            timeout=30,
        )
    except Exception as exc:
        raise CorpusGenerationError(
            f"speech generation failed for {fixture['id']}"
        ) from exc


def read_pcm16_mono(wav_path: Path) -> tuple[int, list[int]]:
    try:
        with wave.open(str(wav_path), "rb") as source:
            if source.getsampwidth() != 2:
                raise CorpusGenerationError("generated speech must be PCM16")
            channels = source.getnchannels()
            sample_rate = source.getframerate()
            samples = array.array("h")
            samples.frombytes(source.readframes(source.getnframes()))
    except CorpusGenerationError:
        raise
    except Exception as exc:
        raise CorpusGenerationError("failed to read generated speech") from exc

    if sys.byteorder != "little":
        samples.byteswap()
    values = list(samples)
    if channels == 2:
        values = [
            int((values[index] + values[index + 1]) / 2)
            for index in range(0, len(values) - 1, 2)
        ]
    elif channels != 1:
        raise CorpusGenerationError("generated speech must be mono or stereo")
    return sample_rate, values


def resample_linear(
    samples: list[int], source_rate: int, target_rate: int
) -> list[float]:
    if source_rate == target_rate:
        return [float(value) for value in samples]
    if not samples:
        return []
    output_count = max(1, round(len(samples) * target_rate / source_rate))
    ratio = source_rate / target_rate
    output: list[float] = []
    for index in range(output_count):
        position = index * ratio
        left = min(int(position), len(samples) - 1)
        right = min(left + 1, len(samples) - 1)
        fraction = position - left
        output.append(
            samples[left] * (1.0 - fraction) + samples[right] * fraction
        )
    return output


def transform(fixture: dict, samples: list[float], sample_rate: int) -> list[int]:
    gain = math.pow(10.0, fixture["gain_db"] / 20.0) * fixture["distance_scale"]
    seed = int.from_bytes(
        hashlib.sha256(fixture["id"].encode("utf-8")).digest()[:8], "big"
    )
    rng = random.Random(seed)
    noise = fixture["noise_amplitude"] * 32767.0
    output: list[int] = []
    for sample in samples:
        value = sample * gain
        if noise:
            value += rng.uniform(-noise, noise)
        output.append(max(-32768, min(32767, round(value))))
    output.extend([0] * (sample_rate // 2))
    return output


def write_pcm(path: Path, samples: list[int]) -> tuple[int, str]:
    data = b"".join(struct.pack("<h", sample) for sample in samples)
    path.write_bytes(data)
    return len(data), hashlib.sha256(data).hexdigest()


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--output-dir", type=Path, required=True)
    parser.add_argument("--generator", default="espeak-ng")
    args = parser.parse_args()

    try:
        manifest = load_manifest()
        binary = shutil.which(args.generator)
        if binary is None:
            raise CorpusGenerationError(
                "required deterministic speech generator is unavailable"
            )
        version = tool_version(binary)
        args.output_dir.mkdir(parents=True, exist_ok=True)
        generated = []
        with tempfile.TemporaryDirectory(prefix="wake-corpus-") as temp_dir:
            temp = Path(temp_dir)
            for fixture in manifest["fixtures"]:
                wav_path = temp / f"{fixture['id']}.wav"
                synthesize(binary, fixture, wav_path)
                source_rate, raw = read_pcm16_mono(wav_path)
                resampled = resample_linear(
                    raw, source_rate, manifest["policy"]["sample_rate_hz"]
                )
                samples = transform(
                    fixture, resampled, manifest["policy"]["sample_rate_hz"]
                )
                relative = fixture["output"]
                destination = args.output_dir / relative
                destination.parent.mkdir(parents=True, exist_ok=True)
                byte_count, digest = write_pcm(destination, samples)
                generated.append(
                    {
                        "id": fixture["id"],
                        "label": fixture["label"],
                        "path": relative,
                        "expected_detection": fixture["expected_detection"],
                        "bytes": byte_count,
                        "sha256": digest,
                    }
                )

        index = {
            "schema_version": 1,
            "corpus_id": manifest["corpus_id"],
            "generator": args.generator,
            "generator_version": version,
            "acceptance_criteria": manifest["acceptance_criteria"],
            "fixtures": generated,
        }
        (args.output_dir / "corpus-index.json").write_text(
            json.dumps(index, indent=2, sort_keys=True) + "\n",
            encoding="utf-8",
        )
        print(
            f"generated {len(generated)} Wake Word fixtures at {args.output_dir}"
        )
        return 0
    except CorpusGenerationError as exc:
        print(f"wake-word corpus generation error: {exc}", file=sys.stderr)
        return 2


if __name__ == "__main__":
    raise SystemExit(main())
