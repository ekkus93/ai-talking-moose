from __future__ import annotations

import importlib.util
import struct
import tempfile
import unittest
import zipfile
from pathlib import Path

SCRIPT = Path(__file__).resolve().parents[1] / "scripts" / "freeze_wake_word_runtime_identity.py"
SPEC = importlib.util.spec_from_file_location("wake_runtime_freezer", SCRIPT)
module = importlib.util.module_from_spec(SPEC)
assert SPEC.loader is not None
SPEC.loader.exec_module(module)


def elf_x86_64() -> bytes:
    data = bytearray(64)
    data[:4] = b"\x7fELF"
    data[4:6] = b"\x02\x01"
    struct.pack_into("<H", data, 18, 62)
    return bytes(data)


def macho_arm64() -> bytes:
    data = bytearray(64)
    data[:4] = b"\xcf\xfa\xed\xfe"
    struct.pack_into("<I", data, 4, 0x0100000C)
    return bytes(data)


class WakeRuntimeIdentityFreezerTests(unittest.TestCase):
    def test_architecture_headers(self):
        self.assertEqual(module.architecture(elf_x86_64()), "elf-x86_64")
        self.assertEqual(module.architecture(macho_arm64()), "macho-arm64")
        self.assertIsNone(module.architecture(b"not-a-native-library"))

    def test_archive_identity_and_architecture(self):
        with tempfile.TemporaryDirectory() as td:
            archive = Path(td) / "runtime.zip"
            payload = elf_x86_64()
            with zipfile.ZipFile(archive, "w") as bundle:
                bundle.writestr("native/libwake.so", payload)
            files = module.inspect_archive(archive, "elf-x86_64")
            self.assertEqual(len(files), 1)
            self.assertEqual(files[0]["architecture"], "elf-x86_64")
            self.assertEqual(files[0]["bytes"], len(payload))
            self.assertEqual(module.identity_bytes(payload)["sha256"], files[0]["sha256"])

    def test_wrong_architecture_is_rejected(self):
        with tempfile.TemporaryDirectory() as td:
            archive = Path(td) / "runtime.zip"
            with zipfile.ZipFile(archive, "w") as bundle:
                bundle.writestr("native/libwake.dylib", macho_arm64())
            with self.assertRaisesRegex(RuntimeError, "wrong architecture"):
                module.inspect_archive(archive, "elf-x86_64")

    def test_archive_traversal_is_rejected(self):
        with tempfile.TemporaryDirectory() as td:
            archive = Path(td) / "runtime.zip"
            with zipfile.ZipFile(archive, "w") as bundle:
                bundle.writestr("../escape.so", elf_x86_64())
            with self.assertRaisesRegex(RuntimeError, "unsafe archive member"):
                module.inspect_archive(archive, "elf-x86_64")


if __name__ == "__main__":
    unittest.main()
