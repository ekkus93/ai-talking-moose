#!/usr/bin/env python3
"""Compile embedded Python blocks in the Whisper real-CPU workflow."""

from __future__ import annotations

import textwrap
from pathlib import Path

WORKFLOW = Path(__file__).resolve().parents[1] / ".github/workflows/whisper-real-cpu-acceptance.yml"


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
            raise ValueError("unterminated embedded Python heredoc")
        blocks.append(textwrap.dedent("\n".join(body)) + "\n")
        index += 1
    return blocks


def main() -> int:
    blocks = embedded_python_blocks(WORKFLOW.read_text(encoding="utf-8"))
    if not blocks:
        raise SystemExit("Whisper acceptance workflow has no embedded Python validation block")
    for number, source in enumerate(blocks, start=1):
        compile(source, f"{WORKFLOW}#python-{number}", "exec")
    print(f"Whisper acceptance workflow Python syntax OK ({len(blocks)} block(s))")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
