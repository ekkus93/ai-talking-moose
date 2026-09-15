#!/usr/bin/env python3
"""Fail-closed verifier for Wake Word V1 production artifacts.

The manifest is intentionally data-only. Downloads/extraction are separate so a cache or
mutable upstream URL can never substitute for identity verification.
"""
from __future__ import annotations

import argparse
import hashlib
import json
import pathlib
import sys


def sha256(path: pathlib.Path) -> str:
    digest = hashlib.sha256()
    with path.open("rb") as stream:
        for chunk in iter(lambda: stream.read(1024 * 1024), b""):
            digest.update(chunk)
    return digest.hexdigest()


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--manifest", default="wake-word-artifacts.json")
    parser.add_argument("--root", required=True)
    parser.add_argument("--platform", required=True)
    args = parser.parse_args()

    manifest_path = pathlib.Path(args.manifest)
    root = pathlib.Path(args.root)
    manifest = json.loads(manifest_path.read_text(encoding="utf-8"))
    if manifest.get("schema_version") != 1:
        print("wake artifact verification failed: unsupported manifest schema", file=sys.stderr)
        return 2

    artifacts = manifest.get("artifacts", [])
    selected = [a for a in artifacts if a.get("platform") in ("all", args.platform)]
    if not selected:
        print("wake artifact verification failed: unsupported platform", file=sys.stderr)
        return 2

    errors: list[str] = []
    for artifact in selected:
        rel = pathlib.PurePosixPath(artifact["path"])
        if rel.is_absolute() or ".." in rel.parts:
            errors.append(f"{artifact['id']}: invalid relative path")
            continue
        path = root.joinpath(*rel.parts)
        if not path.is_file():
            errors.append(f"{artifact['id']}: missing")
            continue
        expected_size = artifact.get("size_bytes")
        expected_sha = artifact.get("sha256")
        if not isinstance(expected_size, int) or expected_size <= 0:
            errors.append(f"{artifact['id']}: manifest size is not frozen")
            continue
        if not isinstance(expected_sha, str) or len(expected_sha) != 64:
            errors.append(f"{artifact['id']}: manifest SHA-256 is not frozen")
            continue
        actual_size = path.stat().st_size
        if actual_size != expected_size:
            errors.append(f"{artifact['id']}: byte-size mismatch")
            continue
        if sha256(path) != expected_sha.lower():
            errors.append(f"{artifact['id']}: SHA-256 mismatch")

    if errors:
        for error in errors:
            print(f"wake artifact verification failed: {error}", file=sys.stderr)
        return 1
    print(f"verified {len(selected)} Wake Word V1 artifacts for {args.platform}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
