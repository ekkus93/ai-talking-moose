#!/usr/bin/env python3
"""Acquire and verify the frozen Wake Word V1 sherpa artifacts.

This script intentionally accepts no mutable model revision. Every production input
is defined in src-tauri/resources/wake_word/artifacts-v1.json and is accepted only
after byte-size and SHA-256 verification.
"""
from __future__ import annotations

import argparse
import hashlib
import json
from pathlib import Path
import shutil
import tarfile
import tempfile
import urllib.request

ROOT = Path(__file__).resolve().parents[1]
MANIFEST_PATH = ROOT / "src-tauri/resources/wake_word/artifacts-v1.json"


def sha256(path: Path) -> str:
    digest = hashlib.sha256()
    with path.open("rb") as handle:
        for chunk in iter(lambda: handle.read(1024 * 1024), b""):
            digest.update(chunk)
    return digest.hexdigest()


def verify(path: Path, expected_bytes: int, expected_sha: str) -> None:
    size = path.stat().st_size
    if size != expected_bytes:
        raise SystemExit(
            f"artifact size mismatch for {path.name}: expected {expected_bytes}, got {size}"
        )
    actual_sha = sha256(path)
    if actual_sha != expected_sha:
        raise SystemExit(f"artifact digest mismatch for {path.name}")


def download(url: str, target: Path) -> None:
    target.parent.mkdir(parents=True, exist_ok=True)
    with urllib.request.urlopen(url, timeout=120) as response, target.open("wb") as output:
        shutil.copyfileobj(response, output)


def safe_extract_tar_bz2(archive: Path, destination: Path) -> None:
    destination.mkdir(parents=True, exist_ok=True)
    root = destination.resolve()
    with tarfile.open(archive, "r:bz2") as bundle:
        for member in bundle.getmembers():
            resolved = (destination / member.name).resolve()
            if root != resolved and root not in resolved.parents:
                raise SystemExit("runtime archive contains an unsafe path")
        bundle.extractall(destination)


def main() -> None:
    parser = argparse.ArgumentParser()
    parser.add_argument("--destination", type=Path, required=True)
    parser.add_argument(
        "--platform",
        choices=["linux-x86_64", "macos-arm64", "macos-x86_64"],
        required=True,
    )
    args = parser.parse_args()

    manifest = json.loads(MANIFEST_PATH.read_text())
    destination = args.destination.resolve()
    model_root = destination / "model"
    runtime_root = destination / "runtime"
    model_root.mkdir(parents=True, exist_ok=True)

    revision = manifest["model_revision"]
    base = f"https://huggingface.co/{manifest['model_repository']}/resolve/{revision}"
    for artifact in manifest["model_artifacts"]:
        relative = Path(artifact["path"])
        target = model_root / relative
        if not target.exists():
            download(f"{base}/{artifact['path']}", target)
        verify(target, artifact["bytes"], artifact["sha256"])

    (model_root / "keywords.txt").write_text(manifest["keyword_tokens"] + "\n")

    runtime = manifest["runtime_archives"][args.platform]
    release_url = (
        "https://github.com/k2-fsa/sherpa-onnx/releases/download/"
        f"v{manifest['sherpa_version']}/{runtime['filename']}"
    )
    with tempfile.TemporaryDirectory(prefix="wake-runtime-") as temporary:
        archive = Path(temporary) / runtime["filename"]
        download(release_url, archive)
        verify(archive, runtime["bytes"], runtime["sha256"])
        if runtime_root.exists():
            shutil.rmtree(runtime_root)
        safe_extract_tar_bz2(archive, runtime_root)

    evidence = {
        "engine": manifest["engine"],
        "sherpa_version": manifest["sherpa_version"],
        "model_revision": revision,
        "platform": args.platform,
        "model_verified": True,
        "runtime_archive_verified": True,
    }
    (destination / "verification.json").write_text(
        json.dumps(evidence, indent=2) + "\n"
    )
    print(json.dumps(evidence, sort_keys=True))


if __name__ == "__main__":
    main()
