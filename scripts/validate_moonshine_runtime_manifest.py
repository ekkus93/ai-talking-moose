#!/usr/bin/env python3
from __future__ import annotations

import json
import pathlib
import re
import sys

ROOT = pathlib.Path(__file__).resolve().parents[1]
MANIFEST_PATH = ROOT / "src-tauri/native/moonshine-runtime.json"
DOC_PATH = ROOT / "docs/MOONSHINE_NATIVE.md"
TAURI_CONFIG = ROOT / "src-tauri/tauri.conf.json"
EXPECTED_MIN_MACOS = "13.4"


def fail(message: str) -> None:
    raise SystemExit(f"moonshine runtime manifest validation failed: {message}")


def main() -> int:
    manifest = json.loads(MANIFEST_PATH.read_text(encoding="utf-8"))
    runtime = manifest.get("runtime", {})
    ort = manifest.get("onnxruntime", {})
    macos = manifest.get("macos", {})

    if manifest.get("schema_version") != 1:
        fail("schema_version must be 1")
    if runtime.get("upstream") != "moonshine-ai/moonshine":
        fail("unexpected upstream repository")
    commit = runtime.get("source_commit", "")
    if not re.fullmatch(r"[0-9a-f]{40}", commit):
        fail("source_commit must be a full Git SHA")
    if runtime.get("header_version") != 30_000:
        fail("header version must match the pinned v3 C ABI")
    if ort.get("version") != "1.23.2":
        fail("unexpected ONNX Runtime version")
    if set(macos) != {"arm64", "x86_64"}:
        fail("macOS support must explicitly cover arm64 and x86_64")

    expected_targets = {
        "arm64": "aarch64-apple-darwin",
        "x86_64": "x86_64-apple-darwin",
    }
    for arch, target in expected_targets.items():
        entry = macos[arch]
        if entry.get("rust_target") != target:
            fail(f"incorrect Rust target for {arch}")
        if entry.get("minimum_macos") != EXPECTED_MIN_MACOS:
            fail(f"incorrect minimum macOS version for {arch}")
        if not re.fullmatch(r"[0-9a-f]{64}", entry.get("onnxruntime_sha256", "")):
            fail(f"invalid ONNX Runtime SHA-256 for {arch}")
        if not isinstance(entry.get("onnxruntime_bytes"), int) or entry["onnxruntime_bytes"] <= 0:
            fail(f"invalid ONNX Runtime byte size for {arch}")

    doc = DOC_PATH.read_text(encoding="utf-8")
    required_doc_values = [
        runtime["release"],
        commit,
        ort["version"],
        EXPECTED_MIN_MACOS,
        macos["arm64"]["onnxruntime_sha256"],
        macos["x86_64"]["onnxruntime_sha256"],
    ]
    for value in required_doc_values:
        if str(value) not in doc:
            fail(f"provenance value missing from MOONSHINE_NATIVE.md: {value}")

    config = json.loads(TAURI_CONFIG.read_text(encoding="utf-8"))
    macos_config = config.get("bundle", {}).get("macOS", {})
    frameworks = macos_config.get("frameworks", [])
    required_frameworks = {
        "native/macos/libmoonshine.dylib",
        "native/macos/libonnxruntime.1.23.2.dylib",
    }
    if not required_frameworks.issubset(set(frameworks)):
        fail("Tauri macOS framework list does not bundle both native dylibs")
    if macos_config.get("minimumSystemVersion") != EXPECTED_MIN_MACOS:
        fail("Tauri minimumSystemVersion does not match the pinned native runtime floor")
    resources = config.get("bundle", {}).get("resources", [])
    if "native/macos/notices/" not in resources:
        fail("Tauri resources do not include generated native notices")

    print("Moonshine native runtime provenance/configuration is internally consistent.")
    return 0


if __name__ == "__main__":
    sys.exit(main())
