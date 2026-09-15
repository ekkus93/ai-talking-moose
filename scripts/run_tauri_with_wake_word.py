#!/usr/bin/env python3
"""Run the Tauri CLI with the exact pinned Wake Word V1 artifacts prepared.

The sherpa native archive is independently size/SHA-256 verified before Cargo sees
it through SHERPA_ONNX_ARCHIVE_DIR. Model files are staged into the configured
Tauri resource tree only for the lifetime of the Tauri process and are removed
again afterward so local builds do not leave large untracked model binaries.
"""
from __future__ import annotations

import os
from pathlib import Path
import platform
import shutil
import subprocess
import sys

ROOT = Path(__file__).resolve().parents[1]
PREPARE = ROOT / "scripts/prepare_wake_word_artifacts.py"
CACHE_ROOT = ROOT / "src-tauri/target/wake-word-build"
RESOURCE_MODEL = ROOT / "src-tauri/resources/wake_word/model"
TAURI = ROOT / "node_modules/.bin/tauri"


def target_platform() -> str:
    system = platform.system().lower()
    machine = platform.machine().lower()
    if system == "linux" and machine in {"x86_64", "amd64"}:
        return "linux-x86_64"
    if system == "darwin" and machine in {"arm64", "aarch64"}:
        return "macos-arm64"
    if system == "darwin" and machine in {"x86_64", "amd64"}:
        return "macos-x86_64"
    raise SystemExit(
        f"Wake Word V1 Tauri build is unsupported on {system}/{machine}; "
        "supported build targets are linux-x86_64, macos-arm64, and macos-x86_64."
    )


def clear_staged_model() -> None:
    if not RESOURCE_MODEL.exists():
        return
    for child in RESOURCE_MODEL.iterdir():
        if child.name == ".gitkeep":
            continue
        if child.is_dir():
            shutil.rmtree(child)
        else:
            child.unlink()


def stage_model(source: Path) -> None:
    RESOURCE_MODEL.mkdir(parents=True, exist_ok=True)
    clear_staged_model()
    for source_path in source.rglob("*"):
        if not source_path.is_file():
            continue
        relative = source_path.relative_to(source)
        target = RESOURCE_MODEL / relative
        target.parent.mkdir(parents=True, exist_ok=True)
        shutil.copy2(source_path, target)


def main() -> int:
    target = target_platform()
    destination = CACHE_ROOT / target
    subprocess.run(
        [
            sys.executable,
            str(PREPARE),
            "--destination",
            str(destination),
            "--platform",
            target,
        ],
        cwd=ROOT,
        check=True,
    )
    stage_model(destination / "model")
    env = os.environ.copy()
    env["SHERPA_ONNX_ARCHIVE_DIR"] = str(destination / "runtime-archive")
    args = [arg for arg in sys.argv[1:] if arg != "--"]
    try:
        completed = subprocess.run([str(TAURI), *args], cwd=ROOT, env=env, check=False)
        return completed.returncode
    finally:
        clear_staged_model()


if __name__ == "__main__":
    raise SystemExit(main())
