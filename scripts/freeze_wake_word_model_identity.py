#!/usr/bin/env python3
"""Freeze immutable identities for the selected Wake Word V1 model archive.

The tool accepts an already-downloaded archive by default so identity freezing can
be performed in a controlled CI job. Optional --url download is explicit. Archive
members are validated before extraction and only the five production-consumed
model/tokenizer files are emitted.
"""
from __future__ import annotations

import argparse
import bz2
import hashlib
import json
import shutil
import tarfile
import tempfile
import urllib.request
from pathlib import Path, PurePosixPath

MODEL_ID = "sherpa-onnx-kws-zipformer-gigaspeech-3.3M-2024-01-01"
ARCHIVE_NAME = f"{MODEL_ID}.tar.bz2"
UPSTREAM_URL = (
    "https://github.com/k2-fsa/sherpa-onnx/releases/download/kws-models/"
    + ARCHIVE_NAME
)
REQUIRED_FILES = (
    "encoder-epoch-12-avg-2-chunk-16-left-64.onnx",
    "decoder-epoch-12-avg-2-chunk-16-left-64.onnx",
    "joiner-epoch-12-avg-2-chunk-16-left-64.onnx",
    "tokens.txt",
    "bpe.model",
)


def identity(path: Path) -> dict[str, object]:
    digest = hashlib.sha256()
    size = 0
    with path.open("rb") as handle:
        for chunk in iter(lambda: handle.read(1024 * 1024), b""):
            size += len(chunk)
            digest.update(chunk)
    return {"bytes": size, "sha256": digest.hexdigest()}


def _safe_member_name(name: str) -> PurePosixPath:
    member = PurePosixPath(name)
    if member.is_absolute() or ".." in member.parts:
        raise ValueError(f"unsafe archive member: {name!r}")
    if not member.parts or member.parts[0] != MODEL_ID:
        raise ValueError(f"unexpected archive root: {name!r}")
    return member


def freeze(archive: Path) -> dict[str, object]:
    archive_identity = identity(archive)
    with tempfile.TemporaryDirectory(prefix="wake-word-model-") as temporary:
        root = Path(temporary)
        with tarfile.open(archive, "r:bz2") as bundle:
            members = bundle.getmembers()
            for member in members:
                _safe_member_name(member.name)
                if member.issym() or member.islnk() or member.isdev():
                    raise ValueError(f"unsupported archive member type: {member.name!r}")
            bundle.extractall(root, members=members, filter="data")
        model_root = root / MODEL_ID
        files: dict[str, dict[str, object]] = {}
        for name in REQUIRED_FILES:
            path = model_root / name
            if not path.is_file():
                raise ValueError(f"required model file missing: {name}")
            files[name] = identity(path)
    return {
        "schema_version": 1,
        "model_id": MODEL_ID,
        "source_url": UPSTREAM_URL,
        "archive": {"name": ARCHIVE_NAME, **archive_identity},
        "files": files,
    }


def download(url: str, destination: Path) -> None:
    request = urllib.request.Request(url, headers={"User-Agent": "ai-talking-moose-identity-freezer/1"})
    with urllib.request.urlopen(request, timeout=120) as response, destination.open("wb") as output:
        shutil.copyfileobj(response, output)


def main() -> int:
    parser = argparse.ArgumentParser()
    source = parser.add_mutually_exclusive_group(required=True)
    source.add_argument("--archive", type=Path)
    source.add_argument("--url", default=None, nargs="?", const=UPSTREAM_URL)
    parser.add_argument("--output", type=Path)
    args = parser.parse_args()

    if args.archive is not None:
        result = freeze(args.archive)
    else:
        with tempfile.TemporaryDirectory(prefix="wake-word-download-") as temporary:
            archive = Path(temporary) / ARCHIVE_NAME
            download(args.url, archive)
            result = freeze(archive)

    rendered = json.dumps(result, indent=2, sort_keys=True) + "\n"
    if args.output:
        args.output.write_text(rendered, encoding="utf-8")
    else:
        print(rendered, end="")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
