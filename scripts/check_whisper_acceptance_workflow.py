#!/usr/bin/env python3
"""Statically validate the Whisper real-CPU acceptance workflow.

The real acceptance workflow is intentionally not run by ordinary CI because it
may download the real Whisper model. This checker keeps ordinary CI cheap while
still failing closed on workflow regressions that would make acceptance evidence
untrustworthy or unreachable.
"""

from __future__ import annotations

import re
import textwrap
from pathlib import Path

WORKFLOW = (
    Path(__file__).resolve().parents[1]
    / ".github/workflows/whisper-real-cpu-acceptance.yml"
)


class WorkflowPolicyError(ValueError):
    """Raised when the real-CPU acceptance workflow violates policy."""


def embedded_python_blocks(text: str) -> list[str]:
    lines = text.splitlines()
    blocks: list[str] = []
    index = 0
    while index < len(lines):
        if "<<'PY'" not in lines[index] and '<<"PY"' not in lines[index]:
            index += 1
            continue
        index += 1
        body: list[str] = []
        while index < len(lines) and lines[index].strip() != "PY":
            body.append(lines[index])
            index += 1
        if index >= len(lines):
            raise WorkflowPolicyError("unterminated embedded Python heredoc")
        blocks.append(textwrap.dedent("\n".join(body)) + "\n")
        index += 1
    return blocks


def require_contains(text: str, snippet: str, description: str) -> None:
    if snippet not in text:
        raise WorkflowPolicyError(f"missing {description}: {snippet}")


def forbid_pattern(text: str, pattern: str, description: str) -> None:
    if re.search(pattern, text, flags=re.MULTILINE):
        raise WorkflowPolicyError(f"forbidden {description}: {pattern}")


def validate_manual_only_trigger(text: str) -> None:
    require_contains(text, "workflow_dispatch:", "manual workflow_dispatch trigger")
    forbid_pattern(text, r"^\s*push\s*:", "push trigger")
    forbid_pattern(text, r"^\s*pull_request\s*:", "pull_request trigger")
    forbid_pattern(text, r"^\s*schedule\s*:", "scheduled trigger")


def validate_acceptance_contract(text: str) -> None:
    require_contains(
        text,
        "python3 scripts/check_whisper_provenance.py",
        "native-source/model provenance preflight",
    )
    require_contains(
        text,
        "--features whisper-acceptance",
        "opt-in whisper-acceptance build feature",
    )
    require_contains(
        text,
        'test ! -e "$model_root"',
        "clean-profile production-installer precondition",
    )
    require_contains(
        text,
        '"$binary" install "$model_root"',
        "production installer invocation",
    )
    require_contains(
        text,
        '"$binary" delete "$model_root"',
        "production installer deletion invocation",
    )
    require_contains(
        text,
        'initial_install["disposition"] == "installed"',
        "clean-profile first-install assertion",
    )
    require_contains(
        text,
        'install["disposition"] == "installed"',
        "delete-and-reinstall assertion",
    )
    require_contains(
        text,
        "sudo unshare --net",
        "network-isolated transcription invocation",
    )
    require_contains(
        text,
        "--require-network-denied",
        "network-denial acceptance assertion",
    )
    require_contains(
        text,
        "whisper-real-cpu-evidence.json",
        "machine-readable real-CPU evidence output",
    )
    require_contains(
        text,
        "actions/upload-artifact@v4",
        "real-CPU evidence artifact upload",
    )
    require_contains(
        text,
        "hashlib.file_digest(model_file, \"sha256\")",
        "independent streaming model SHA-256 verification",
    )
    require_contains(
        text,
        '"workflow_identity"',
        "machine-readable immutable workflow run identity",
    )
    require_contains(
        text,
        'whisper_source_expected = manifest_value("WHISPER_SOURCE_COMMIT")',
        "canonical source revision read independently of the install report",
    )
    require_contains(
        text,
        'assert whisper_source_actual == whisper_source_expected',
        "actual gitlink versus canonical source assertion",
    )
    require_contains(
        text,
        'assert install["sha256"] == model_sha256_expected',
        "model SHA versus canonical manifest assertion",
    )
    require_contains(
        text,
        'assert install["expected_bytes"] == model_bytes_expected',
        "model byte count versus canonical manifest assertion",
    )
    require_contains(
        text,
        "whisper-real-cpu-${{ github.run_id }}-${{ github.run_attempt }}-${{ github.sha }}",
        "run-attempt and SHA-bound real-CPU evidence artifact name",
    )
    require_contains(
        text,
        'tail -n 80 "$log"',
        "bounded phase failure diagnostics",
    )
    forbid_pattern(text, r"\b(?:curl|wget)\b", "ad hoc shell model download")


def validate_shell_continuations(text: str) -> None:
    """Reject shell line continuations that silently split a command."""
    lines = text.splitlines()
    for number, line in enumerate(lines[:-1], start=1):
        if line.rstrip().endswith("\\"):
            if line != line.rstrip():
                raise WorkflowPolicyError(
                    f"trailing whitespace after shell continuation on line {number}"
                )
            if not lines[number].strip():
                raise WorkflowPolicyError(
                    f"blank line after shell continuation on line {number}"
                )


def validate_embedded_python(text: str) -> None:
    blocks = embedded_python_blocks(text)
    if not blocks:
        raise WorkflowPolicyError(
            "Whisper acceptance workflow has no embedded Python validation block"
        )
    for number, source in enumerate(blocks, start=1):
        compile(source, f"{WORKFLOW}#python-{number}", "exec")


def validate_workflow_text(text: str) -> int:
    validate_manual_only_trigger(text)
    validate_acceptance_contract(text)
    validate_shell_continuations(text)
    validate_embedded_python(text)
    return len(embedded_python_blocks(text))


def main() -> int:
    text = WORKFLOW.read_text(encoding="utf-8")
    block_count = validate_workflow_text(text)
    print(
        "Whisper acceptance workflow policy OK "
        f"({block_count} embedded Python block(s))"
    )
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
