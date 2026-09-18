#!/usr/bin/env python3
"""Freeze immutable identities for the selected Wake Word V1 model.

This script deliberately downloads the selected upstream release archive and hashes
both the archive and the exact five files consumed by production. It never trusts
metadata from the archive for identity. Extraction rejects links and traversal.
"""
from __future__ import annotations

import argparse
import hashlib
import json
import tarfile
import tempfile
import urllib.request
from pathlib import Path, PurePosixPath

MODEL_ID = "sherpa-onnx-kws-zipformer-gigaspeech-3.3M-2024-01-01"
ARCHIVE_URL = (
    "https://github.com/k2-fsa/sherpa-onnx/releases/download/kws-models/"
    f"{MODEL_ID}.tar.bz2"
)
FILES = [
    "encoder-epoch-12-avg-2-chunk-16-left-64.onnx",
    "decoder-epoch-12-avg-2-chunk-16-left-64.onnx",
    "joiner-epoch-12-avg-2-chunk-16-left-64.onnx",
    "tokens.txt",
    "bpe.model",
]
KEYWORD_SOURCE = "HEY MOOSE"


def identity(path: Path) -> dict[str, object]:
    digest = hashlib.sha256()
    size = 0
    with path.open("rb") as handle:
        while chunk := handle.read(1024 * 1024):
            size += len(chunk)
            digest.update(chunk)
    return {"bytes": size, "sha256": digest.hexdigest()}


def safe_extract_selected(archive: Path, destination: Path) -> None:
    required = {f"{MODEL_ID}/{name}" for name in FILES}
    found: set[str] = set()
    with tarfile.open(archive, "r:bz2") as bundle:
        for member in bundle.getmembers():
            pure = PurePosixPath(member.name)
            if pure.is_absolute() or ".." in pure.parts:
                raise RuntimeError(f"unsafe archive member: {member.name!r}")
            if member.issym() or member.islnk():
                if member.name in required:
                    raise RuntimeError(f"required file is a link: {member.name!r}")
                continue
            if member.name not in required:
                continue
            if not member.isfile():
                raise RuntimeError(f"required member is not a regular file: {member.name!r}")
            target = destination / pure.name
            source = bundle.extractfile(member)
            if source is None:
                raise RuntimeError(f"unable to read required member: {member.name!r}")
            with target.open("wb") as output:
                while chunk := source.read(1024 * 1024):
                    output.write(chunk)
            found.add(member.name)
    missing = sorted(required - found)
    if missing:
        raise RuntimeError(f"archive missing required files: {missing}")


def keyword_representation(bpe_model: Path) -> str:
    try:
        import sentencepiece as spm
    except ImportError as exc:
        raise RuntimeError("sentencepiece is required to freeze the BPE keyword") from exc
    processor = spm.SentencePieceProcessor(model_file=str(bpe_model))
    pieces = processor.encode(KEYWORD_SOURCE, out_type=str)
    if not pieces:
        raise RuntimeError("keyword tokenization produced no pieces")
    return " ".join(pieces) + "\n"


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--output", type=Path)
    args = parser.parse_args()
    with tempfile.TemporaryDirectory(prefix="wake-model-freeze-") as temp:
        root = Path(temp)
        archive = root / f"{MODEL_ID}.tar.bz2"
        with urllib.request.urlopen(ARCHIVE_URL, timeout=120) as response, archive.open("wb") as out:
            while chunk := response.read(1024 * 1024):
                out.write(chunk)
        extracted = root / "selected"
        extracted.mkdir()
        safe_extract_selected(archive, extracted)
        keyword = keyword_representation(extracted / "bpe.model")
        keyword_path = root / "hey-moose.tokens.txt"
        keyword_path.write_text(keyword, encoding="utf-8", newline="\n")
        result = {
            "schema_version": 1,
            "model_id": MODEL_ID,
            "source_url": ARCHIVE_URL,
            "archive": {"filename": archive.name, **identity(archive)},
            "files": {name: identity(extracted / name) for name in FILES},
            "keyword": {
                "source": KEYWORD_SOURCE,
                "representation": keyword.rstrip("\n"),
                "filename": keyword_path.name,
                **identity(keyword_path),
            },
        }
        rendered = json.dumps(result, indent=2, sort_keys=True) + "\n"
        if args.output:
            args.output.write_text(rendered, encoding="utf-8", newline="\n")
        print("WAKE_WORD_MODEL_IDENTITY=" + json.dumps(result, sort_keys=True, separators=(",", ":")))
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
