#!/usr/bin/env python3
"""Fail closed if Local LLM packaging/CI invariants drift.

This gate is intentionally model-weight-free. It validates the checked-in build and
packaging configuration only; real GGUF acceptance remains a separately dispatched
workflow.
"""
from __future__ import annotations

import json
import re
import subprocess
import tomllib
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
MAIN_CARGO = ROOT / "src-tauri/Cargo.toml"
PROOF_CARGO = ROOT / "src-tauri/llama-compile-proof/Cargo.toml"
CI_WORKFLOW = ROOT / ".github/workflows/ci.yml"
RELEASE_WORKFLOW = ROOT / ".github/workflows/release.yml"
TAURI_CONFIG = ROOT / "src-tauri/tauri.conf.json"
PACKAGE_JSON = ROOT / "package.json"
CATALOG = ROOT / "src-tauri/src/ai/local/catalog.rs"
MODEL_LICENSES = ROOT / "docs/LOCAL_LLM_MODEL_LICENSES.md"
LOCAL_LLM_NOTICE = ROOT / "src-tauri/native/macos/notices/LocalLlmRuntime/README.md"

LLAMA_VERSION = "=0.1.154"
LLAMA_CPP_RS_COMMIT = "bed81ad4ab1a6c904b11d425608e50f976d8ea62"
LLAMA_CPP_NATIVE_COMMIT = "5f55650a78f92aff4d48d671423e888fac0469ff"
REQUIRED_COMPILE_LABELS = {"linux-x86_64", "macos-arm64", "macos-x86_64"}
FORBIDDEN_ORDINARY_CI_TOKENS = (
    ".gguf",
    "local_llm_acceptance",
    "local-llm-real-cpu-acceptance",
    "huggingface.co/",
)
MODEL_IDS = (
    "smollm2-360m-instruct-q4-k-m",
    "qwen3-0-6b-instruct-q4-k-m",
)


def fail(message: str) -> None:
    raise SystemExit(f"check_local_llm_packaging_policy: {message}")


def read_toml(path: Path) -> dict:
    with path.open("rb") as handle:
        return tomllib.load(handle)


def require_llama_dependency(table: dict, name: str, source: str) -> None:
    value = table.get(name)
    if not isinstance(value, dict):
        fail(f"{source} must declare {name} as an explicit dependency table")
    if value.get("version") != LLAMA_VERSION:
        fail(f"{source} must pin {name} to {LLAMA_VERSION}")
    if value.get("default-features") is not False:
        fail(f"{source} must disable {name} default features for the CPU baseline")


def check_llama_pins() -> None:
    main = read_toml(MAIN_CARGO).get("dependencies", {})
    proof = read_toml(PROOF_CARGO).get("dependencies", {})
    for name in ("llama-cpp-2", "llama-cpp-sys-2"):
        require_llama_dependency(main, name, "application Cargo.toml")
        require_llama_dependency(proof, name, "compile-proof Cargo.toml")


def check_compile_matrix() -> None:
    text = CI_WORKFLOW.read_text(encoding="utf-8")
    if "local-llm-compile-proof:" not in text:
        fail("ordinary CI is missing the Local LLM compile-proof job")
    labels = set(re.findall(r"^\s*- label:\s*([^\s#]+)\s*$", text, flags=re.MULTILINE))
    missing = REQUIRED_COMPILE_LABELS - labels
    if missing:
        fail(f"ordinary CI compile matrix is missing: {', '.join(sorted(missing))}")
    command = "cargo test --manifest-path src-tauri/llama-compile-proof/Cargo.toml --locked"
    if command not in text:
        fail("ordinary CI no longer runs the locked llama.cpp compile/CPU-policy proof")
    if re.search(r"(?:brew|apt(?:-get)?)\s+.*install.*llama", text, flags=re.IGNORECASE):
        fail("ordinary CI must not install a developer/system llama.cpp package")


def check_ordinary_ci_is_model_weight_free() -> None:
    package = json.loads(PACKAGE_JSON.read_text(encoding="utf-8"))
    check_all = str(package.get("scripts", {}).get("check:all", ""))
    if not check_all:
        fail("package.json is missing scripts.check:all")
    lowered = check_all.lower()
    for token in FORBIDDEN_ORDINARY_CI_TOKENS:
        if token in lowered:
            fail(f"npm run check:all references real-model acceptance token {token!r}")

    for path in (CI_WORKFLOW, RELEASE_WORKFLOW):
        text = path.read_text(encoding="utf-8").lower()
        for token in FORBIDDEN_ORDINARY_CI_TOKENS:
            if token in text:
                fail(f"{path.relative_to(ROOT)} references forbidden model-weight token {token!r}")

    runtime_tests = (ROOT / "src-tauri/src/ai/local/runtime/tests.rs").read_text(encoding="utf-8")
    residual_tests = (ROOT / "src-tauri/src/ai/local/runtime/residual_tests.rs").read_text(
        encoding="utf-8"
    )
    text_model = (ROOT / "src-tauri/src/ai/local/text_model.rs").read_text(encoding="utf-8")
    if "struct FakeEngine" not in runtime_tests:
        fail("runtime unit tests no longer expose explicit FakeEngine injection")
    if "struct CountingEngine" not in residual_tests:
        fail("runtime residual tests no longer use an explicit injected engine")
    if "LocalGenerationRuntime" not in text_model or "from_runtime" not in text_model:
        fail("LocalTextModel unit tests no longer have explicit runtime injection")


def check_bundle_configuration() -> None:
    config = json.loads(TAURI_CONFIG.read_text(encoding="utf-8"))
    bundle = config.get("bundle", {})
    serialized = json.dumps(bundle).lower()
    if ".gguf" in serialized:
        fail("Tauri bundle configuration must not embed GGUF files")
    if re.search(r"(?:^|[/\\])models?(?:[/\\]|$)", serialized):
        fail("Tauri bundle configuration must not embed a model directory")
    resources = bundle.get("resources", [])
    if resources != ["native/macos/notices/"]:
        fail("Tauri bundle resources must remain notice-only for Local LLM V1")

    tracked = subprocess.run(
        ["git", "ls-files", "*.gguf", "*.GGUF"],
        cwd=ROOT,
        check=True,
        capture_output=True,
        text=True,
    ).stdout.strip()
    if tracked:
        fail(f"GGUF model weights are tracked by Git:\n{tracked}")

    gitignore = (ROOT / ".gitignore").read_text(encoding="utf-8")
    if "*.gguf" not in gitignore or "*.GGUF" not in gitignore:
        fail(".gitignore must reject lower- and upper-case GGUF model weights")


def check_native_runtime_notice() -> None:
    if not LOCAL_LLM_NOTICE.is_file():
        fail("Local LLM native runtime notice metadata is missing")
    notice = LOCAL_LLM_NOTICE.read_text(encoding="utf-8")
    for required in (
        "llama-cpp-2 = 0.1.154",
        "llama-cpp-sys-2 = 0.1.154",
        LLAMA_CPP_RS_COMMIT,
        LLAMA_CPP_NATIVE_COMMIT,
        "LLAMA_CPP_LICENSE",
        "LLAMA_CPP_RS_LICENSE_MIT",
    ):
        if required not in notice:
            fail(f"Local LLM native runtime notice is missing provenance token {required!r}")


def check_model_license_document() -> None:
    if not MODEL_LICENSES.is_file():
        fail("model catalog licenses must be documented separately from code dependencies")
    catalog = CATALOG.read_text(encoding="utf-8")
    document = MODEL_LICENSES.read_text(encoding="utf-8")
    if document.count("Apache-2.0") < len(MODEL_IDS):
        fail("model license document does not record Apache-2.0 for every catalog entry")
    if "not bundled" not in document.lower() or "download" not in document.lower():
        fail("model license document must state that GGUF weights are downloaded, not bundled")
    for model_id in MODEL_IDS:
        if model_id not in catalog:
            fail(f"expected catalog model is missing: {model_id}")
        if model_id not in document:
            fail(f"model license document is missing catalog ID: {model_id}")

    catalog_licenses = re.findall(r'\blicense:\s*"([^"]+)"', catalog)
    if catalog_licenses != ["Apache-2.0", "Apache-2.0"]:
        fail(f"unexpected Local LLM catalog license set: {catalog_licenses!r}")


def main() -> None:
    check_llama_pins()
    check_compile_matrix()
    check_ordinary_ci_is_model_weight_free()
    check_bundle_configuration()
    check_native_runtime_notice()
    check_model_license_document()
    print(
        "local-llm-packaging-policy-ok "
        "targets=linux-x86_64,macos-arm64,macos-x86_64 "
        "weights=external runtime=llama-cpp-2@0.1.154"
    )


if __name__ == "__main__":
    main()
