#!/usr/bin/env python3
"""Fail closed if Local TTS packaging/provenance invariants drift.

This gate is intentionally model-weight-free. It validates checked-in catalog,
runtime, licensing, bundle, and repository policy only. Real KittenTTS artifacts
remain owned by the explicit production CPU acceptance workflow and the runtime
installer.
"""
from __future__ import annotations

import json
import re
import subprocess
import tomllib
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
CATALOG = ROOT / "src-tauri/src/ai/local_tts/manifest/catalog.rs"
MANIFEST = ROOT / "src-tauri/src/ai/local_tts/manifest.rs"
ENGINE = ROOT / "src-tauri/src/ai/local_tts/runtime/engine.rs"
CARGO = ROOT / "src-tauri/Cargo.toml"
TAURI_CONFIG = ROOT / "src-tauri/tauri.conf.json"
PACKAGE_JSON = ROOT / "package.json"
CI_WORKFLOW = ROOT / ".github/workflows/ci.yml"
FULL_VALIDATION_WORKFLOW = ROOT / ".github/workflows/full-validation-label.yml"
RELEASE_WORKFLOW = ROOT / ".github/workflows/release.yml"
ACCEPTANCE_WORKFLOW = ROOT / ".github/workflows/kittentts-production-cpu-acceptance.yml"
ASSET_LICENSES = ROOT / "docs/LOCAL_TTS_ASSET_LICENSES.md"
THIRD_PARTY_NOTICES = ROOT / "docs/THIRD_PARTY_NOTICES.md"
RELEASE_LICENSE_COLLECTOR = ROOT / "scripts/collect_release_licenses.py"

MODEL_REVISION = "c02725660cea441db4c383af69f1f26f5cd00947"
G2P_REVISION = "244ffeb44108347a514ebfc0c2f773d938c9613b"
ORT_VERSION = "1.23.2"
ORT_RELEASE = "v1.23.2"
ORT_CRATE_VERSION = "2.0.0-rc.13"
G2P_CRATE_VERSION = "0.4.0"
EXPECTED_PLATFORMS = {"LinuxX86_64", "MacosArm64", "MacosX86_64"}
EXPECTED_ORT_ARCHIVES = {
    "onnxruntime-linux-x64-1.23.2.tgz",
    "onnxruntime-osx-arm64-1.23.2.tgz",
    "onnxruntime-osx-x86_64-1.23.2.tgz",
}
FORBIDDEN_BUNDLE_TOKENS = (
    ".onnx",
    "voices.npz",
    "cmudict_data.json",
    "onnxruntime-linux-",
    "onnxruntime-osx-",
    "kitten-tts-mini",
)


def fail(message: str) -> None:
    raise SystemExit(f"check_local_tts_packaging_policy: {message}")


def require_token(text: str, token: str, source: str) -> None:
    if token not in text:
        fail(f"{source} is missing required token {token!r}")


def read_cargo_dependencies() -> dict:
    with CARGO.open("rb") as handle:
        return tomllib.load(handle).get("dependencies", {})


def check_dependency_features() -> None:
    dependencies = read_cargo_dependencies()

    ort = dependencies.get("ort")
    if not isinstance(ort, dict):
        fail("Cargo.toml must declare ort as an explicit dependency table")
    if ort.get("version") != f"={ORT_CRATE_VERSION}":
        fail(f"ort must remain pinned to ={ORT_CRATE_VERSION}")
    if ort.get("default-features") is not False:
        fail("ort default features must remain disabled")
    if set(ort.get("features", [])) != {"std", "api-23", "load-dynamic"}:
        fail("ort must remain app-owned/load-dynamic with the frozen API feature set")

    g2p = dependencies.get("piper-plus-g2p")
    if not isinstance(g2p, dict):
        fail("Cargo.toml must declare piper-plus-g2p as an explicit dependency table")
    if g2p.get("version") != f"={G2P_CRATE_VERSION}":
        fail(f"piper-plus-g2p must remain pinned to ={G2P_CRATE_VERSION}")
    if g2p.get("default-features") is not False:
        fail("piper-plus-g2p default features must remain disabled")
    if g2p.get("features") != ["english"]:
        fail("piper-plus-g2p must remain English-only without default/optional payloads")


def check_catalog_identity() -> None:
    text = CATALOG.read_text(encoding="utf-8")
    for token in (
        f'KITTEN_TTS_MODEL_REVISION: &str = "{MODEL_REVISION}"',
        f'KITTEN_TTS_G2P_SOURCE_REVISION: &str = "{G2P_REVISION}"',
        f'KITTEN_TTS_ORT_CRATE_VERSION: &str = "{ORT_CRATE_VERSION}"',
        f'KITTEN_TTS_ONNX_RUNTIME_VERSION: &str = "{ORT_VERSION}"',
        f'KITTEN_TTS_ONNX_RUNTIME_RELEASE: &str = "{ORT_RELEASE}"',
        f'KITTEN_TTS_G2P_CRATE_VERSION: &str = "{G2P_CRATE_VERSION}"',
        'KITTEN_TTS_INFERENCE_THREADS: u8 = 2',
        'KITTEN_TTS_RUNTIME_COMPATIBILITY_VERSION: u32 = 1',
        'adapter_contract: "talking-moose-kittentts-v1"',
    ):
        require_token(text, token, "Local TTS catalog")

    revisions = re.findall(r'\bsource_revision:\s*([^,]+),', text)
    if len(revisions) != 6:
        fail(f"expected six production artifact source revisions, found {len(revisions)}")

    urls = re.findall(r'\bsource_url:\s*"([^"]+)"', text)
    if len(urls) != 6:
        fail(f"expected six production artifact URLs, found {len(urls)}")
    for url in urls:
        if not url.startswith("https://"):
            fail(f"artifact URL is not HTTPS: {url}")
        if "huggingface.co/KittenML/kitten-tts-mini-0.8/resolve/" in url and MODEL_REVISION not in url:
            fail("KittenTTS Hugging Face URL is not pinned to the frozen model revision")
        if "raw.githubusercontent.com/ayutaz/piper-plus/" in url and G2P_REVISION not in url:
            fail("G2P data URL is not pinned to the frozen source revision")
        if "github.com/microsoft/onnxruntime/releases/download/" in url and f"/{ORT_RELEASE}/" not in url:
            fail("ONNX Runtime URL is not pinned to the frozen release")

    sha_values = re.findall(r'\bsha256:\s*"([0-9a-f]+)"', text)
    if len(sha_values) != 6 or any(len(value) != 64 for value in sha_values):
        fail("every production Local TTS artifact must carry one exact SHA-256")

    byte_values = [
        int(value.replace("_", ""))
        for value in re.findall(r'\bexpected_bytes:\s*([0-9_]+),', text)
    ]
    if len(byte_values) != 6 or any(value <= 0 for value in byte_values):
        fail("every production Local TTS artifact must carry a positive exact byte count")

    platforms = set(re.findall(r'platform:\s*LocalTtsPlatform::([A-Za-z0-9_]+)', text))
    if platforms != EXPECTED_PLATFORMS:
        fail(f"platform runtime archive coverage drifted: {sorted(platforms)!r}")

    archive_names = set(re.findall(r'filename:\s*"(onnxruntime-[^"]+\.tgz)"', text))
    if archive_names != EXPECTED_ORT_ARCHIVES:
        fail(f"ONNX Runtime archive set drifted: {sorted(archive_names)!r}")

    licenses = re.findall(r'\blicense:\s*"([^"]+)"', text)
    if licenses.count("Apache-2.0") < 3:
        fail("Kitten model/voice manifest license evidence must remain Apache-2.0")
    if licenses.count("MIT") != 3:
        fail("every platform ONNX Runtime archive must remain explicitly MIT licensed")
    if "BSD-style (CMU)" not in licenses:
        fail("CMUdict license evidence is missing from the Local TTS artifact catalog")


def check_platform_enum() -> None:
    text = MANIFEST.read_text(encoding="utf-8")
    require_token(
        text,
        "Self::LinuxX86_64, Self::MacosArm64, Self::MacosX86_64",
        "Local TTS platform enum",
    )
    require_token(text, "OnnxRuntimeArchive", "Local TTS artifact-kind enum")


def check_runtime_policy() -> None:
    text = ENGINE.read_text(encoding="utf-8")
    for token in (
        "with_execution_providers([ep::CPU::default().build()])",
        "with_intra_threads(manifest.runtime.inference_threads as usize)",
        ".with_inter_threads(1)",
        '"/lib/libonnxruntime.so.1.23.2"',
        '"/lib/libonnxruntime.1.23.2.dylib"',
        "ort::init_from(library_file.path())",
    ):
        require_token(text, token, "Local TTS runtime engine")
    lowered = text.lower()
    for forbidden in ("cudaexecutionprovider", "coremlexecutionprovider", "tensorrtexecutionprovider"):
        if forbidden in lowered:
            fail(f"Local TTS CPU baseline unexpectedly references {forbidden}")


def check_no_unapproved_espeak_or_gpl_payload() -> None:
    local_tts_root = ROOT / "src-tauri/src/ai/local_tts"
    for path in local_tts_root.rglob("*"):
        if not path.is_file():
            continue
        lowered_path = str(path.relative_to(ROOT)).lower()
        if "espeak" in lowered_path:
            fail(f"unapproved eSpeak payload is present: {path.relative_to(ROOT)}")
        if path.suffix.lower() in {".rs", ".json", ".toml", ".txt"}:
            text = path.read_text(encoding="utf-8", errors="ignore").lower()
            for forbidden in ("espeak-ng", "libespeak", "gnu general public license", "gpl-2", "gpl-3"):
                if forbidden in text:
                    fail(
                        "unapproved eSpeak/GPL Local TTS dependency or payload reference is present: "
                        f"{path.relative_to(ROOT)}"
                    )


def check_bundle_is_model_weight_free() -> None:
    config = json.loads(TAURI_CONFIG.read_text(encoding="utf-8"))
    bundle = config.get("bundle", {})
    serialized = json.dumps(bundle).lower()
    for token in FORBIDDEN_BUNDLE_TOKENS:
        if token in serialized:
            fail(f"Tauri bundle configuration embeds Local TTS runtime/model token {token!r}")

    resources = bundle.get("resources", [])
    if resources != ["native/macos/notices/"]:
        fail("ordinary Tauri resources must remain notice-only; Local TTS artifacts are installer-owned")

    tracked = subprocess.run(
        ["git", "ls-files"],
        cwd=ROOT,
        check=True,
        capture_output=True,
        text=True,
    ).stdout.splitlines()
    offenders = []
    for entry in tracked:
        lowered = entry.lower()
        name = Path(lowered).name
        if lowered.endswith(".onnx") or name in {"voices.npz", "cmudict_data.json"}:
            offenders.append(entry)
        elif name.startswith("onnxruntime-") and (name.endswith(".tgz") or name.endswith(".tar.gz")):
            offenders.append(entry)
    if offenders:
        fail("Local TTS model/runtime payloads must not be tracked by Git:\n" + "\n".join(offenders))


def check_license_evidence() -> None:
    if not ASSET_LICENSES.is_file():
        fail("Local TTS downloaded-asset license evidence document is missing")
    evidence = ASSET_LICENSES.read_text(encoding="utf-8")
    for token in (
        MODEL_REVISION,
        G2P_REVISION,
        ORT_RELEASE,
        "kitten_tts_mini_v0_8.onnx",
        "voices.npz",
        "cmudict_data.json",
        "Apache-2.0",
        "BSD-style (CMU)",
        "MIT",
        "not bundled",
        "piper-plus-g2p = 0.4.0",
        "ort = 2.0.0-rc.13",
        "bundled-dicts",
    ):
        require_token(evidence, token, "Local TTS asset license evidence")

    third_party = THIRD_PARTY_NOTICES.read_text(encoding="utf-8")
    for token in (
        "## Local TTS runtime and downloaded assets",
        "docs/LOCAL_TTS_ASSET_LICENSES.md",
        "piper-plus-g2p",
        "ort",
        "KittenTTS Mini 0.8 ONNX model",
        "CMUdict JSON",
        "ONNX Runtime CPU archive",
    ):
        require_token(third_party, token, "third-party notice inventory")

    collector = RELEASE_LICENSE_COLLECTOR.read_text(encoding="utf-8")
    for token in (
        '"ort": "2.0.0-rc.13"',
        '"piper-plus-g2p": "0.4.0"',
        'validate_required_cargo_rows(cargo, REQUIRED_LOCAL_TTS_CARGO, "Local TTS")',
    ):
        require_token(collector, token, "release license collector")


def check_workflow_enforcement() -> None:
    for path, label in (
        (CI_WORKFLOW, "ordinary CI"),
        (FULL_VALIDATION_WORKFLOW, "explicit full validation"),
        (RELEASE_WORKFLOW, "tagged release"),
    ):
        text = path.read_text(encoding="utf-8")
        require_token(text, "scripts/check_local_tts_packaging_policy.py", label)
        require_token(
            text,
            "python3 scripts/check_local_tts_packaging_policy.py",
            label,
        )

    ci = CI_WORKFLOW.read_text(encoding="utf-8")
    require_token(ci, "src-tauri/src/ai/local_tts/*", "ordinary CI release path classifier")


def check_ordinary_ci_is_model_weight_free() -> None:
    package = json.loads(PACKAGE_JSON.read_text(encoding="utf-8"))
    check_all = str(package.get("scripts", {}).get("check:all", "")).lower()
    for token in ("kitten_tts_mini_v0_8.onnx", "voices.npz", "onnxruntime-linux-x64"):
        if token in check_all:
            fail(f"npm run check:all references real Local TTS artifact {token!r}")

    ci = CI_WORKFLOW.read_text(encoding="utf-8").lower()
    for token in ("kitten_tts_mini_v0_8.onnx", "voices.npz", "huggingface.co/kittenml"):
        if token in ci:
            fail(f"ordinary CI references real Local TTS download token {token!r}")

    acceptance = ACCEPTANCE_WORKFLOW.read_text(encoding="utf-8")
    for token in (
        "KittenTTS production CPU acceptance",
        "Verify acceptance metadata matches the production catalog",
        "Download and verify frozen KittenTTS artifacts",
        "Run real Kitten Mini CPU acceptance",
        "timeout-minutes: 20",
    ):
        require_token(acceptance, token, "explicit KittenTTS acceptance workflow")


def main() -> None:
    check_dependency_features()
    check_catalog_identity()
    check_platform_enum()
    check_runtime_policy()
    check_no_unapproved_espeak_or_gpl_payload()
    check_bundle_is_model_weight_free()
    check_license_evidence()
    check_workflow_enforcement()
    check_ordinary_ci_is_model_weight_free()
    print(
        "local-tts-packaging-policy-ok "
        "targets=linux-x86_64,macos-arm64,macos-x86_64 "
        f"model_revision={MODEL_REVISION} ort={ORT_VERSION} weights=external cpu_only=true "
        "license_evidence=checked"
    )


if __name__ == "__main__":
    main()
