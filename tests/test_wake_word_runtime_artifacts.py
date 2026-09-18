from __future__ import annotations

import hashlib
import importlib.util
import json
import struct
import tempfile
import unittest
import zipfile
from pathlib import Path

SCRIPT = Path(__file__).resolve().parents[1] / "scripts" / "prepare_wake_word_runtime.py"
SPEC = importlib.util.spec_from_file_location("wake_runtime", SCRIPT)
wake_runtime = importlib.util.module_from_spec(SPEC)
assert SPEC.loader is not None
SPEC.loader.exec_module(wake_runtime)


def ident(data: bytes) -> dict:
    return {"bytes": len(data), "sha256": hashlib.sha256(data).hexdigest()}


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


def synthetic_runtime(architecture: str, lib_data: bytes, archive: Path) -> dict:
    lib_path = "native/libwake.so" if architecture == "elf-x86_64" else "native/libwake.dylib"
    archive_bytes = archive.read_bytes()
    return {
        "version": "v-test",
        "platforms": {
            "test": {
                "source_url": "https://example.invalid/runtime.zip",
                "architecture": architecture,
                "install_root": "runtime/test",
                "archive": {"filename": "runtime.zip", **ident(archive_bytes)},
                "files": [{"path": lib_path, **ident(lib_data)}],
            }
        },
    }


class WakeRuntimeArtifactsTests(unittest.TestCase):
    def make_archive(self, root: Path, name: str, data: bytes) -> Path:
        archive = root / "runtime.zip"
        with zipfile.ZipFile(archive, "w") as zf:
            zf.writestr(name, data)
        return archive

    def test_offline_prepared_runtime_is_deterministic_and_verified(self):
        with tempfile.TemporaryDirectory() as td:
            root = Path(td)
            data = elf_x86_64()
            archive = self.make_archive(root, "native/libwake.so", data)
            runtime = synthetic_runtime("elf-x86_64", data, archive)
            target = wake_runtime.prepare_runtime(root / "out", "test", runtime, archive_path=archive)
            self.assertEqual(target, root / "out" / "runtime/test")
            self.assertTrue((target / "native/libwake.so").is_file())
            self.assertEqual(wake_runtime.verify_installed(root / "out", "test", runtime["platforms"]["test"]), target)

    def test_wrong_architecture_fails(self):
        with tempfile.TemporaryDirectory() as td:
            root = Path(td)
            data = macho_arm64()
            archive = self.make_archive(root, "native/libwake.so", data)
            runtime = synthetic_runtime("elf-x86_64", data, archive)
            with self.assertRaisesRegex(wake_runtime.RuntimePreparationError, "architecture mismatch"):
                wake_runtime.prepare_runtime(root / "out", "test", runtime, archive_path=archive)

    def test_corrupt_archive_fails_before_extraction(self):
        with tempfile.TemporaryDirectory() as td:
            root = Path(td)
            data = elf_x86_64()
            archive = self.make_archive(root, "native/libwake.so", data)
            runtime = synthetic_runtime("elf-x86_64", data, archive)
            runtime["platforms"]["test"]["archive"]["sha256"] = "0" * 64
            with self.assertRaisesRegex(wake_runtime.RuntimePreparationError, "archive identity mismatch"):
                wake_runtime.prepare_runtime(root / "out", "test", runtime, archive_path=archive)

    def test_corrupt_consumed_library_fails(self):
        with tempfile.TemporaryDirectory() as td:
            root = Path(td)
            expected = elf_x86_64()
            archive = self.make_archive(root, "native/libwake.so", expected + b"x")
            runtime = synthetic_runtime("elf-x86_64", expected, archive)
            runtime["platforms"]["test"]["archive"] = {"filename": "runtime.zip", **ident(archive.read_bytes())}
            with self.assertRaisesRegex(wake_runtime.RuntimePreparationError, "library identity mismatch"):
                wake_runtime.prepare_runtime(root / "out", "test", runtime, archive_path=archive)

    def test_missing_required_library_fails(self):
        with tempfile.TemporaryDirectory() as td:
            root = Path(td)
            data = elf_x86_64()
            archive = self.make_archive(root, "native/other.so", data)
            runtime = synthetic_runtime("elf-x86_64", data, archive)
            runtime["platforms"]["test"]["archive"] = {"filename": "runtime.zip", **ident(archive.read_bytes())}
            with self.assertRaisesRegex(wake_runtime.RuntimePreparationError, "missing required"):
                wake_runtime.prepare_runtime(root / "out", "test", runtime, archive_path=archive)

    def test_path_traversal_is_rejected(self):
        with tempfile.TemporaryDirectory() as td:
            root = Path(td)
            archive = root / "runtime.zip"
            with zipfile.ZipFile(archive, "w") as zf:
                zf.writestr("../escape.so", elf_x86_64())
                zf.writestr("native/libwake.so", elf_x86_64())
            runtime = synthetic_runtime("elf-x86_64", elf_x86_64(), archive)
            with self.assertRaisesRegex(wake_runtime.RuntimePreparationError, "unsafe runtime archive member"):
                wake_runtime.prepare_runtime(root / "out", "test", runtime, archive_path=archive)

    def test_cached_install_is_reverified(self):
        with tempfile.TemporaryDirectory() as td:
            root = Path(td)
            data = elf_x86_64()
            archive = self.make_archive(root, "native/libwake.so", data)
            runtime = synthetic_runtime("elf-x86_64", data, archive)
            target = wake_runtime.prepare_runtime(root / "out", "test", runtime, archive_path=archive)
            (target / "native/libwake.so").write_bytes(b"corrupt")
            repaired = wake_runtime.prepare_runtime(root / "out", "test", runtime, archive_path=archive)
            self.assertEqual((repaired / "native/libwake.so").read_bytes(), data)

    def test_unsupported_platform_is_sanitized(self):
        runtime = {"platforms": {}}
        with tempfile.TemporaryDirectory() as td:
            with self.assertRaisesRegex(wake_runtime.RuntimePreparationError, "^Wake Word native runtime is unsupported"):
                wake_runtime.prepare_runtime(Path(td), "other", runtime)


if __name__ == "__main__":
    unittest.main()
