#!/usr/bin/env python3
"""Fail closed on regressions in the Whisper native build policy."""

from __future__ import annotations

import argparse
import re
from pathlib import Path


class BuildPolicyError(ValueError):
    """Raised when src-tauri/build.rs violates Whisper build policy."""


REQUIRED_NATIVE_RERUN_SNIPPETS = (
    'whisper_src.join("CMakeLists.txt")',
    'whisper_src.join("cmake")',
    'whisper_src.join("include")',
    'whisper_src.join("src")',
    'whisper_src.join("ggml").join("CMakeLists.txt")',
    'whisper_src.join("ggml").join("cmake")',
    'whisper_src.join("ggml").join("include")',
    'whisper_src.join("ggml").join("src")',
)


def require_contains(text: str, snippet: str, description: str) -> None:
    if snippet not in text:
        raise BuildPolicyError(f"missing {description}: {snippet}")


def require_pattern(text: str, pattern: str, description: str) -> None:
    if re.search(pattern, text, flags=re.DOTALL) is None:
        raise BuildPolicyError(f"missing {description}")


def validate_build_policy_text(text: str) -> None:
    if "/proc/nproc" in text:
        raise BuildPolicyError("native build parallelism must not probe /proc/nproc")

    require_contains(
        text,
        'const WHISPER_BUILD_JOBS: &str = "TALKING_MOOSE_WHISPER_BUILD_JOBS";',
        "explicit Whisper build parallelism override",
    )
    require_contains(
        text,
        "cargo:rerun-if-env-changed={WHISPER_BUILD_JOBS}",
        "Cargo rerun marker for the Whisper build parallelism override",
    )
    require_pattern(
        text,
        r"fn\s+cpu_count\s*\(\s*\)\s*->\s*usize\s*\{.*std::thread::available_parallelism\s*\(",
        "portable available_parallelism() CPU fallback",
    )
    require_pattern(
        text,
        r"fn\s+explicit_whisper_build_jobs\s*\(\s*\)\s*->\s*Option<usize>\s*\{",
        "explicit Whisper build jobs helper",
    )
    require_pattern(
        text,
        r"fn\s+whisper_build_jobs\s*\(\s*\)\s*->\s*usize\s*\{.*explicit_whisper_build_jobs\s*\(\s*\)\s*\.unwrap_or_else\s*\(\s*cpu_count\s*\)",
        "override-preserving Whisper build jobs selector",
    )
    require_contains(
        text,
        "let threads = whisper_build_jobs().to_string();",
        "Whisper native build using the override-aware jobs selector",
    )

    for snippet in REQUIRED_NATIVE_RERUN_SNIPPETS:
        require_contains(text, snippet, "Whisper native rerun root")


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument(
        "--build-rs",
        type=Path,
        default=Path(__file__).resolve().parents[1] / "src-tauri/build.rs",
    )
    args = parser.parse_args()

    validate_build_policy_text(args.build_rs.read_text(encoding="utf-8"))
    print("Whisper build policy OK")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
