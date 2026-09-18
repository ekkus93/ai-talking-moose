#!/usr/bin/env python3
"""Offline deterministic tests for Wake Word V1 artifact preparation helpers."""
from __future__ import annotations

import importlib.util
import pathlib
import tarfile
import tempfile
import zipfile

SCRIPT = pathlib.Path(__file__).with_name("prepare-wake-word-artifacts.py")
spec = importlib.util.spec_from_file_location("prepare_wake", SCRIPT)
assert spec and spec.loader
module = importlib.util.module_from_spec(spec)
spec.loader.exec_module(module)


def main() -> None:
    with tempfile.TemporaryDirectory() as temporary:
        root = pathlib.Path(temporary)
        payload = root / "payload.bin"
        payload.write_bytes(b"frozen-wake-artifact")
        identity = {
            "size_bytes": payload.stat().st_size,
            "sha256": module.sha256(payload),
        }
        module.verify_identity(payload, identity)
        try:
            module.verify_identity(payload, {**identity, "size_bytes": identity["size_bytes"] + 1})
            raise AssertionError("size mismatch was accepted")
        except ValueError as error:
            assert "byte-size mismatch" in str(error)
        try:
            module.verify_identity(payload, {**identity, "sha256": "0" * 64})
            raise AssertionError("SHA mismatch was accepted")
        except ValueError as error:
            assert "SHA-256 mismatch" in str(error)

        source = root / "source"
        source.mkdir()
        (source / "model.bin").write_bytes(b"model")
        tar_path = root / "fixture.tar.bz2"
        with tarfile.open(tar_path, "w:bz2") as bundle:
            bundle.add(source / "model.bin", arcname="model/model.bin")
        extracted = root / "tar-output"
        module.extract_archive(tar_path, extracted, "tar.bz2")
        assert (extracted / "model/model.bin").read_bytes() == b"model"

        zip_path = root / "fixture.zip"
        with zipfile.ZipFile(zip_path, "w") as bundle:
            bundle.writestr("runtime/lib.bin", b"runtime")
        extracted = root / "zip-output"
        module.extract_archive(zip_path, extracted, "zip")
        assert (extracted / "runtime/lib.bin").read_bytes() == b"runtime"

        for bad in ("../escape", "/absolute"):
            try:
                module.safe_destination(root, bad)
                raise AssertionError("unsafe archive path was accepted")
            except ValueError:
                pass

        malicious_tar = root / "malicious.tar.bz2"
        evil = root / "evil.bin"
        evil.write_bytes(b"evil")
        with tarfile.open(malicious_tar, "w:bz2") as bundle:
            bundle.add(evil, arcname="../escape.bin")
        try:
            module.extract_archive(malicious_tar, root / "malicious-tar-output", "tar.bz2")
            raise AssertionError("tar traversal member was extracted")
        except ValueError:
            pass

        malicious_zip = root / "malicious.zip"
        with zipfile.ZipFile(malicious_zip, "w") as bundle:
            bundle.writestr("../escape.bin", b"evil")
        try:
            module.extract_archive(malicious_zip, root / "malicious-zip-output", "zip")
            raise AssertionError("zip traversal member was extracted")
        except ValueError:
            pass

    print("wake artifact preparation tests passed")


if __name__ == "__main__":
    main()
