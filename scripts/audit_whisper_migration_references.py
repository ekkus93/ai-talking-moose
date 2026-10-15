#!/usr/bin/env python3
"""Inventory tracked ASR migration references without changing production behavior.

This is a review/disposition index, not a deletion plan or acceptance proof.
The git index is authoritative; submodule contents are separately pinned.
"""
from __future__ import annotations

import argparse
import json
import re
import subprocess
from collections import Counter
from pathlib import Path

TOKENS = (
    "moonshine",
    "moonshine_tiny_streaming",
    "moonshine_small_streaming",
    "whisper_native_linked",
    "libonnxruntime",
    "prepare_moonshine",
    "macosx_deployment_target",
    "minimumsystemversion",
    "13.4",
)
PATTERN = re.compile("|".join(re.escape(token) for token in TOKENS), re.IGNORECASE)


def disposition(path: str) -> tuple[str, str]:
    if path.startswith("docs/evidence/"):
        return "historical_evidence", "Review for archival labeling; never count old evidence as current"
    if path.startswith("src-tauri/src/ai/local_tts/"):
        return "kitten_tts", "Retain independently owned ONNX/KittenTTS runtime"
    if path.startswith("src-tauri/src/asr/moonshine/"):
        return "moonshine_runtime", "Remove only after WSP-200/300/400 acceptance"
    if path.startswith("src-tauri/native/"):
        return "native_assets", "Review provenance; preserve KittenTTS-owned runtime and notices"
    if path.startswith(".github/workflows/"):
        return "ci_workflows", "Replace obsolete Mac ASR gates with actual Whisper qualification"
    if path.startswith("src/generated/"):
        return "generated_contract", "Regenerate with Rust/TypeScript ASR cutover"
    if path.startswith("src/"):
        return "frontend", "Update UI and IPC selectors after WSP-200"
    if path.startswith("src-tauri/src/asr/"):
        return "asr_backend", "Retain Whisper; remove legacy ASR routing after WSP-200"
    if path.startswith("src-tauri/"):
        return "app_native", "Audit build, settings, packaging and other owner boundaries"
    if path.startswith("scripts/"):
        return "release_tooling", "Replace Moonshine checks; retain unrelated ONNX checks"
    if path.startswith("docs/") or path == "README.md":
        return "documentation", "Update active docs; preserve labeled historical records"
    return "other", "Manual owner classification required"


def main() -> None:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--output", type=Path, required=True)
    args = parser.parse_args()
    root = Path(__file__).resolve().parent.parent
    sha = subprocess.check_output(["git", "rev-parse", "HEAD"], cwd=root, text=True).strip()
    raw = subprocess.check_output(["git", "ls-files", "-z"], cwd=root)
    tracked = [item.decode("utf-8", errors="surrogateescape") for item in raw.split(b"\0") if item]
    files = []
    skipped = []
    for name in tracked:
        path = root / name
        if path.is_symlink() or not path.is_file():
            skipped.append({"path": name, "reason": "symlink, gitlink or nonregular"})
            continue
        data = path.read_bytes()
        if b"\0" in data[:8192]:
            skipped.append({"path": name, "reason": "binary"})
            continue
        try:
            content = data.decode("utf-8")
        except UnicodeDecodeError:
            skipped.append({"path": name, "reason": "non-UTF8"})
            continue
        matches = []
        for line_number, line in enumerate(content.splitlines(), 1):
            found = sorted(set(match.group(0).lower() for match in PATTERN.finditer(line)))
            if found:
                matches.append({"line": line_number, "tokens": found})
        if matches:
            owner, action = disposition(name)
            files.append({"path": name, "owner": owner, "disposition": action, "matches": matches})
    counts = Counter(file["owner"] for file in files)
    report = {
        "schema_version": 1,
        "head_sha": sha,
        "tracked_path_count": len(tracked),
        "matched_file_count": len(files),
        "owner_counts": dict(sorted(counts.items())),
        "files": files,
        "skipped": skipped,
        "limitations": [
            "Ownership/disposition is provisional and requires human review.",
            "Git submodule contents are not scanned; verify pinned gitlinks separately.",
            "No production removal or macOS transcription acceptance is implied.",
        ],
    }
    args.output.parent.mkdir(parents=True, exist_ok=True)
    args.output.write_text(json.dumps(report, indent=2, sort_keys=True) + "\n", encoding="utf-8")
    print(f"ASR reference inventory: head={sha} tracked={len(tracked)} matched={len(files)} skipped={len(skipped)}")
    for owner, count in sorted(counts.items()):
        print(f"  {owner}: {count}")
    print(f"JSON evidence: {args.output}")


if __name__ == "__main__":
    main()
