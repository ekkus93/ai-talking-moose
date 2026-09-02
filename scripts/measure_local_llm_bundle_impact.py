#!/usr/bin/env python3
"""Compare logical macOS .app sizes before/after the Local LLM native runtime."""
from __future__ import annotations

import argparse
import json
import os
from pathlib import Path


def fail(message: str) -> None:
    raise SystemExit(f"measure_local_llm_bundle_impact: {message}")


def snapshot(app: Path) -> dict[str, int]:
    if not app.is_dir() or app.suffix != ".app":
        fail(f"not a macOS .app directory: {app}")
    total = 0
    files = 0
    gguf_files = 0
    executable_bytes = 0
    executable_root = app / "Contents/MacOS"
    for root, _dirs, names in os.walk(app, followlinks=False):
        root_path = Path(root)
        for name in names:
            path = root_path / name
            if path.is_symlink():
                continue
            if not path.is_file():
                continue
            size = path.stat().st_size
            total += size
            files += 1
            if path.suffix.lower() == ".gguf":
                gguf_files += 1
            if path.parent == executable_root and os.access(path, os.X_OK):
                executable_bytes += size
    return {
        "logical_file_bytes": total,
        "file_count": files,
        "executable_bytes": executable_bytes,
        "gguf_file_count": gguf_files,
    }


def main() -> None:
    parser = argparse.ArgumentParser()
    parser.add_argument("--baseline-app", required=True, type=Path)
    parser.add_argument("--current-app", required=True, type=Path)
    parser.add_argument("--baseline-sha", required=True)
    parser.add_argument("--current-sha", required=True)
    parser.add_argument("--arch", required=True, choices=("arm64", "x86_64"))
    parser.add_argument("--output", required=True, type=Path)
    args = parser.parse_args()

    baseline = snapshot(args.baseline_app)
    current = snapshot(args.current_app)
    if baseline["gguf_file_count"] or current["gguf_file_count"]:
        fail("GGUF model weights must not be embedded in either comparison bundle")

    delta = current["logical_file_bytes"] - baseline["logical_file_bytes"]
    percent = (
        (delta / baseline["logical_file_bytes"] * 100.0)
        if baseline["logical_file_bytes"]
        else None
    )
    report = {
        "schema_version": 1,
        "architecture": args.arch,
        "baseline_sha": args.baseline_sha,
        "current_sha": args.current_sha,
        "baseline": baseline,
        "current": current,
        "delta_bytes": delta,
        "delta_percent": percent,
        "gguf_weights_embedded": False,
        "measurement": "sum of regular-file logical byte sizes inside .app; symlinks excluded",
    }
    args.output.parent.mkdir(parents=True, exist_ok=True)
    args.output.write_text(json.dumps(report, indent=2, sort_keys=True) + "\n", encoding="utf-8")
    print(json.dumps(report, sort_keys=True))


if __name__ == "__main__":
    main()
