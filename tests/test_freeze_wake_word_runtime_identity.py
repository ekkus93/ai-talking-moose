from __future__ import annotations

import importlib.util
import io
import struct
import tarfile
import tempfile
import unittest
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


def add_file(bundle: tarfile.TarFile, name: str, payload: bytes) -> None:
    info = tarfile.TarInfo(name)
    info.size = len(payload)
    bundle.addfile(info, io.BytesIO(payload))


class WakeRuntimeIdentityFreezerTests(unittest.TestCase):
    def test_architecture_headers(self):
        self.assertEqual(module.architecture(elf_x86_64()), "elf-x86_64")
        self.assertEqual(module.architecture(macho_arm64()), "macho-arm64")
        self.assertIsNone(module.architecture(b"not-a-native-library"))

    def test_archive_identity_and_architecture(self):
        with tempfile.TemporaryDirectory() as td:
            archive = Path(td) / "runtime.tar.bz2"
            payload = elf_x86_64()
            with tarfile.open(archive, "w:bz2") as bundle:
                add_file(bundle, "runtime/libonnxruntime.so", payload)
                add_file(bundle, "runtime/libsherpa-onnx-c-api.so", payload)
            files = module.inspect_archive(
                archive,
                "elf-x86_64",
                ("libonnxruntime.so", "libsherpa-onnx-c-api.so"),
            )
            self.assertEqual(len(files), 2)
            self.assertTrue(all(item["architecture"] == "elf-x86_64" for item in files))
            self.assertTrue(all(item["bytes"] == len(payload) for item in files))
            self.assertTrue(all(module.identity_bytes(payload)["sha256"] == item["sha256"] for item in files))

    def test_wrong_architecture_is_rejected(self):
        with tempfile.TemporaryDirectory() as td:
            archive = Path(td) / "runtime.tar.bz2"
            with tarfile.open(archive, "w:bz2") as bundle:
                add_file(bundle, "runtime/libonnxruntime.so", macho_arm64())
                add_file(bundle, "runtime/libsherpa-onnx-c-api.so", macho_arm64())
            with self.assertRaisesRegex(RuntimeError, "wrong architecture"):
                module.inspect_archive(
                    archive,
                    "elf-x86_64",
                    ("libonnxruntime.so", "libsherpa-onnx-c-api.so"),
                )

    def test_archive_traversal_is_rejected(self):
        with tempfile.TemporaryDirectory() as td:
            archive = Path(td) / "runtime.tar.bz2"
            with tarfile.open(archive, "w:bz2") as bundle:
                add_file(bundle, "../escape.so", elf_x86_64())
            with self.assertRaisesRegex(RuntimeError, "unsafe archive member"):
                module.inspect_archive(
                    archive,
                    "elf-x86_64",
                    ("libonnxruntime.so", "libsherpa-onnx-c-api.so"),
                )


if __name__ == "__main__":
    unittest.main()
