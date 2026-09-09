#!/usr/bin/env python3
"""Evaluate the P0 production-candidate Cargo graph for permissive licensing."""

from __future__ import annotations

import argparse
import json
import re
from pathlib import Path

PERMISSIVE = {
    "0BSD",
    "Apache-2.0",
    "Apache-2.0 WITH LLVM-exception",
    "BSD-2-Clause",
    "BSD-3-Clause",
    "BSL-1.0",
    "CDLA-Permissive-2.0",
    "ISC",
    "MIT",
    "Unicode-3.0",
    "Unlicense",
    "Zlib",
}

TOKEN_RE = re.compile(r"\(|\)|\bAND\b|\bOR\b|\bWITH\b|[A-Za-z0-9.+-]+")


def normalize_expression(expression: str) -> str:
    # Older Cargo metadata sometimes uses the deprecated slash spelling.
    return expression.replace("MIT/Apache-2.0", "(MIT OR Apache-2.0)")


def has_permissive_choice(expression: str) -> bool:
    tokens = TOKEN_RE.findall(normalize_expression(expression))
    position = 0

    def atom() -> bool:
        nonlocal position
        if position >= len(tokens):
            raise ValueError(f"truncated SPDX expression: {expression!r}")
        token = tokens[position]
        if token == "(":
            position += 1
            value = parse_or()
            if position >= len(tokens) or tokens[position] != ")":
                raise ValueError(f"unbalanced SPDX expression: {expression!r}")
            position += 1
            return value
        if token in {"AND", "OR", "WITH", ")"}:
            raise ValueError(f"unexpected SPDX token {token!r} in {expression!r}")
        position += 1
        identifier = token
        if position < len(tokens) and tokens[position] == "WITH":
            position += 1
            if position >= len(tokens):
                raise ValueError(f"truncated WITH expression: {expression!r}")
            identifier = f"{identifier} WITH {tokens[position]}"
            position += 1
        return identifier in PERMISSIVE

    def parse_and() -> bool:
        nonlocal position
        value = atom()
        while position < len(tokens) and tokens[position] == "AND":
            position += 1
            right = atom()
            value = value and right
        return value

    def parse_or() -> bool:
        nonlocal position
        value = parse_and()
        while position < len(tokens) and tokens[position] == "OR":
            position += 1
            right = parse_and()
            value = value or right
        return value

    result = parse_or()
    if position != len(tokens):
        raise ValueError(f"unparsed SPDX tokens in {expression!r}: {tokens[position:]}")
    return result


def self_test() -> None:
    accepted = [
        "MIT",
        "MIT OR Apache-2.0",
        "MIT OR Apache-2.0 OR LGPL-2.1-or-later",
        "Apache-2.0 AND ISC",
        "(MIT OR Apache-2.0) AND Unicode-3.0",
        "Apache-2.0 WITH LLVM-exception OR MIT",
        "MIT/Apache-2.0",
    ]
    rejected = [
        "GPL-3.0-only",
        "LGPL-2.1-or-later",
        "MPL-2.0",
        "MIT AND GPL-3.0-only",
        "(MPL-2.0 OR GPL-3.0-only) AND MIT",
    ]
    for expression in accepted:
        assert has_permissive_choice(expression), expression
    for expression in rejected:
        assert not has_permissive_choice(expression), expression


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("metadata", nargs="?")
    parser.add_argument("inventory", nargs="?")
    parser.add_argument("--self-test", action="store_true")
    args = parser.parse_args()

    if args.self_test:
        self_test()
        return 0
    if not args.metadata or not args.inventory:
        parser.error("metadata and inventory paths are required")

    metadata = json.loads(Path(args.metadata).read_text(encoding="utf-8"))
    nodes = {node["id"]: node for node in metadata["resolve"]["nodes"]}
    root = metadata["resolve"]["root"]
    reachable: set[str] = set()
    stack = [root]
    while stack:
        package_id = stack.pop()
        if package_id in reachable:
            continue
        reachable.add(package_id)
        stack.extend(dependency["pkg"] for dependency in nodes[package_id]["deps"])

    packages = {package["id"]: package for package in metadata["packages"]}
    rows = []
    blockers = []
    for package_id in sorted(reachable):
        package = packages[package_id]
        row = {
            "name": package["name"],
            "version": package["version"],
            "license": package.get("license"),
            "license_file": package.get("license_file"),
            "source": package.get("source"),
        }
        rows.append(row)
        if package_id == root:
            continue
        expression = package.get("license")
        if not expression or not has_permissive_choice(expression):
            blockers.append(row)

    Path(args.inventory).write_text(
        json.dumps(rows, indent=2, sort_keys=True) + "\n", encoding="utf-8"
    )
    if blockers:
        raise SystemExit(
            "candidate graph has dependencies without a permissive SPDX choice: "
            + json.dumps(blockers, sort_keys=True)
        )
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
