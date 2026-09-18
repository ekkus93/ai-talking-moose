#!/usr/bin/env python3
"""Validate Wake Word V1 production artifact manifest schema and freezer consistency."""
from __future__ import annotations

import argparse
import importlib.util
import json
import re
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
SHA256 = re.compile(r"^[0-9a-f]{64}$")


class ManifestError(RuntimeError):
    pass


def load_module(path: Path, name: str):
    spec = importlib.util.spec_from_file_location(name, path)
    module = importlib.util.module_from_spec(spec)
    assert spec.loader is not None
    spec.loader.exec_module(module)
    return module


def require_identity(item: dict, label: str, production: bool) -> None:
    size = item.get("bytes")
    digest = item.get("sha256")
    if not isinstance(size, int) or size < 0:
        raise ManifestError(f"{label} has invalid byte size")
    if not isinstance(digest, str) or not SHA256.fullmatch(digest):
        raise ManifestError(f"{label} has invalid SHA-256")
    if production and (size == 0 or digest == "0" * 64):
        raise ManifestError(f"{label} contains a production placeholder")


def validate(document: dict, *, production: bool) -> None:
    if document.get("schema_version") != 1:
        raise ManifestError("unsupported manifest schema")
    policy = document.get("policy")
    if not isinstance(policy, dict):
        raise ManifestError("missing policy")
    production_enabled = bool(policy.get("production_mode"))
    if production and not production_enabled:
        raise ManifestError("production validation requested but production_mode is disabled")

    model_freezer = load_module(ROOT / "scripts/freeze_wake_word_model_identity.py", "model_freezer")
    runtime_freezer = load_module(ROOT / "scripts/freeze_wake_word_runtime_identity.py", "runtime_freezer")

    artifacts = document.get("artifacts")
    if not isinstance(artifacts, list) or len(artifacts) != 1:
        raise ManifestError("exactly one V1 model artifact is required")
    model = artifacts[0]
    if model.get("kind") != "kws-model" or model.get("id") != model_freezer.MODEL_ID:
        raise ManifestError("model identity disagrees with freezer policy")
    if model.get("source_url") != model_freezer.ARCHIVE_URL:
        raise ManifestError("model source URL disagrees with freezer policy")
    require_identity(model.get("archive", {}), "model archive", production_enabled)
    files = model.get("files")
    if not isinstance(files, list):
        raise ManifestError("model files must be a list")
    by_name = {item.get("name"): item for item in files if isinstance(item, dict)}
    if set(by_name) != set(model_freezer.FILES):
        raise ManifestError("model consumed-file set disagrees with freezer policy")
    for name in model_freezer.FILES:
        require_identity(by_name[name], f"model file {name}", production_enabled)
    keyword = model.get("keyword")
    if not isinstance(keyword, dict) or keyword.get("source") != model_freezer.KEYWORD_SOURCE:
        raise ManifestError("keyword source disagrees with freezer policy")
    require_identity(keyword, "keyword artifact", production_enabled)

    runtime = document.get("runtime")
    if not isinstance(runtime, dict):
        raise ManifestError("missing runtime manifest")
    expected_version = f"v{runtime_freezer.VERSION}"
    if policy.get("runtime_version") != expected_version or runtime.get("version") != expected_version:
        raise ManifestError("runtime version disagrees with freezer policy")
    platforms = runtime.get("platforms")
    if not isinstance(platforms, dict) or set(platforms) != set(runtime_freezer.PLATFORMS):
        raise ManifestError("runtime platform set disagrees with freezer policy")
    for key, freezer_cfg in runtime_freezer.PLATFORMS.items():
        cfg = platforms[key]
        expected_url = f"{runtime_freezer.BASE}/{freezer_cfg['filename']}"
        if cfg.get("source_url") != expected_url:
            raise ManifestError(f"{key} runtime source URL disagrees with freezer policy")
        if cfg.get("architecture") != freezer_cfg["architecture"]:
            raise ManifestError(f"{key} runtime architecture disagrees with freezer policy")
        if cfg.get("install_root") != f"runtime/sherpa-onnx/{expected_version}/{key}":
            raise ManifestError(f"{key} install root is not deterministic")
        if cfg.get("bundle_root") != f"resources/runtime/sherpa-onnx/{expected_version}/{key}":
            raise ManifestError(f"{key} bundle root is not deterministic")
        archive = cfg.get("archive", {})
        if archive.get("filename") != freezer_cfg["filename"]:
            raise ManifestError(f"{key} archive filename disagrees with freezer policy")
        require_identity(archive, f"{key} runtime archive", production_enabled)
        runtime_files = cfg.get("files")
        if not isinstance(runtime_files, list) or not runtime_files:
            raise ManifestError(f"{key} runtime files are missing")
        paths = set()
        for item in runtime_files:
            path = item.get("path")
            if not isinstance(path, str) or path in paths or path.startswith("/") or ".." in Path(path).parts:
                raise ManifestError(f"{key} runtime file path is invalid")
            paths.add(path)
            require_identity(item, f"{key} runtime file {path}", production_enabled)


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--manifest", default="wake-word-artifacts.json")
    parser.add_argument("--production", action="store_true")
    args = parser.parse_args()
    try:
        document = json.loads(Path(args.manifest).read_text(encoding="utf-8"))
        validate(document, production=args.production)
    except (OSError, json.JSONDecodeError, ManifestError) as exc:
        print(f"wake artifact manifest validation failed: {exc}", file=sys.stderr)
        return 1
    print("wake artifact production manifest is consistent")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
