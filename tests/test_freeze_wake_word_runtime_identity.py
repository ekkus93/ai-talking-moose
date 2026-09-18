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


def elf_x86_64(marker: bytes = b"") -> bytes:
    data = bytearray(64 + len(marker))
    data[:4] = b"\x7fELF"
    data[4:6] = b"\x02\x01"
    struct.pack_into("<H", data, 18, 62)
    data[64:] = marker
    return bytes(data)


def macho_arm64(marker: bytes = b"") -> bytes:
    data = bytearray(64 + len(marker))
    data[:4] = b"\xcf\xfa\xed\xfe"
    struct.pack_into("<I", data, 4, 0x0100000C)
    data[64:] = marker
    return bytes(data)


def write_member(bundle: tarfile.TarFile, name: str, data: bytes) -> None:
    info = tarfile.TarInfo(name)
    info.size = len(data)
    bundle.addfile(info, io.BytesIO(data))


class WakeRuntimeIdentityFreezerTests(unittest.TestCase):
    def test_architecture_headers(self):
        self.assertEqual(module.architecture(elf_x86_64()), "elf-x86_64")
        self.assertEqual(module.architecture(macho_arm64()), "macho-arm64")
        self.assertIsNone(module.architecture(b"not-native"))

    def test_exact_c_api_library_set_is_frozen(self):
        with tempfile.TemporaryDirectory() as td:
            archive = Path(td) / "runtime.tar.bz2"
            required = ("libsherpa-onnx-c-api.so", "libonnxruntime.so")
            with tarfile.open(archive, "w:bz2") as bundle:
                write_member(bundle, "pkg/lib/libsherpa-onnx-c-api.so", elf_x86_64(b"capi"))
                write_member(bundle, "pkg/lib/libonnxruntime.so", elf_x86_64(b"ort"))
                write_member(bundle, "pkg/lib/ignored.so", elf_x86_64(b"ignored"))
            files = module.inspect_archive(archive, "elf-x86_64", required)
            self.assertEqual([Path(item["path"]).name for item in files], list(required))
            self.assertTrue(all(item["architecture"] == "elf-x86_64" for item in files))

    def test_wrong_architecture_is_rejected(self):
        with tempfile.TemporaryDirectory() as td:
            archive = Path(td) / "runtime.tar.bz2"
            required = ("libsherpa-onnx-c-api.so", "libonnxruntime.so")
            with tarfile.open(archive, "w:bz2") as bundle:
                write_member(bundle, "pkg/lib/libsherpa-onnx-c-api.so", macho_arm64())
                write_member(bundle, "pkg/lib/libonnxruntime.so", elf_x86_64())
            with self.assertRaisesRegex(RuntimeError, "wrong architecture"):
                module.inspect_archive(archive, "elf-x86_64", required)

    def test_missing_library_is_rejected(self):
        with tempfile.TemporaryDirectory() as td:
            archive = Path(td) / "runtime.tar.bz2"
            with tarfile.open(archive, "w:bz2") as bundle:
                write_member(bundle, "pkg/lib/libsherpa-onnx-c-api.so", elf_x86_64())
            with self.assertRaisesRegex(RuntimeError, "missing required native libraries"):
                module.inspect_archive(
                    archive,
                    "elf-x86_64",
                    ("libsherpa-onnx-c-api.so", "libonnxruntime.so"),
                )

    def test_traversal_is_rejected(self):
        with tempfile.TemporaryDirectory() as td:
            archive = Path(td) / "runtime.tar.bz2"
            with tarfile.open(archive, "w:bz2") as bundle:
                write_member(bundle, "../libsherpa-onnx-c-api.so", elf_x86_64())
            with self.assertRaisesRegex(RuntimeError, "unsafe archive member"):
                module.inspect_archive(archive, "elf-x86_64", ("libsherpa-onnx-c-api.so",))


if __name__ == "__main__":
    unittest.main()
