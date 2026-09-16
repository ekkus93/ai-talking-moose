#!/usr/bin/env python3
"""Offline tests for deterministic Wake Word V1 model identity freezing."""
from __future__ import annotations

import importlib.util
import io
import pathlib
import tarfile
import tempfile

SCRIPT = pathlib.Path(__file__).with_name("freeze-wake-word-model-identities.py")
spec = importlib.util.spec_from_file_location("freeze_wake_model", SCRIPT)
assert spec and spec.loader
module = importlib.util.module_from_spec(spec)
spec.loader.exec_module(module)


def write_archive(path: pathlib.Path, omit: str | None = None) -> None:
    with tarfile.open(path, "w:bz2") as bundle:
        for index, filename in enumerate(module.REQUIRED_FILES, start=1):
            if filename == omit:
                continue
            payload = (filename.encode("utf-8") + b"\n") * index
            info = tarfile.TarInfo(f"{module.MODEL_ID}/{filename}")
            info.size = len(payload)
            bundle.addfile(info, io.BytesIO(payload))


def main() -> None:
    with tempfile.TemporaryDirectory() as temporary:
        root = pathlib.Path(temporary)
        archive = root / f"{module.MODEL_ID}.tar.bz2"
        write_archive(archive)
        first = module.freeze(archive)
        second = module.freeze(archive)
        assert first == second
        assert first["archive"]["size_bytes"] == archive.stat().st_size
        assert len(first["archive"]["sha256"]) == 64
        assert [pathlib.PurePosixPath(item["path"]).name for item in first["artifacts"]] == list(
            module.REQUIRED_FILES
        )
        assert all(item["size_bytes"] > 0 and len(item["sha256"]) == 64 for item in first["artifacts"])

        missing = root / "missing.tar.bz2"
        try:
            module.freeze(missing)
            raise AssertionError("missing archive was accepted")
        except ValueError:
            pass

        incomplete_dir = root / "incomplete"
        incomplete_dir.mkdir()
        incomplete = incomplete_dir / f"{module.MODEL_ID}.tar.bz2"
        write_archive(incomplete, omit=module.REQUIRED_FILES[-1])
        try:
            module.freeze(incomplete)
            raise AssertionError("incomplete production model set was accepted")
        except ValueError as error:
            assert module.REQUIRED_FILES[-1] in str(error)

    print("wake model identity freezer tests passed")


if __name__ == "__main__":
    main()
