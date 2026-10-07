#!/usr/bin/env python3
"""Fail closed when Whisper manifest source identity and tracked gitlink diverge."""

from __future__ import annotations

import argparse
import re
import subprocess
from pathlib import Path

SOURCE_RE = re.compile(
    r'pub const WHISPER_SOURCE_COMMIT: &str = "([0-9a-fA-F]{40})";'
)
MODEL_REVISION_RE = re.compile(
    r'pub const WHISPER_MODEL_REVISION: &str = "([0-9a-fA-F]{40})";'
)
MODEL_LICENSE_RE = re.compile(
    r'pub const WHISPER_MODEL_LICENSE: &str = "([^"]+)";'
)


def expected_source_commit(manifest_path: Path) -> str:
    text = manifest_path.read_text(encoding="utf-8")
    match = SOURCE_RE.search(text)
    if not match:
        raise ValueError("WHISPER_SOURCE_COMMIT is missing or is not a full Git SHA")
    return match.group(1).lower()


def expected_model_revision(manifest_path: Path) -> str:
    text = manifest_path.read_text(encoding="utf-8")
    match = MODEL_REVISION_RE.search(text)
    if not match:
        raise ValueError("WHISPER_MODEL_REVISION is missing or is not a full Git SHA")
    return match.group(1).lower()


def expected_model_license(manifest_path: Path) -> str:
    text = manifest_path.read_text(encoding="utf-8")
    match = MODEL_LICENSE_RE.search(text)
    if not match:
        raise ValueError("WHISPER_MODEL_LICENSE is missing")
    return match.group(1)


def tracked_gitlink(repo_root: Path) -> tuple[str, str]:
    proc = subprocess.run(
        ["git", "ls-tree", "HEAD", "--", "third_party/whisper.cpp"],
        cwd=repo_root,
        check=True,
        text=True,
        stdout=subprocess.PIPE,
        stderr=subprocess.PIPE,
    )
    fields = proc.stdout.strip().split()
    if len(fields) < 3:
        raise ValueError("third_party/whisper.cpp is not tracked in HEAD")
    mode, object_type, sha = fields[:3]
    if mode != "160000" or object_type != "commit":
        raise ValueError(
            "third_party/whisper.cpp must be a gitlink (mode 160000, type commit)"
        )
    if not re.fullmatch(r"[0-9a-fA-F]{40}", sha):
        raise ValueError("tracked whisper.cpp gitlink is not a full Git SHA")
    return mode, sha.lower()


def validate(expected: str, actual: str) -> None:
    if expected.lower() != actual.lower():
        raise ValueError(
            "Whisper source provenance mismatch: "
            f"manifest expects {expected.lower()}, gitlink is {actual.lower()}"
        )


def checked_out_source_commit(repo_root: Path) -> str | None:
    source_root = repo_root / "third_party/whisper.cpp"
    if not (source_root / "CMakeLists.txt").is_file():
        return None
    proc = subprocess.run(
        ["git", "-C", str(source_root), "rev-parse", "HEAD"],
        cwd=repo_root,
        check=True,
        text=True,
        stdout=subprocess.PIPE,
        stderr=subprocess.PIPE,
    )
    sha = proc.stdout.strip().lower()
    if not re.fullmatch(r"[0-9a-f]{40}", sha):
        raise ValueError("checked-out whisper.cpp source is not a full Git SHA")
    return sha


def require_documented(path: Path, required: tuple[str, ...]) -> None:
    text = path.read_text(encoding="utf-8")
    for value in required:
        if value not in text:
            raise ValueError(f"{path.name} is missing required Whisper provenance: {value}")


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument(
        "--repo-root",
        type=Path,
        default=Path(__file__).resolve().parents[1],
    )
    args = parser.parse_args()
    repo_root = args.repo_root.resolve()
    manifest = repo_root / "src-tauri/src/asr/whisper/manifest.rs"
    expected = expected_source_commit(manifest)
    model_revision = expected_model_revision(manifest)
    model_license = expected_model_license(manifest)
    _mode, actual = tracked_gitlink(repo_root)
    validate(expected, actual)

    checked_out = checked_out_source_commit(repo_root)
    if checked_out is not None:
        validate(expected, checked_out)

    required_docs = (expected, model_revision, model_license)
    require_documented(repo_root / "docs/WHISPER_MODEL_LICENSES.md", required_docs)
    require_documented(repo_root / "docs/THIRD_PARTY_NOTICES.md", required_docs)

    print(
        "Whisper provenance OK: "
        f"source={actual} model_revision={model_revision} "
        f"submodule_checked_out={checked_out is not None}"
    )
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
