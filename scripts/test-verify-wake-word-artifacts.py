#!/usr/bin/env python3
"""Deterministic tests for the fail-closed Wake Word V1 artifact verifier."""
from __future__ import annotations

import hashlib
import json
import pathlib
import struct
import subprocess
import tempfile

SCRIPT = pathlib.Path(__file__).with_name("verify-wake-word-artifacts.py")


def run(root: pathlib.Path, manifest: pathlib.Path, platform: str = "linux-x86_64") -> subprocess.CompletedProcess[str]:
    return subprocess.run(
        [str(SCRIPT), "--manifest", str(manifest), "--root", str(root), "--platform", platform],
        check=False,
        text=True,
        capture_output=True,
    )


def write_manifest(
    path: pathlib.Path,
    payload: bytes,
    *,
    artifact_path: str = "model.bin",
    platform: str = "all",
    architecture: str | None = None,
) -> None:
    artifact: dict[str, object] = {
        "id": "fixture",
        "platform": platform,
        "path": artifact_path,
        "size_bytes": len(payload),
        "sha256": hashlib.sha256(payload).hexdigest(),
    }
    if architecture is not None:
        artifact["architecture"] = architecture
    path.write_text(
        json.dumps({"schema_version": 1, "artifacts": [artifact]}),
        encoding="utf-8",
    )


def elf_x86_64_fixture() -> bytes:
    header = bytearray(32)
    header[:4] = b"\x7fELF"
    header[4] = 2
    header[5] = 1
    struct.pack_into("<H", header, 18, 62)
    return bytes(header) + b"synthetic-linux-runtime"


def macho_arm64_fixture() -> bytes:
    header = bytearray(32)
    header[:4] = b"\xcf\xfa\xed\xfe"
    struct.pack_into("<I", header, 4, 0x0100000C)
    return bytes(header) + b"synthetic-macos-runtime"


def main() -> None:
    with tempfile.TemporaryDirectory() as temp:
        root = pathlib.Path(temp)
        payload = b"wake-word-fixture"
        artifact = root / "model.bin"
        manifest = root / "manifest.json"
        artifact.write_bytes(payload)
        write_manifest(manifest, payload)
        assert run(root, manifest).returncode == 0

        artifact.write_bytes(b"corrupt")
        failure = run(root, manifest)
        assert failure.returncode != 0
        assert "mismatch" in failure.stderr

        artifact.unlink()
        failure = run(root, manifest)
        assert failure.returncode != 0
        assert "missing" in failure.stderr

        # Restore the artifact so this probe exercises manifest freezing rather
        # than being short-circuited by the earlier missing-file condition.
        artifact.write_bytes(payload)
        unfrozen = json.loads(manifest.read_text(encoding="utf-8"))
        unfrozen["artifacts"][0]["size_bytes"] = None
        manifest.write_text(json.dumps(unfrozen), encoding="utf-8")
        failure = run(root, manifest)
        assert failure.returncode != 0
        assert "not frozen" in failure.stderr

        unsupported = root / "unsupported.json"
        unsupported.write_text(json.dumps({"schema_version": 1, "artifacts": [{
            "id": "mac-only", "platform": "macos-arm64", "path": "x", "size_bytes": 1,
            "sha256": "0" * 64,
        }]}), encoding="utf-8")
        failure = run(root, unsupported)
        assert failure.returncode != 0
        assert "unsupported platform" in failure.stderr

        linux_runtime = root / "runtime-linux.so"
        linux_payload = elf_x86_64_fixture()
        linux_runtime.write_bytes(linux_payload)
        write_manifest(
            manifest,
            linux_payload,
            artifact_path=linux_runtime.name,
            platform="linux-x86_64",
            architecture="elf-x86_64",
        )
        assert run(root, manifest).returncode == 0

        wrong_linux = bytearray(linux_payload)
        struct.pack_into("<H", wrong_linux, 18, 183)  # AArch64
        linux_runtime.write_bytes(wrong_linux)
        write_manifest(
            manifest,
            bytes(wrong_linux),
            artifact_path=linux_runtime.name,
            platform="linux-x86_64",
            architecture="elf-x86_64",
        )
        failure = run(root, manifest)
        assert failure.returncode != 0
        assert "wrong architecture or binary format" in failure.stderr
        assert str(root) not in failure.stderr

        mac_runtime = root / "runtime-macos.dylib"
        mac_payload = macho_arm64_fixture()
        mac_runtime.write_bytes(mac_payload)
        write_manifest(
            manifest,
            mac_payload,
            artifact_path=mac_runtime.name,
            platform="macos-arm64",
            architecture="macho-arm64",
        )
        assert run(root, manifest, "macos-arm64").returncode == 0

        wrong_mac = bytearray(mac_payload)
        struct.pack_into("<I", wrong_mac, 4, 0x01000007)  # x86_64
        mac_runtime.write_bytes(wrong_mac)
        write_manifest(
            manifest,
            bytes(wrong_mac),
            artifact_path=mac_runtime.name,
            platform="macos-arm64",
            architecture="macho-arm64",
        )
        failure = run(root, manifest, "macos-arm64")
        assert failure.returncode != 0
        assert "wrong architecture or binary format" in failure.stderr
        assert str(root) not in failure.stderr

    print("wake artifact verifier tests passed")


if __name__ == "__main__":
    main()
