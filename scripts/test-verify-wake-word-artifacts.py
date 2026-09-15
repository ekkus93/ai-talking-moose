#!/usr/bin/env python3
"""Deterministic tests for the fail-closed Wake Word V1 artifact verifier."""
from __future__ import annotations

import hashlib
import json
import pathlib
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


def write_manifest(path: pathlib.Path, payload: bytes) -> None:
    path.write_text(json.dumps({
        "schema_version": 1,
        "artifacts": [{
            "id": "fixture",
            "platform": "all",
            "path": "model.bin",
            "size_bytes": len(payload),
            "sha256": hashlib.sha256(payload).hexdigest(),
        }],
    }), encoding="utf-8")


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

    print("wake artifact verifier tests passed")


if __name__ == "__main__":
    main()
