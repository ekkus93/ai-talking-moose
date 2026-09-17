#!/usr/bin/env python3
from __future__ import annotations

import hashlib
import importlib.util
import io
import tarfile
import tempfile
import unittest
from pathlib import Path

SCRIPT = Path(__file__).with_name("freeze_wake_word_model_identity.py")
spec = importlib.util.spec_from_file_location("freezer", SCRIPT)
assert spec and spec.loader
freezer = importlib.util.module_from_spec(spec)
spec.loader.exec_module(freezer)


class FreezeWakeWordModelIdentityTests(unittest.TestCase):
    def make_archive(self, *, unsafe: str | None = None, omit: str | None = None) -> Path:
        temporary = tempfile.TemporaryDirectory()
        self.addCleanup(temporary.cleanup)
        archive = Path(temporary.name) / freezer.ARCHIVE_NAME
        with tarfile.open(archive, "w:bz2") as bundle:
            for index, name in enumerate(freezer.REQUIRED_FILES, start=1):
                if name == omit:
                    continue
                payload = (name + "\n").encode() * index
                info = tarfile.TarInfo(f"{freezer.MODEL_ID}/{name}")
                info.size = len(payload)
                bundle.addfile(info, io.BytesIO(payload))
            if unsafe:
                payload = b"escape"
                info = tarfile.TarInfo(unsafe)
                info.size = len(payload)
                bundle.addfile(info, io.BytesIO(payload))
        return archive

    def test_freeze_records_exact_archive_and_required_file_identities(self) -> None:
        archive = self.make_archive()
        result = freezer.freeze(archive)
        self.assertEqual(result["model_id"], freezer.MODEL_ID)
        self.assertEqual(result["source_url"], freezer.UPSTREAM_URL)
        self.assertEqual(set(result["files"]), set(freezer.REQUIRED_FILES))
        self.assertEqual(result["archive"]["bytes"], archive.stat().st_size)
        self.assertEqual(result["archive"]["sha256"], hashlib.sha256(archive.read_bytes()).hexdigest())
        for file_identity in result["files"].values():
            self.assertGreater(file_identity["bytes"], 0)
            self.assertRegex(file_identity["sha256"], r"^[0-9a-f]{64}$")

    def test_path_traversal_is_rejected_before_extraction(self) -> None:
        archive = self.make_archive(unsafe=f"{freezer.MODEL_ID}/../../escape")
        with self.assertRaisesRegex(ValueError, "unsafe archive member"):
            freezer.freeze(archive)

    def test_missing_required_file_fails_closed(self) -> None:
        archive = self.make_archive(omit="bpe.model")
        with self.assertRaisesRegex(ValueError, "required model file missing: bpe.model"):
            freezer.freeze(archive)


if __name__ == "__main__":
    unittest.main()
