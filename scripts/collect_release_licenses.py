#!/usr/bin/env python3
"""Collect distributable dependency license texts into the staged macOS notice tree.

The collector intentionally over-includes resolved Rust crates and production npm
packages. Native Moonshine/ONNX notices are staged separately by
prepare_moonshine_macos.sh.
"""
from __future__ import annotations

import json
import re
import shutil
import subprocess
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
OUTPUT = ROOT / "src-tauri/native/macos/notices/Dependencies"
NOTICE_PREFIXES = ("license", "copying", "notice")


def safe(value: str) -> str:
    return re.sub(r"[^A-Za-z0-9._+-]+", "_", value)


def notice_files(directory: Path) -> list[Path]:
    if not directory.is_dir():
        return []
    return sorted(
        path
        for path in directory.iterdir()
        if path.is_file() and path.name.lower().startswith(NOTICE_PREFIXES)
    )


def copy_notices(kind: str, name: str, version: str, directory: Path) -> list[str]:
    files = notice_files(directory)
    destination = OUTPUT / kind / f"{safe(name)}-{safe(version)}"
    destination.mkdir(parents=True, exist_ok=True)
    copied: list[str] = []
    for source in files:
        target = destination / source.name
        shutil.copy2(source, target)
        copied.append(str(target.relative_to(OUTPUT)))
    return copied


def npm_rows() -> list[tuple[str, str, str, list[str]]]:
    lock = json.loads((ROOT / "package-lock.json").read_text(encoding="utf-8"))
    rows = []
    for relative, entry in sorted(lock.get("packages", {}).items()):
        if not relative.startswith("node_modules/") or entry.get("dev", False):
            continue
        name = entry.get("name") or relative.removeprefix("node_modules/")
        version = entry.get("version", "unknown")
        license_expression = entry.get("license", "UNKNOWN")
        copied = copy_notices("npm", name, version, ROOT / relative)
        rows.append((name, version, license_expression, copied))
    return rows


def cargo_rows() -> list[tuple[str, str, str, list[str]]]:
    result = subprocess.run(
        [
            "cargo",
            "metadata",
            "--locked",
            "--format-version",
            "1",
            "--manifest-path",
            str(ROOT / "src-tauri/Cargo.toml"),
        ],
        check=True,
        capture_output=True,
        text=True,
    )
    metadata = json.loads(result.stdout)
    rows = []
    for package in sorted(metadata["packages"], key=lambda item: (item["name"], item["version"])):
        if package.get("source") is None:
            continue
        name = package["name"]
        version = package["version"]
        license_expression = package.get("license") or package.get("license_file") or "UNKNOWN"
        manifest_dir = Path(package["manifest_path"]).parent
        copied = copy_notices("cargo", name, version, manifest_dir)
        rows.append((name, version, license_expression, copied))
    return rows


def main() -> None:
    if OUTPUT.exists():
        shutil.rmtree(OUTPUT)
    OUTPUT.mkdir(parents=True)

    npm = npm_rows()
    cargo = cargo_rows()
    inventory = OUTPUT / "DEPENDENCY_LICENSES.md"
    lines = [
        "# Bundled dependency license inventory",
        "",
        "Generated at release build time from `package-lock.json`, installed production npm packages, and `cargo metadata --locked`.",
        "",
        "This inventory deliberately over-includes resolved Rust crates. Native Moonshine/ONNX notices live beside this directory.",
        "",
        "| Ecosystem | Package | Version | Declared license | Copied notice files |",
        "| --- | --- | --- | --- | --- |",
    ]
    missing = []
    for ecosystem, rows in (("npm", npm), ("cargo", cargo)):
        for name, version, license_expression, copied in rows:
            if not copied:
                missing.append(f"{ecosystem}:{name}@{version} ({license_expression})")
            rendered = ", ".join(f"`{path}`" for path in copied) or "**MISSING**"
            lines.append(
                f"| {ecosystem} | `{name}` | `{version}` | `{license_expression}` | {rendered} |"
            )
    lines.extend(["", "## Missing notice files", ""])
    if missing:
        lines.extend(f"- {item}" for item in missing)
    else:
        lines.append("None detected by the collector.")
    inventory.write_text("\n".join(lines) + "\n", encoding="utf-8")

    if missing:
        raise SystemExit(
            "collect_release_licenses: dependency notice files missing:\n  " + "\n  ".join(missing)
        )
    print(f"release-license-inventory-ok npm={len(npm)} cargo={len(cargo)}")


if __name__ == "__main__":
    main()
