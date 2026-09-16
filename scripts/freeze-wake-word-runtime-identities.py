#!/usr/bin/env python3
"""Derive immutable identities from pinned Wake Word V1 sherpa runtime archives.

This helper never downloads anything. It accepts independently obtained upstream
sherpa-onnx v1.13.8 shared-runtime archives for the two V1 platforms, verifies the
expected platform binary formats, and emits archive plus native-library byte sizes
and SHA-256 values suitable for the fail-closed wake-word artifact manifest.
"""
from __future__ import annotations

import argparse
import hashlib
import json
import pathlib
import struct
import tarfile

RUNTIME_VERSION = "v1.13.8"
PLATFORMS = {
    "linux-x86_64": {
        "archive_name": f"sherpa-onnx-{RUNTIME_VERSION}-linux-x64-shared.tar.bz2",
        "architecture": "elf-x86_64",
        "libraries": ("libsherpa-onnx-c-api.so", "libonnxruntime.so"),
    },
    "macos-arm64": {
        "archive_name": f"sherpa-onnx-{RUNTIME_VERSION}-osx-arm64-shared.tar.bz2",
        "architecture": "macho-arm64",
        "libraries": ("libsherpa-onnx-c-api.dylib", "libonnxruntime.dylib"),
    },
}
ELF_MAGIC = b"\x7fELF"
MACHO64_LE_MAGIC = b"\xcf\xfa\xed\xfe"
ELF_MACHINE_X86_64 = 62
MACHO_CPU_TYPE_ARM64 = 0x0100000C


def digest_bytes(data: bytes) -> str:
    return hashlib.sha256(data).hexdigest()


def digest_file(path: pathlib.Path) -> str:
    digest = hashlib.sha256()
    with path.open("rb") as stream:
        for chunk in iter(lambda: stream.read(1024 * 1024), b""):
            digest.update(chunk)
    return digest.hexdigest()


def architecture_matches(data: bytes, architecture: str) -> bool:
    if architecture == "elf-x86_64":
        return (
            len(data) >= 20
            and data[:4] == ELF_MAGIC
            and data[4] == 2
            and data[5] == 1
            and struct.unpack_from("<H", data, 18)[0] == ELF_MACHINE_X86_64
        )
    if architecture == "macho-arm64":
        return (
            len(data) >= 8
            and data[:4] == MACHO64_LE_MAGIC
            and struct.unpack_from("<I", data, 4)[0] == MACHO_CPU_TYPE_ARM64
        )
    return False


def freeze(archive: pathlib.Path, platform: str) -> dict[str, object]:
    config = PLATFORMS.get(platform)
    if config is None:
        raise ValueError("unsupported runtime platform")
    if not archive.is_file():
        raise ValueError("runtime archive is missing")
    if archive.name != config["archive_name"]:
        raise ValueError("unexpected runtime archive filename")

    identities: list[dict[str, object]] = []
    with tarfile.open(archive, "r:bz2") as bundle:
        members = [member for member in bundle.getmembers() if member.isfile()]
        for library in config["libraries"]:
            matches = [member for member in members if pathlib.PurePosixPath(member.name).name == library]
            if len(matches) != 1:
                raise ValueError(f"required runtime library must occur exactly once: {library}")
            member = matches[0]
            stream = bundle.extractfile(member)
            if stream is None:
                raise ValueError(f"required runtime library is unreadable: {library}")
            data = stream.read()
            if len(data) != member.size or not data:
                raise ValueError(f"required runtime library has invalid size: {library}")
            if not architecture_matches(data, str(config["architecture"])):
                raise ValueError(f"required runtime library has wrong architecture: {library}")
            identities.append({
                "id": f"sherpa-runtime-{platform}-{library}",
                "path": member.name,
                "platform": platform,
                "architecture": config["architecture"],
                "size_bytes": len(data),
                "sha256": digest_bytes(data),
            })

    return {
        "runtime_version": RUNTIME_VERSION,
        "platform": platform,
        "archive": {
            "id": f"sherpa-runtime-{platform}",
            "filename": archive.name,
            "archive_type": "tar.bz2",
            "platform": platform,
            "size_bytes": archive.stat().st_size,
            "sha256": digest_file(archive),
        },
        "artifacts": identities,
    }


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--platform", required=True, choices=sorted(PLATFORMS))
    parser.add_argument("archive", type=pathlib.Path)
    args = parser.parse_args()
    try:
        print(json.dumps(freeze(args.archive, args.platform), indent=2, sort_keys=True))
    except (OSError, tarfile.TarError, ValueError) as error:
        parser.error(str(error))
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
