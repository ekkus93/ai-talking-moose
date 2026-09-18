#!/usr/bin/env python3
"""Prepare the exact pinned sherpa-onnx native runtime for Wake Word V1."""
from __future__ import annotations

import argparse
import hashlib
import json
import platform as host_platform
import shutil
import struct
import sys
import tarfile
import tempfile
import urllib.request
import zipfile
from pathlib import Path, PurePosixPath

MANIFEST = Path(__file__).resolve().parents[1] / "wake-word-artifacts.json"


class RuntimePreparationError(RuntimeError):
    """Sanitized runtime preparation failure."""


def _identity(path: Path) -> tuple[int, str]:
    h = hashlib.sha256()
    size = 0
    with path.open("rb") as src:
        for chunk in iter(lambda: src.read(1024 * 1024), b""):
            size += len(chunk)
            h.update(chunk)
    return size, h.hexdigest()


def _verify_identity(path: Path, expected: dict, label: str) -> None:
    if not path.is_file():
        raise RuntimePreparationError(f"missing required {label}")
    size, digest = _identity(path)
    if size != expected["bytes"] or digest != expected["sha256"]:
        raise RuntimePreparationError(f"{label} identity mismatch")


def _architecture(data: bytes) -> str | None:
    if (
        len(data) >= 20
        and data[:4] == b"\x7fELF"
        and data[4:6] == b"\x02\x01"
        and struct.unpack_from("<H", data, 18)[0] == 62
    ):
        return "elf-x86_64"
    if (
        len(data) >= 8
        and data[:4] == b"\xcf\xfa\xed\xfe"
        and struct.unpack_from("<I", data, 4)[0] == 0x0100000C
    ):
        return "macho-arm64"
    return None


def _verify_architecture(path: Path, expected: str) -> None:
    with path.open("rb") as src:
        header = src.read(64)
    if _architecture(header) != expected:
        raise RuntimePreparationError("native runtime architecture mismatch")


def _safe_member(name: str) -> PurePosixPath:
    member = PurePosixPath(name)
    if member.is_absolute() or ".." in member.parts or not member.parts:
        raise RuntimePreparationError("unsafe runtime archive member")
    return member


def _host_key() -> str:
    machine = host_platform.machine().lower()
    if sys.platform.startswith("linux") and machine in {"x86_64", "amd64"}:
        return "linux-x86_64"
    if sys.platform == "darwin" and machine in {"arm64", "aarch64"}:
        return "macos-arm64"
    raise RuntimePreparationError("Wake Word native runtime is unsupported on this platform")


def load_runtime_manifest(path: Path = MANIFEST) -> dict:
    try:
        document = json.loads(path.read_text(encoding="utf-8"))
        runtime = document["runtime"]
        if runtime["version"] != document["policy"]["runtime_version"]:
            raise RuntimePreparationError("runtime version policy mismatch")
        return runtime
    except RuntimePreparationError:
        raise
    except Exception as exc:
        raise RuntimePreparationError("invalid Wake Word artifact manifest") from exc


def verify_installed(root: Path, platform_key: str, cfg: dict) -> Path:
    install_root = root / cfg["install_root"]
    for item in cfg["files"]:
        target = install_root / PurePosixPath(item["path"])
        _verify_identity(target, item, "native runtime library")
        _verify_architecture(target, cfg["architecture"])
    return install_root


def _extract_required_archive_members(archive_path: Path, stage: Path, cfg: dict) -> None:
    required = {item["path"]: item for item in cfg["files"]}
    archive_format = cfg.get("archive_format", "zip")
    try:
        if archive_format == "zip":
            with zipfile.ZipFile(archive_path) as archive:
                names = {info.filename for info in archive.infolist()}
                for name in names:
                    _safe_member(name)
                for member_name, item in required.items():
                    if member_name not in names:
                        raise RuntimePreparationError("missing required native runtime library")
                    data = archive.read(member_name)
                    target = stage / PurePosixPath(member_name)
                    target.parent.mkdir(parents=True, exist_ok=True)
                    target.write_bytes(data)
                    _verify_identity(target, item, "native runtime library")
                    _verify_architecture(target, cfg["architecture"])
        elif archive_format == "tar.bz2":
            with tarfile.open(archive_path, mode="r:bz2") as archive:
                members = {member.name: member for member in archive.getmembers()}
                for member in archive.getmembers():
                    _safe_member(member.name)
                    if member.issym() or member.islnk():
                        raise RuntimePreparationError("unsafe runtime archive member")
                for member_name, item in required.items():
                    member = members.get(member_name)
                    if member is None or not member.isfile():
                        raise RuntimePreparationError("missing required native runtime library")
                    source = archive.extractfile(member)
                    if source is None:
                        raise RuntimePreparationError("missing required native runtime library")
                    target = stage / PurePosixPath(member_name)
                    target.parent.mkdir(parents=True, exist_ok=True)
                    with source, target.open("wb") as out:
                        shutil.copyfileobj(source, out, length=1024 * 1024)
                    _verify_identity(target, item, "native runtime library")
                    _verify_architecture(target, cfg["architecture"])
        else:
            raise RuntimePreparationError("unsupported Wake Word native runtime archive format")
    except RuntimePreparationError:
        raise
    except (OSError, tarfile.TarError, zipfile.BadZipFile, KeyError) as exc:
        raise RuntimePreparationError("invalid Wake Word native runtime archive") from exc


def prepare_runtime(
    root: Path,
    platform_key: str,
    runtime: dict,
    *,
    archive_path: Path | None = None,
    cache_dir: Path | None = None,
) -> Path:
    platforms = runtime["platforms"]
    if platform_key not in platforms:
        raise RuntimePreparationError("Wake Word native runtime is unsupported on this platform")
    cfg = platforms[platform_key]
    install_root = root / cfg["install_root"]

    # A prepared cache is never trusted by presence alone.
    if install_root.exists():
        try:
            return verify_installed(root, platform_key, cfg)
        except RuntimePreparationError:
            shutil.rmtree(install_root)

    cache = cache_dir or root / ".cache" / "wake-word-runtime"
    cache.mkdir(parents=True, exist_ok=True)
    cached_archive = cache / cfg["archive"]["filename"]

    if archive_path is not None:
        shutil.copyfile(archive_path, cached_archive)
    elif not cached_archive.exists():
        try:
            with urllib.request.urlopen(cfg["source_url"], timeout=120) as response, cached_archive.open("wb") as out:
                shutil.copyfileobj(response, out, length=1024 * 1024)
        except Exception as exc:
            cached_archive.unlink(missing_ok=True)
            raise RuntimePreparationError("failed to obtain pinned Wake Word native runtime") from exc

    # Cached downloads are always re-hashed before extraction.
    _verify_identity(cached_archive, cfg["archive"], "native runtime archive")

    with tempfile.TemporaryDirectory(prefix="wake-runtime-stage-") as td:
        stage = Path(td)
        _extract_required_archive_members(cached_archive, stage, cfg)

        install_root.parent.mkdir(parents=True, exist_ok=True)
        if install_root.exists():
            shutil.rmtree(install_root)
        shutil.move(str(stage), str(install_root))

    return verify_installed(root, platform_key, cfg)


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--root", type=Path, default=Path("."))
    parser.add_argument("--platform", dest="platform_key")
    parser.add_argument("--archive", type=Path)
    parser.add_argument("--cache-dir", type=Path)
    parser.add_argument("--verify-only", action="store_true")
    args = parser.parse_args()

    try:
        runtime = load_runtime_manifest()
        key = args.platform_key or _host_key()
        cfg = runtime["platforms"].get(key)
        if cfg is None:
            raise RuntimePreparationError("Wake Word native runtime is unsupported on this platform")
        if args.verify_only:
            path = verify_installed(args.root, key, cfg)
        else:
            path = prepare_runtime(
                args.root,
                key,
                runtime,
                archive_path=args.archive,
                cache_dir=args.cache_dir,
            )
        print(f"wake-word runtime ready: {key} {runtime['version']} at {cfg['install_root']}")
        return 0
    except RuntimePreparationError as exc:
        print(f"wake-word runtime error: {exc}", file=sys.stderr)
        return 2


if __name__ == "__main__":
    raise SystemExit(main())
