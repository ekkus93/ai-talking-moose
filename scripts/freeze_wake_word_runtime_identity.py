#!/usr/bin/env python3
"""Freeze sherpa-onnx V1 native C-API runtime archive and library identities."""
from __future__ import annotations

import hashlib
import json
import struct
import tarfile
import tempfile
import urllib.request
from pathlib import Path, PurePosixPath

VERSION = "1.13.8"
BASE = f"https://github.com/k2-fsa/sherpa-onnx/releases/download/v{VERSION}"
PLATFORMS = {
    "linux-x86_64": {
        "filename": f"sherpa-onnx-v{VERSION}-linux-x64-shared.tar.bz2",
        "architecture": "elf-x86_64",
        "libraries": ("libsherpa-onnx-c-api.so", "libonnxruntime.so"),
    },
    "macos-arm64": {
        "filename": f"sherpa-onnx-v{VERSION}-osx-arm64-shared.tar.bz2",
        "architecture": "macho-arm64",
        "libraries": ("libsherpa-onnx-c-api.dylib", "libonnxruntime.dylib"),
    },
}


def identity_bytes(data: bytes) -> dict[str, object]:
    return {"bytes": len(data), "sha256": hashlib.sha256(data).hexdigest()}


def identity_file(path: Path) -> dict[str, object]:
    digest = hashlib.sha256()
    size = 0
    with path.open("rb") as stream:
        for chunk in iter(lambda: stream.read(1024 * 1024), b""):
            size += len(chunk)
            digest.update(chunk)
    return {"bytes": size, "sha256": digest.hexdigest()}


def architecture(data: bytes) -> str | None:
    if (
        len(data) >= 20
        and data[:4] == b"\x7fELF"
        and data[4:6] == b"\x02\x01"
        and struct.unpack_from("<H", data, 18)[0] == 62
    ):
        return "elf-x86_64"
    if (
        len(data) >= 8
        and data[:4] == b"\xcf\xfa\xed\xfe"
        and struct.unpack_from("<I", data, 4)[0] == 0x0100000C
    ):
        return "macho-arm64"
    return None


def inspect_archive(path: Path, expected_arch: str, required_libraries: tuple[str, ...]) -> list[dict[str, object]]:
    found: dict[str, dict[str, object]] = {}
    with tarfile.open(path, "r:bz2") as bundle:
        for member in bundle.getmembers():
            pure = PurePosixPath(member.name)
            if pure.is_absolute() or ".." in pure.parts:
                raise RuntimeError("unsafe archive member")
            if member.issym() or member.islnk():
                if pure.name in required_libraries:
                    raise RuntimeError("required native library is a link")
                continue
            if not member.isfile() or pure.name not in required_libraries:
                continue
            if pure.name in found:
                raise RuntimeError("required native library occurs more than once")
            source = bundle.extractfile(member)
            if source is None:
                raise RuntimeError("required native library is unreadable")
            data = source.read()
            arch = architecture(data)
            if arch != expected_arch:
                raise RuntimeError("wrong architecture in native member")
            found[pure.name] = {
                "path": member.name,
                "architecture": arch,
                **identity_bytes(data),
            }
    missing = [name for name in required_libraries if name not in found]
    if missing:
        raise RuntimeError("missing required native libraries")
    return [found[name] for name in required_libraries]


def main() -> int:
    result = {
        "schema_version": 1,
        "runtime_version": f"v{VERSION}",
        "abi": "sherpa-onnx-c-api",
        "license": "Apache-2.0",
        "platforms": {},
    }
    with tempfile.TemporaryDirectory(prefix="wake-runtime-freeze-") as td:
        root = Path(td)
        for platform, cfg in PLATFORMS.items():
            filename = cfg["filename"]
            url = f"{BASE}/{filename}"
            path = root / filename
            with urllib.request.urlopen(url, timeout=120) as response, path.open("wb") as out:
                while chunk := response.read(1024 * 1024):
                    out.write(chunk)
            result["platforms"][platform] = {
                "source_url": url,
                "archive": {"filename": filename, **identity_file(path)},
                "install_root": f"runtime/sherpa-onnx/v{VERSION}/{platform}",
                "files": inspect_archive(path, cfg["architecture"], cfg["libraries"]),
            }
    print("WAKE_WORD_RUNTIME_IDENTITY=" + json.dumps(result, sort_keys=True, separators=(",", ":")))
    print(json.dumps(result, indent=2, sort_keys=True))
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
