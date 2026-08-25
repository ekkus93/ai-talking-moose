#!/usr/bin/env python3
"""Fail closed when release metadata drifts across package formats."""
from __future__ import annotations

import argparse
import json
import re
import struct
import sys
import xml.etree.ElementTree as ET
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
EXPECTED_PRODUCT = "Talking Moose AI"
EXPECTED_IDENTIFIER = "com.talkingmoose.ai"
EXPECTED_MIN_MACOS = "13.4"
EXPECTED_CARGO_BINARY = "talking-moose-ai"


def fail(message: str) -> None:
    raise SystemExit(f"validate_release_metadata: {message}")


def parse_cargo_package_value(path: Path, key: str) -> str:
    # Avoid requiring a particular Python tomllib version in CI.
    in_package = False
    pattern = re.compile(rf'{re.escape(key)}\s*=\s*"([^"]+)"')
    for raw in path.read_text(encoding="utf-8").splitlines():
        line = raw.strip()
        if line.startswith("["):
            in_package = line == "[package]"
            continue
        if in_package:
            match = pattern.fullmatch(line)
            if match:
                return match.group(1)
    fail(f"could not read package {key!r} from {path}")
    raise AssertionError


def png_dimensions(path: Path) -> tuple[int, int]:
    data = path.read_bytes()
    if len(data) < 24 or data[:8] != b"\x89PNG\r\n\x1a\n":
        fail(f"{path} is not a PNG")
    return struct.unpack(">II", data[16:24])


def validate_icons(config: dict) -> None:
    configured = config["bundle"]["icon"]
    expected_png = {
        "icons/icon.png": (1024, 1024),
        "icons/32x32.png": (32, 32),
        "icons/128x128.png": (128, 128),
        "icons/128x128@2x.png": (256, 256),
    }
    for relative, dimensions in expected_png.items():
        if relative not in configured:
            fail(f"required icon is not configured: {relative}")
        path = ROOT / "src-tauri" / relative
        if png_dimensions(path) != dimensions:
            fail(f"{path} must be {dimensions[0]}x{dimensions[1]}")
        minimum_bytes = 200 if dimensions == (32, 32) else 1_000
        if path.stat().st_size < minimum_bytes:
            fail(f"{path} still looks like a placeholder")

    icns = ROOT / "src-tauri/icons/icon.icns"
    ico = ROOT / "src-tauri/icons/icon.ico"
    if "icons/icon.icns" not in configured:
        fail("ICNS application icon is not configured")
    icns_data = icns.read_bytes()
    if len(icns_data) < 16 or icns_data[:4] != b"icns":
        fail("valid ICNS application icon is required")
    declared_icns_size = struct.unpack(">I", icns_data[4:8])[0]
    if declared_icns_size != len(icns_data):
        fail("ICNS application icon has an invalid container size")
    chunk_type = icns_data[8:12]
    chunk_size = struct.unpack(">I", icns_data[12:16])[0]
    if chunk_type != b"ic10" or chunk_size != len(icns_data) - 8:
        fail("ICNS application icon must contain the generated 1024px ic10 chunk")
    embedded_png = icns_data[16:]
    if len(embedded_png) < 24 or embedded_png[:8] != b"\x89PNG\r\n\x1a\n":
        fail("ICNS ic10 payload is not a PNG")
    if struct.unpack(">II", embedded_png[16:24]) != (1024, 1024):
        fail("ICNS ic10 payload must be 1024x1024")

    if "icons/icon.ico" not in configured:
        fail("ICO application icon is not configured")
    ico_data = ico.read_bytes()
    if len(ico_data) < 6 or ico_data[:4] != b"\x00\x00\x01\x00":
        fail("valid ICO application icon is required")
    image_count = struct.unpack("<H", ico_data[4:6])[0]
    if image_count != 3 or len(ico_data) < 6 + image_count * 16:
        fail("ICO application icon must contain 32px, 128px, and 256px entries")
    found_sizes: set[int] = set()
    for index in range(image_count):
        offset = 6 + index * 16
        width, height, colors, reserved, planes, bits, payload_size, payload_offset = struct.unpack(
            "<BBBBHHII", ico_data[offset : offset + 16]
        )
        size = 256 if width == 0 else width
        height_size = 256 if height == 0 else height
        if size != height_size or colors != 0 or reserved != 0 or planes != 1 or bits != 32:
            fail("ICO application icon contains an invalid directory entry")
        payload_end = payload_offset + payload_size
        if payload_offset < 6 + image_count * 16 or payload_end > len(ico_data):
            fail("ICO application icon contains an invalid payload range")
        payload = ico_data[payload_offset:payload_end]
        if len(payload) < 24 or payload[:8] != b"\x89PNG\r\n\x1a\n":
            fail("ICO application icon entries must use embedded PNG payloads")
        if struct.unpack(">II", payload[16:24]) != (size, size):
            fail("ICO embedded PNG dimensions do not match the directory entry")
        found_sizes.add(size)
    if found_sizes != {32, 128, 256}:
        fail(f"ICO application icon has unexpected sizes: {sorted(found_sizes)}")


def main() -> None:
    parser = argparse.ArgumentParser()
    parser.add_argument("--tag", help="expected release tag, e.g. v0.1.0")
    args = parser.parse_args()

    package = json.loads((ROOT / "package.json").read_text(encoding="utf-8"))
    package_lock = json.loads((ROOT / "package-lock.json").read_text(encoding="utf-8"))
    tauri = json.loads((ROOT / "src-tauri/tauri.conf.json").read_text(encoding="utf-8"))
    native_runtime = json.loads((ROOT / "src-tauri/native/moonshine-runtime.json").read_text(encoding="utf-8"))
    cargo_manifest = ROOT / "src-tauri/Cargo.toml"
    cargo_version = parse_cargo_package_value(cargo_manifest, "version")
    cargo_name = parse_cargo_package_value(cargo_manifest, "name")
    cargo_default_run = parse_cargo_package_value(cargo_manifest, "default-run")
    lock_root_version = package_lock.get("packages", {}).get("", {}).get("version")
    versions = {package["version"], package_lock.get("version"), lock_root_version, tauri["version"], cargo_version}
    if None in versions or len(versions) != 1:
        fail(
            "version mismatch: "
            f"npm={package['version']} lock={package_lock.get('version')} "
            f"lock-root={lock_root_version} tauri={tauri['version']} cargo={cargo_version}"
        )
    version = versions.pop()
    if not re.fullmatch(r"[0-9]+\.[0-9]+\.[0-9]+(?:-[0-9A-Za-z.-]+)?", version):
        fail(f"application version is not release SemVer: {version!r}")

    if cargo_name != EXPECTED_CARGO_BINARY:
        fail(f"Cargo package name must be {EXPECTED_CARGO_BINARY!r}, got {cargo_name!r}")
    if cargo_default_run != EXPECTED_CARGO_BINARY:
        fail(
            "Cargo package default-run must select the Tauri application binary; "
            f"expected {EXPECTED_CARGO_BINARY!r}, got {cargo_default_run!r}"
        )
    if tauri.get("productName") != EXPECTED_PRODUCT:
        fail(f"productName must be {EXPECTED_PRODUCT!r}")
    if tauri.get("identifier") != EXPECTED_IDENTIFIER:
        fail(f"identifier must be {EXPECTED_IDENTIFIER!r}")
    bundle = tauri.get("bundle", {})
    if bundle.get("active") is not True:
        fail("Tauri bundling must be active for release")
    resources = bundle.get("resources", [])
    if "native/macos/notices/" not in resources:
        fail("macOS release notice resource directory is not bundled")
    macos = bundle.get("macOS", {})
    minimum = macos.get("minimumSystemVersion")
    if minimum != EXPECTED_MIN_MACOS:
        fail(f"minimum macOS version must be {EXPECTED_MIN_MACOS}, got {minimum!r}")
    if macos.get("entitlements") is not None:
        fail("V1 direct-download release intentionally requires no custom macOS entitlements")

    runtime_macos = native_runtime.get("macos", {})
    for arch in ("arm64", "x86_64"):
        runtime_minimum = runtime_macos.get(arch, {}).get("minimum_macos")
        if runtime_minimum != EXPECTED_MIN_MACOS:
            fail(
                f"native runtime minimum for {arch} must be {EXPECTED_MIN_MACOS}, "
                f"got {runtime_minimum!r}"
            )

    info_plist = ROOT / "src-tauri/Info.plist"
    try:
        plist_root = ET.parse(info_plist).getroot()
    except (ET.ParseError, OSError) as exc:
        fail(f"could not parse {info_plist}: {exc}")
    plist_dict = plist_root.find("dict")
    if plist_dict is None:
        fail("Info.plist does not contain a dictionary")
    children = list(plist_dict)
    microphone_description = None
    for index, child in enumerate(children[:-1]):
        if child.tag == "key" and child.text == "NSMicrophoneUsageDescription":
            microphone_description = children[index + 1].text
            break
    if not microphone_description or not microphone_description.strip():
        fail("NSMicrophoneUsageDescription is missing or empty")

    prepare_script = (ROOT / "scripts/prepare_moonshine_macos.sh").read_text(encoding="utf-8")
    if '-DCMAKE_OSX_DEPLOYMENT_TARGET="$deployment_target"' not in prepare_script:
        fail("Moonshine native build no longer applies the provenance deployment target")

    if args.tag and args.tag != f"v{version}":
        fail(f"tag {args.tag!r} does not match application version v{version}")

    release_notes = ROOT / "docs/releases" / f"v{version}.md"
    if not release_notes.is_file():
        fail(f"release notes are missing: {release_notes.relative_to(ROOT)}")
    if not (ROOT / "LICENSE").is_file():
        fail("project LICENSE is missing")
    if not (ROOT / "docs/THIRD_PARTY_NOTICES.md").is_file():
        fail("third-party notice inventory is missing")

    validate_icons(tauri)
    print(
        f"release-metadata-ok version={version} identifier={EXPECTED_IDENTIFIER} "
        f"minimum-macos={EXPECTED_MIN_MACOS}"
    )


if __name__ == "__main__":
    main()
