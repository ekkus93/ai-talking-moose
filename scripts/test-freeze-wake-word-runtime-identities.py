#!/usr/bin/env python3
"""Offline tests for deterministic Wake Word V1 runtime identity freezing."""
from __future__ import annotations

import importlib.util
import io
import pathlib
import struct
import tarfile
import tempfile

SCRIPT = pathlib.Path(__file__).with_name("freeze-wake-word-runtime-identities.py")
spec = importlib.util.spec_from_file_location("freeze_wake_runtime", SCRIPT)
assert spec and spec.loader
module = importlib.util.module_from_spec(spec)
spec.loader.exec_module(module)


def binary_payload(architecture: str, marker: bytes) -> bytes:
    data = bytearray(64)
    if architecture == "elf-x86_64":
        data[:4] = module.ELF_MAGIC
        data[4] = 2
        data[5] = 1
        struct.pack_into("<H", data, 18, module.ELF_MACHINE_X86_64)
    else:
        data[:4] = module.MACHO64_LE_MAGIC
        struct.pack_into("<I", data, 4, module.MACHO_CPU_TYPE_ARM64)
    data[-len(marker):] = marker
    return bytes(data)


def write_archive(path: pathlib.Path, platform: str, wrong_arch: bool = False) -> None:
    config = module.PLATFORMS[platform]
    architecture = str(config["architecture"])
    if wrong_arch:
        architecture = "macho-arm64" if architecture == "elf-x86_64" else "elf-x86_64"
    root = path.name.removesuffix(".tar.bz2")
    with tarfile.open(path, "w:bz2") as bundle:
        for index, library in enumerate(config["libraries"], start=1):
            payload = binary_payload(architecture, bytes([index]))
            info = tarfile.TarInfo(f"{root}/lib/{library}")
            info.size = len(payload)
            bundle.addfile(info, io.BytesIO(payload))


def main() -> None:
    with tempfile.TemporaryDirectory() as temporary:
        root = pathlib.Path(temporary)
        for platform, config in module.PLATFORMS.items():
            archive = root / str(config["archive_name"])
            write_archive(archive, platform)
            first = module.freeze(archive, platform)
            second = module.freeze(archive, platform)
            assert first == second
            assert first["runtime_version"] == module.RUNTIME_VERSION
            assert first["archive"]["size_bytes"] == archive.stat().st_size
            assert len(first["archive"]["sha256"]) == 64
            assert len(first["artifacts"]) == len(config["libraries"])
            assert all(item["architecture"] == config["architecture"] for item in first["artifacts"])

            bad_dir = root / f"bad-{platform}"
            bad_dir.mkdir()
            bad = bad_dir / str(config["archive_name"])
            write_archive(bad, platform, wrong_arch=True)
            try:
                module.freeze(bad, platform)
                raise AssertionError("wrong-architecture runtime was accepted")
            except ValueError as error:
                assert "wrong architecture" in str(error)

    print("wake runtime identity freezer tests passed")


if __name__ == "__main__":
    main()
