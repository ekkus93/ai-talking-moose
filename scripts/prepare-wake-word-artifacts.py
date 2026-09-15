#!/usr/bin/env python3
"""Prepare Wake Word V1 artifacts from a frozen manifest.

Downloads are allowed only from manifest URLs. Every downloaded archive is checked for
exact size and SHA-256 before extraction; extracted files are then verified by the
existing fail-closed verifier. The manifest therefore remains the sole production
identity boundary, independent of HTTP caches or mutable upstream state.
"""
from __future__ import annotations

import argparse
import hashlib
import json
import pathlib
import shutil
import subprocess
import sys
import tarfile
import tempfile
import urllib.request
import zipfile


def sha256(path: pathlib.Path) -> str:
    digest = hashlib.sha256()
    with path.open("rb") as stream:
        for chunk in iter(lambda: stream.read(1024 * 1024), b""):
            digest.update(chunk)
    return digest.hexdigest()


def safe_destination(root: pathlib.Path, relative: str) -> pathlib.Path:
    rel = pathlib.PurePosixPath(relative)
    if rel.is_absolute() or ".." in rel.parts:
        raise ValueError("invalid relative path")
    return root.joinpath(*rel.parts)


def verify_identity(path: pathlib.Path, item: dict[str, object]) -> None:
    expected_size = item.get("size_bytes")
    expected_sha = item.get("sha256")
    if not isinstance(expected_size, int) or expected_size <= 0:
        raise ValueError("artifact byte size is not frozen")
    if not isinstance(expected_sha, str) or len(expected_sha) != 64:
        raise ValueError("artifact SHA-256 is not frozen")
    if path.stat().st_size != expected_size:
        raise ValueError("artifact byte-size mismatch")
    if sha256(path) != expected_sha.lower():
        raise ValueError("artifact SHA-256 mismatch")


def extract_archive(archive: pathlib.Path, destination: pathlib.Path, archive_type: str) -> None:
    destination.mkdir(parents=True, exist_ok=True)
    if archive_type == "tar.bz2":
        with tarfile.open(archive, "r:bz2") as bundle:
            members = bundle.getmembers()
            for member in members:
                safe_destination(destination, member.name)
            bundle.extractall(destination, members=members, filter="data")
    elif archive_type == "zip":
        with zipfile.ZipFile(archive) as bundle:
            for name in bundle.namelist():
                safe_destination(destination, name)
            bundle.extractall(destination)
    else:
        raise ValueError(f"unsupported archive type: {archive_type}")


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--manifest", default="wake-word-artifacts.json")
    parser.add_argument("--root", required=True)
    parser.add_argument("--platform", required=True)
    parser.add_argument("--download-dir")
    args = parser.parse_args()

    manifest_path = pathlib.Path(args.manifest)
    root = pathlib.Path(args.root)
    manifest = json.loads(manifest_path.read_text(encoding="utf-8"))
    if manifest.get("schema_version") != 1:
        print("wake artifact preparation failed: unsupported manifest schema", file=sys.stderr)
        return 2

    archives = [a for a in manifest.get("archives", []) if a.get("platform") in ("all", args.platform)]
    if not archives:
        print("wake artifact preparation failed: unsupported platform", file=sys.stderr)
        return 2

    try:
        with tempfile.TemporaryDirectory() as temporary:
            downloads = pathlib.Path(args.download_dir) if args.download_dir else pathlib.Path(temporary)
            downloads.mkdir(parents=True, exist_ok=True)
            for item in archives:
                url = item.get("url")
                filename = item.get("filename")
                if not isinstance(url, str) or not url.startswith("https://"):
                    raise ValueError("archive URL must be frozen HTTPS")
                if not isinstance(filename, str):
                    raise ValueError("archive filename is not frozen")
                archive = safe_destination(downloads, filename)
                if not archive.exists():
                    with urllib.request.urlopen(url) as response, archive.open("wb") as output:
                        shutil.copyfileobj(response, output)
                verify_identity(archive, item)
                extract_archive(archive, root, str(item.get("archive_type")))

        verifier = pathlib.Path(__file__).with_name("verify-wake-word-artifacts.py")
        completed = subprocess.run(
            [str(verifier), "--manifest", str(manifest_path), "--root", str(root), "--platform", args.platform],
            check=False,
        )
        return completed.returncode
    except (OSError, ValueError, tarfile.TarError, zipfile.BadZipFile) as error:
        print(f"wake artifact preparation failed: {error}", file=sys.stderr)
        return 1


if __name__ == "__main__":
    raise SystemExit(main())
