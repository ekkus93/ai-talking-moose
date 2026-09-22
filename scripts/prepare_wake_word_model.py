#!/usr/bin/env python3
"""Prepare the exact pinned sherpa Wake Word model with fail-closed identity checks."""
from __future__ import annotations

import argparse
import hashlib
import json
import shutil
import tarfile
import tempfile
import urllib.request
from pathlib import Path, PurePosixPath

MANIFEST = Path(__file__).resolve().parents[1] / "wake-word-artifacts.json"


class ModelPreparationError(RuntimeError):
    """Sanitized model preparation failure."""


def identity(path: Path) -> tuple[int, str]:
    digest = hashlib.sha256()
    size = 0
    with path.open("rb") as source:
        for chunk in iter(lambda: source.read(1024 * 1024), b""):
            size += len(chunk)
            digest.update(chunk)
    return size, digest.hexdigest()


def verify_identity(path: Path, expected: dict, label: str) -> None:
    if not path.is_file():
        raise ModelPreparationError(f"missing required {label}")
    size, digest = identity(path)
    if size != expected["bytes"] or digest != expected["sha256"]:
        raise ModelPreparationError(f"{label} identity mismatch")


def safe_member(name: str) -> PurePosixPath:
    member = PurePosixPath(name)
    if member.is_absolute() or ".." in member.parts or not member.parts:
        raise ModelPreparationError("unsafe model archive member")
    return member


def load_model() -> dict:
    try:
        document = json.loads(MANIFEST.read_text(encoding="utf-8"))
        models = [item for item in document["artifacts"] if item.get("kind") == "kws-model"]
        if len(models) != 1:
            raise ModelPreparationError("artifact manifest must contain exactly one KWS model")
        return models[0]
    except ModelPreparationError:
        raise
    except Exception as exc:
        raise ModelPreparationError("invalid Wake Word artifact manifest") from exc


def install_root(root: Path, model: dict) -> Path:
    return root / "models" / "wake-word" / model["id"]


def verify_installed(root: Path, model: dict) -> Path:
    target = install_root(root, model)
    for item in model["files"]:
        verify_identity(target / item["name"], item, "Wake Word model file")
    verify_identity(
        target / model["keyword"]["filename"],
        model["keyword"],
        "Wake Word keyword file",
    )
    return target


def prepare_model(
    root: Path,
    model: dict,
    *,
    archive_path: Path | None = None,
    cache_dir: Path | None = None,
) -> Path:
    target = install_root(root, model)
    if target.exists():
        try:
            return verify_installed(root, model)
        except ModelPreparationError:
            shutil.rmtree(target)

    cache = cache_dir or root / ".cache" / "wake-word-model"
    cache.mkdir(parents=True, exist_ok=True)
    cached_archive = cache / model["archive"]["filename"]
    if archive_path is not None:
        shutil.copyfile(archive_path, cached_archive)
    elif not cached_archive.exists():
        try:
            with urllib.request.urlopen(model["source_url"], timeout=120) as response, cached_archive.open("wb") as out:
                shutil.copyfileobj(response, out, length=1024 * 1024)
        except Exception as exc:
            cached_archive.unlink(missing_ok=True)
            raise ModelPreparationError("failed to obtain pinned Wake Word model") from exc

    verify_identity(cached_archive, model["archive"], "Wake Word model archive")
    required = {item["name"]: item for item in model["files"]}

    with tempfile.TemporaryDirectory(prefix="wake-model-stage-") as temp_dir:
        stage = Path(temp_dir)
        try:
            with tarfile.open(cached_archive, mode="r:bz2") as archive:
                candidates: dict[str, list[tarfile.TarInfo]] = {
                    name: [] for name in required
                }
                for member in archive.getmembers():
                    safe_member(member.name)
                    if member.issym() or member.islnk():
                        raise ModelPreparationError("unsafe model archive member")
                    basename = PurePosixPath(member.name).name
                    if member.isfile() and basename in candidates:
                        candidates[basename].append(member)
                for name, item in required.items():
                    matches = candidates[name]
                    if len(matches) != 1:
                        raise ModelPreparationError(
                            "model archive required-file set is ambiguous"
                        )
                    source = archive.extractfile(matches[0])
                    if source is None:
                        raise ModelPreparationError(
                            "missing required Wake Word model file"
                        )
                    destination = stage / name
                    with source, destination.open("wb") as out:
                        shutil.copyfileobj(source, out, length=1024 * 1024)
                    verify_identity(destination, item, "Wake Word model file")
        except ModelPreparationError:
            raise
        except (OSError, tarfile.TarError) as exc:
            raise ModelPreparationError("invalid Wake Word model archive") from exc

        keyword = model["keyword"]
        keyword_path = stage / keyword["filename"]
        keyword_path.write_bytes((keyword["representation"] + "\n").encode("utf-8"))
        verify_identity(keyword_path, keyword, "Wake Word keyword file")

        target.parent.mkdir(parents=True, exist_ok=True)
        if target.exists():
            shutil.rmtree(target)
        shutil.move(str(stage), str(target))

    return verify_installed(root, model)


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--root", type=Path, default=Path("."))
    parser.add_argument("--archive", type=Path)
    parser.add_argument("--cache-dir", type=Path)
    parser.add_argument("--verify-only", action="store_true")
    args = parser.parse_args()

    try:
        model = load_model()
        if args.verify_only:
            path = verify_installed(args.root, model)
        else:
            path = prepare_model(
                args.root,
                model,
                archive_path=args.archive,
                cache_dir=args.cache_dir,
            )
        print(f"wake-word model ready: {model['id']} at {path.relative_to(args.root)}")
        return 0
    except ModelPreparationError as exc:
        print(f"wake-word model error: {exc}", file=__import__("sys").stderr)
        return 2


if __name__ == "__main__":
    raise SystemExit(main())
