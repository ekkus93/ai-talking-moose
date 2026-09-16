#!/usr/bin/env python3
"""Derive reviewable immutable identities from the selected Wake Word V1 model archive.

This helper never downloads anything. It accepts an independently obtained upstream archive,
requires the exact selected model directory and fp32 production input set, and emits the byte
sizes/SHA-256 values needed to populate the fail-closed production manifests.
"""
from __future__ import annotations

import argparse
import hashlib
import json
import pathlib
import tarfile

MODEL_ID = "sherpa-onnx-kws-zipformer-gigaspeech-3.3M-2024-01-01"
ARCHIVE_URL = (
    "https://github.com/k2-fsa/sherpa-onnx/releases/download/kws-models/"
    f"{MODEL_ID}.tar.bz2"
)
REQUIRED_FILES = (
    "encoder-epoch-12-avg-2-chunk-16-left-64.onnx",
    "decoder-epoch-12-avg-2-chunk-16-left-64.onnx",
    "joiner-epoch-12-avg-2-chunk-16-left-64.onnx",
    "tokens.txt",
    "bpe.model",
)


def digest_bytes(data: bytes) -> str:
    return hashlib.sha256(data).hexdigest()


def digest_file(path: pathlib.Path) -> str:
    digest = hashlib.sha256()
    with path.open("rb") as stream:
        for chunk in iter(lambda: stream.read(1024 * 1024), b""):
            digest.update(chunk)
    return digest.hexdigest()


def freeze(archive: pathlib.Path) -> dict[str, object]:
    if not archive.is_file():
        raise ValueError("model archive is missing")
    if archive.name != f"{MODEL_ID}.tar.bz2":
        raise ValueError("unexpected model archive filename")

    identities: list[dict[str, object]] = []
    with tarfile.open(archive, "r:bz2") as bundle:
        members = {member.name: member for member in bundle.getmembers() if member.isfile()}
        for filename in REQUIRED_FILES:
            member_name = f"{MODEL_ID}/{filename}"
            member = members.get(member_name)
            if member is None:
                raise ValueError(f"required production model input is missing: {filename}")
            stream = bundle.extractfile(member)
            if stream is None:
                raise ValueError(f"required production model input is unreadable: {filename}")
            data = stream.read()
            if len(data) != member.size or not data:
                raise ValueError(f"required production model input has invalid size: {filename}")
            identities.append(
                {
                    "id": f"model-{filename}",
                    "path": f"{MODEL_ID}/{filename}",
                    "platform": "all",
                    "size_bytes": len(data),
                    "sha256": digest_bytes(data),
                }
            )

    return {
        "model_id": MODEL_ID,
        "archive": {
            "id": "sherpa-kws-model",
            "url": ARCHIVE_URL,
            "filename": archive.name,
            "archive_type": "tar.bz2",
            "platform": "all",
            "size_bytes": archive.stat().st_size,
            "sha256": digest_file(archive),
        },
        "artifacts": identities,
    }


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("archive", type=pathlib.Path)
    args = parser.parse_args()
    try:
        print(json.dumps(freeze(args.archive), indent=2, sort_keys=True))
    except (OSError, tarfile.TarError, ValueError) as error:
        parser.error(str(error))
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
