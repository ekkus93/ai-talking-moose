#!/usr/bin/env python3
"""Collect distributable dependency license evidence into the macOS notice tree.

Production npm packages and Rust crates reachable for either shipped macOS target
must have either packaged license/notice text or an explicit declared license
expression. Native Moonshine/ONNX notices are staged separately by
prepare_moonshine_macos.sh. The statically linked llama.cpp runtime also has a
checked-in native notice, while its Rust binding crates must remain present in
this generated dependency inventory.
"""
from __future__ import annotations

import json
import re
import shutil
import subprocess
from pathlib import Path
from typing import Any

ROOT = Path(__file__).resolve().parents[1]
OUTPUT = ROOT / "src-tauri/native/macos/notices/Dependencies"
NOTICE_PREFIXES = ("license", "copying", "notice", "copyright")
MACOS_TARGETS = ("aarch64-apple-darwin", "x86_64-apple-darwin")
REQUIRED_LOCAL_LLM_CARGO = {
    "llama-cpp-2": "0.1.154",
    "llama-cpp-sys-2": "0.1.154",
}


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


def copy_notice_files(
    kind: str,
    name: str,
    version: str,
    directory: Path,
    explicit_license_file: Path | None = None,
) -> list[str]:
    files = notice_files(directory)
    if explicit_license_file is not None and explicit_license_file.is_file():
        if explicit_license_file not in files:
            files.append(explicit_license_file)
    files = sorted(set(files), key=lambda path: str(path))

    destination = OUTPUT / kind / f"{safe(name)}-{safe(version)}"
    destination.mkdir(parents=True, exist_ok=True)
    copied: list[str] = []
    used_names: set[str] = set()
    for source in files:
        target_name = source.name
        if target_name in used_names:
            target_name = f"{safe(source.parent.name)}-{target_name}"
        used_names.add(target_name)
        target = destination / target_name
        shutil.copy2(source, target)
        copied.append(str(target.relative_to(OUTPUT)))
    return copied


def write_declared_license_evidence(
    kind: str,
    name: str,
    version: str,
    license_expression: str,
    source: str | None,
    repository: str | None,
) -> str:
    destination = OUTPUT / kind / f"{safe(name)}-{safe(version)}"
    destination.mkdir(parents=True, exist_ok=True)
    target = destination / "LICENSE-DECLARATION.txt"
    lines = [
        f"Package: {name}",
        f"Version: {version}",
        f"Declared license expression: {license_expression}",
    ]
    if source:
        lines.append(f"Package source: {source}")
    if repository:
        lines.append(f"Repository: {repository}")
    lines.extend(
        [
            "",
            "The installed/published package did not contain a standalone license,",
            "copying, notice, or copyright file. This release inventory therefore",
            "records the package's declared license expression from package metadata.",
            "Final release review must confirm that this declaration satisfies the",
            "distribution obligations for the selected release.",
        ]
    )
    target.write_text("\n".join(lines) + "\n", encoding="utf-8")
    return str(target.relative_to(OUTPUT))


def license_evidence(
    *,
    kind: str,
    name: str,
    version: str,
    directory: Path,
    license_expression: str | None,
    explicit_license_file: Path | None = None,
    source: str | None = None,
    repository: str | None = None,
) -> tuple[list[str], str]:
    copied = copy_notice_files(
        kind,
        name,
        version,
        directory,
        explicit_license_file=explicit_license_file,
    )
    if copied:
        return copied, "packaged notice/license text"

    declared = (license_expression or "").strip()
    if declared and declared.upper() != "UNKNOWN":
        generated = write_declared_license_evidence(
            kind,
            name,
            version,
            declared,
            source,
            repository,
        )
        return [generated], "declared license metadata"

    return [], "unresolved"


def npm_rows() -> list[tuple[str, str, str, list[str], str]]:
    lock = json.loads((ROOT / "package-lock.json").read_text(encoding="utf-8"))
    rows = []
    for relative, entry in sorted(lock.get("packages", {}).items()):
        if not relative.startswith("node_modules/") or entry.get("dev", False):
            continue
        package_dir = ROOT / relative
        name = entry.get("name") or relative.removeprefix("node_modules/")
        version = entry.get("version", "unknown")
        installed_package: dict[str, Any] = {}
        package_json = package_dir / "package.json"
        if package_json.is_file():
            installed_package = json.loads(package_json.read_text(encoding="utf-8"))
        license_expression = entry.get("license") or installed_package.get("license")
        repository_value = installed_package.get("repository")
        if isinstance(repository_value, dict):
            repository = repository_value.get("url")
        else:
            repository = repository_value if isinstance(repository_value, str) else None
        evidence, method = license_evidence(
            kind="npm",
            name=name,
            version=version,
            directory=package_dir,
            license_expression=license_expression,
            source=entry.get("resolved"),
            repository=repository,
        )
        rows.append(
            (
                name,
                version,
                license_expression or "UNKNOWN",
                evidence,
                method,
            )
        )
    return rows


def cargo_metadata(target: str) -> dict[str, Any]:
    result = subprocess.run(
        [
            "cargo",
            "metadata",
            "--locked",
            "--format-version",
            "1",
            "--filter-platform",
            target,
            "--manifest-path",
            str(ROOT / "src-tauri/Cargo.toml"),
        ],
        check=True,
        capture_output=True,
        text=True,
    )
    return json.loads(result.stdout)


def non_dev_reachable_package_ids(metadata: dict[str, Any]) -> set[str]:
    resolve = metadata.get("resolve")
    if not resolve:
        raise SystemExit("collect_release_licenses: cargo metadata returned no resolve graph")

    nodes = {node["id"]: node for node in resolve.get("nodes", [])}
    root = resolve.get("root")
    roots = [root] if root else list(metadata.get("workspace_default_members", []))
    if not roots:
        roots = list(metadata.get("workspace_members", []))
    if not roots:
        raise SystemExit("collect_release_licenses: cargo metadata returned no workspace root")

    reachable: set[str] = set()
    pending = list(roots)
    while pending:
        package_id = pending.pop()
        if package_id in reachable:
            continue
        reachable.add(package_id)
        node = nodes.get(package_id)
        if node is None:
            continue
        for dependency in node.get("deps", []):
            dep_kinds = dependency.get("dep_kinds", [])
            if dep_kinds and all(kind.get("kind") == "dev" for kind in dep_kinds):
                continue
            pending.append(dependency["pkg"])
    return reachable


def cargo_rows() -> list[tuple[str, str, str, list[str], str]]:
    packages_by_id: dict[str, dict[str, Any]] = {}
    reachable_ids: set[str] = set()

    for target in MACOS_TARGETS:
        metadata = cargo_metadata(target)
        reachable_ids.update(non_dev_reachable_package_ids(metadata))
        for package in metadata.get("packages", []):
            packages_by_id[package["id"]] = package

    rows = []
    for package_id in sorted(
        reachable_ids,
        key=lambda item: (
            packages_by_id.get(item, {}).get("name", ""),
            packages_by_id.get(item, {}).get("version", ""),
            item,
        ),
    ):
        package = packages_by_id.get(package_id)
        if package is None or package.get("source") is None:
            continue
        name = package["name"]
        version = package["version"]
        license_expression = package.get("license")
        manifest_dir = Path(package["manifest_path"]).parent

        explicit_license_file = None
        raw_license_file = package.get("license_file")
        if raw_license_file:
            license_path = Path(raw_license_file)
            explicit_license_file = (
                license_path if license_path.is_absolute() else manifest_dir / license_path
            )

        evidence, method = license_evidence(
            kind="cargo",
            name=name,
            version=version,
            directory=manifest_dir,
            license_expression=license_expression,
            explicit_license_file=explicit_license_file,
            source=package.get("source"),
            repository=package.get("repository"),
        )
        rows.append(
            (
                name,
                version,
                license_expression or (raw_license_file or "UNKNOWN"),
                evidence,
                method,
            )
        )
    return rows


def validate_required_local_llm_rows(
    rows: list[tuple[str, str, str, list[str], str]],
) -> None:
    by_name = {
        name: (version, evidence, method)
        for name, version, _license, evidence, method in rows
    }
    for name, expected_version in REQUIRED_LOCAL_LLM_CARGO.items():
        found = by_name.get(name)
        if found is None:
            raise SystemExit(
                f"collect_release_licenses: shipped Local LLM dependency missing from inventory: {name}"
            )
        version, evidence, method = found
        if version != expected_version:
            raise SystemExit(
                "collect_release_licenses: shipped Local LLM dependency version drift: "
                f"{name} expected {expected_version}, found {version}"
            )
        if not evidence or method == "unresolved":
            raise SystemExit(
                f"collect_release_licenses: shipped Local LLM dependency has no license evidence: {name}"
            )


def main() -> None:
    if OUTPUT.exists():
        shutil.rmtree(OUTPUT)
    OUTPUT.mkdir(parents=True)

    npm = npm_rows()
    cargo = cargo_rows()
    validate_required_local_llm_rows(cargo)
    inventory = OUTPUT / "DEPENDENCY_LICENSES.md"
    lines = [
        "# Bundled dependency license inventory",
        "",
        "Generated at release build time from installed production npm packages and "
        "the non-dev Cargo dependency graphs filtered for both shipped macOS targets.",
        "",
        "Native Moonshine/ONNX and Local LLM native-runtime notices live beside "
        "this directory. Entries marked "
        "`declared license metadata` require final release review because the "
        "published package did not include standalone license/notice text.",
        "",
        "| Ecosystem | Package | Version | Declared license | Evidence method | Bundled evidence |",
        "| --- | --- | --- | --- | --- | --- |",
    ]
    unresolved = []
    declaration_only = []
    for ecosystem, rows in (("npm", npm), ("cargo", cargo)):
        for name, version, license_expression, evidence, method in rows:
            if not evidence:
                unresolved.append(f"{ecosystem}:{name}@{version} ({license_expression})")
            if method == "declared license metadata":
                declaration_only.append(f"{ecosystem}:{name}@{version} ({license_expression})")
            rendered = ", ".join(f"`{path}`" for path in evidence) or "**MISSING**"
            lines.append(
                f"| {ecosystem} | `{name}` | `{version}` | `{license_expression}` | "
                f"{method} | {rendered} |"
            )

    lines.extend(["", "## Declaration-only dependencies", ""])
    if declaration_only:
        lines.extend(f"- {item}" for item in declaration_only)
    else:
        lines.append("None.")

    lines.extend(["", "## Unresolved license evidence", ""])
    if unresolved:
        lines.extend(f"- {item}" for item in unresolved)
    else:
        lines.append("None.")

    inventory.write_text("\n".join(lines) + "\n", encoding="utf-8")

    if unresolved:
        raise SystemExit(
            "collect_release_licenses: dependency license evidence unresolved:\n  "
            + "\n  ".join(unresolved)
        )
    print(
        "release-license-inventory-ok "
        f"npm={len(npm)} cargo={len(cargo)} declaration_only={len(declaration_only)}"
    )


if __name__ == "__main__":
    main()
