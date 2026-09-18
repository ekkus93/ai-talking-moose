from __future__ import annotations

import importlib.util
import tarfile
import tempfile
import unittest
from pathlib import Path

SCRIPT = Path(__file__).resolve().parents[1] / "scripts" / "freeze_wake_word_model_identity.py"
SPEC = importlib.util.spec_from_file_location("wake_model_freezer", SCRIPT)
module = importlib.util.module_from_spec(SPEC)
assert SPEC.loader is not None
SPEC.loader.exec_module(module)


class WakeModelIdentityFreezerTests(unittest.TestCase):
    def _archive(self, root: Path, *, traversal: bool = False) -> Path:
        source = root / "source"
        source.mkdir()
        for name in module.FILES:
            (source / name).write_bytes(("fixture-" + name).encode())
        archive = root / "fixture.tar.bz2"
        with tarfile.open(archive, "w:bz2") as bundle:
            for name in module.FILES:
                bundle.add(source / name, arcname=f"{module.MODEL_ID}/{name}")
            if traversal:
                extra = source / module.FILES[0]
                bundle.add(extra, arcname="../escape")
        return archive

    def test_selected_files_extract_deterministically(self):
        with tempfile.TemporaryDirectory() as td:
            root = Path(td)
            archive = self._archive(root)
            destination = root / "out"
            destination.mkdir()
            module.safe_extract_selected(archive, destination)
            self.assertEqual(sorted(p.name for p in destination.iterdir()), sorted(module.FILES))
            self.assertGreater(module.identity(destination / module.FILES[0])["bytes"], 0)

    def test_traversal_is_rejected(self):
        with tempfile.TemporaryDirectory() as td:
            root = Path(td)
            archive = self._archive(root, traversal=True)
            destination = root / "out"
            destination.mkdir()
            with self.assertRaisesRegex(RuntimeError, "unsafe archive member"):
                module.safe_extract_selected(archive, destination)

    def test_missing_required_file_is_rejected(self):
        with tempfile.TemporaryDirectory() as td:
            root = Path(td)
            archive = root / "missing.tar.bz2"
            with tarfile.open(archive, "w:bz2"):
                pass
            destination = root / "out"
            destination.mkdir()
            with self.assertRaisesRegex(RuntimeError, "archive missing required files"):
                module.safe_extract_selected(archive, destination)


if __name__ == "__main__":
    unittest.main()
