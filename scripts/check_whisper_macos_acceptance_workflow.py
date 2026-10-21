#!/usr/bin/env python3
"""Static safety contract for manual macOS Whisper Small real-model acceptance.

This script never downloads weights. It rejects accidental conversion of the
opt-in acceptance workflow into ordinary CI, loss of either architecture,
submodule/provenance checks, or substitution of compile-only for inference.
"""

from __future__ import annotations

import re
import textwrap
from pathlib import Path

WORKFLOW = (
    Path(__file__).resolve().parents[1]
    / ".github/workflows/whisper-macos-real-cpu-acceptance.yml"
)


def check_workflow(text: str) -> None:
    required = {
        "manual dispatch": "workflow_dispatch:",
        "arm64 target": "arch: arm64",
        "Intel target": "arch: x86_64",
        "arm64 runner": "runner: macos-15",
        "Intel runner": "runner: macos-15-intel",
        "pinned source checkout": "submodules: true",
        "exact-head verification": 'test "$(git rev-parse HEAD)" = "$GITHUB_SHA"',
        "source provenance": "python3 scripts/check_whisper_provenance.py",
        "hard native build requirement": 'TALKING_MOOSE_REQUIRE_WHISPER_NATIVE: "1"',
        "real acceptance feature": "--features whisper-acceptance --bin whisper_acceptance",
        "real corpus": "third_party/whisper.cpp/samples/jfk.wav",
        "native probe": '"$binary" probe-native',
        "production model install": '"$binary" install "$model" "$reports/install.json"',
        "macOS pipeline lifecycle tests": "asr::pipeline::tests -- --test-threads=1",
        "actual transcription": '"$binary" transcribe "$model" third_party/whisper.cpp/samples/jfk.wav "$reports/transcribe.json"',
        "production model deletion": '"$binary" delete "$model" "$reports/delete.json"',
        "installed model integrity": 'assert install["production_installer_verified"] is True',
        "model bytes": '487601967',
        "model hash": "1be3a9b2063867b937e64e2ec7483364a79917e157fa98c5d94b5c1fffea987b",
        "source revision": "60c0be6ac8fa71b1a2ae2dd938a31a34a508e774",
        "genuine transcription": 'assert transcript["transcript"]["transcription_ok"] is True',
        "meaningful words": '("ask", "country", "you")',
        "runtime identity": 'assert transcript["runtime"]["whisper_native_linked"] is True',
        "architecture evidence": 'assert transcript["host_arch"] == rust_arch',
        "JSON evidence upload": "actions/upload-artifact@v4",
        "no model weight artifacts": "/reports/*.json",
    }
    for description, snippet in required.items():
        if snippet not in text:
            raise ValueError(f"Mac Whisper acceptance missing {description}: {snippet}")

    for event in ("push", "pull_request", "schedule", "repository_dispatch"):
        if re.search(rf"^\s*{event}\s*:", text, flags=re.MULTILINE):
            raise ValueError(f"Mac real-model workflow must not trigger on {event}")
    if text.count("arch: arm64") != 1 or text.count("arch: x86_64") != 1:
        raise ValueError("Mac acceptance matrix must contain exactly one job per architecture")
    if "sudo unshare --net" in text:
        raise ValueError("Mac workflow must not claim Linux namespace-based egress isolation")

    # macOS uses a separate Seatbelt policy, not Linux network namespaces.
    # Protect both *successful* real-model transcription runs: no bare CLI
    # run, unprotected repeat, socket-only probe, or JSON assertion without
    # a required enforced network boundary can count as offline acceptance.
    # Inspect the actual executable heredoc body, not arbitrary text.
    # A comment or disconnected copy of "(deny network*)" must not make a
    # network-permissive Seatbelt profile pass this policy check.
    profile_start = re.compile(
        r"""^[ \t]*cat > "\$sandbox_profile" <<'SEATBELT'[ \t]*$""",
        flags=re.MULTILINE,
    )
    starts = list(profile_start.finditer(text))
    if len(starts) != 1:
        raise ValueError("macOS Seatbelt profile must be defined exactly once")
    remainder = text[starts[0].end() :]
    profile_end = re.search(r"^[ \t]*SEATBELT[ \t]*$", remainder, flags=re.MULTILINE)
    if profile_end is None:
        raise ValueError("macOS Seatbelt profile heredoc is not terminated")
    profile_lines = [
        line.strip()
        for line in remainder[: profile_end.start()].splitlines()
        if line.strip()
    ]
    if profile_lines != ["(version 1)", "(allow default)", "(deny network*)"]:
        raise ValueError("macOS Seatbelt profile must enforce network denial")

    required_sandbox = {
        "explicit sandbox availability": "command -v sandbox-exec",
        "preflight under sandbox": 'sandbox-exec -f "$sandbox_profile" python3 -c',
        "permission-denied preflight": "errno.EPERM, errno.EACCES",
        "first inference under sandbox": (
            'sandbox-exec -f "$sandbox_profile" "$binary" transcribe '
            '"$model" third_party/whisper.cpp/samples/jfk.wav '
            '"$reports/transcribe.json" --require-network-denied'
        ),
        "repeated inference under sandbox": (
            'sandbox-exec -f "$sandbox_profile" "$binary" transcribe '
            '"$model" third_party/whisper.cpp/samples/jfk.wav '
            '"$reports/transcribe-repeat.json" --require-network-denied'
        ),
        "first probe required": 'assert transcript["network_denied_required"] is True',
        "first probe passed": 'assert transcript["network_denial_probe_passed"] is True',
        "repeat probe required": 'assert repeated["network_denied_required"] is True',
        "repeat probe passed": 'assert repeated["network_denial_probe_passed"] is True',
    }
    for description, snippet in required_sandbox.items():
        if snippet not in text:
            raise ValueError(f"Mac offline qualification missing {description}: {snippet}")
    if text.count("--require-network-denied") != 2:
        raise ValueError("Only the two sandboxed genuine-model transcriptions may require denial")

    # Match the executable shell commands, not arbitrary substrings. A
    # commented copy of a sandboxed command must never authorize an actual
    # unprotected model-backed transcription. Keep the two intentionally
    # unprotected *negative* missing/corrupt-model probes fail-closed.
    expected_transcriptions = {
        'if "$binary" transcribe "$model" third_party/whisper.cpp/samples/jfk.wav "$reports/missing-model.json" > "$root/missing-model.log" 2>&1; then',
        'if "$binary" transcribe "$model" third_party/whisper.cpp/samples/jfk.wav "$reports/corrupt-model.json" > "$root/corrupt-model.log" 2>&1; then',
        'if ! sandbox-exec -f "$sandbox_profile" "$binary" transcribe "$model" third_party/whisper.cpp/samples/jfk.wav "$reports/transcribe.json" --require-network-denied > "$native_log" 2>&1; then',
        'if ! sandbox-exec -f "$sandbox_profile" "$binary" transcribe "$model" third_party/whisper.cpp/samples/jfk.wav "$reports/transcribe-repeat.json" --require-network-denied > "$repeat_log" 2>&1; then',
    }
    active_transcriptions = [
        line.strip()
        for line in text.splitlines()
        if '"$binary" transcribe ' in line and not line.lstrip().startswith("#")
    ]
    if (
        len(active_transcriptions) != len(expected_transcriptions)
        or set(active_transcriptions) != expected_transcriptions
    ):
        raise ValueError(
            "macOS Whisper active transcription commands must preserve the "
            "two guarded negative probes and two Seatbelt-protected real runs"
        )

    if re.search(r"^\s*path:.*(?:ggml-small[.]bin|/models/)", text, re.MULTILINE):
        raise ValueError("Mac acceptance must not upload model weights")

    # Compile the embedded JSON assertion program before any costly model run.
    lines = text.splitlines()
    blocks = []
    for index, line in enumerate(lines):
        if "<<'PY'" not in line:
            continue
        end = next(
            (n for n in range(index + 1, len(lines)) if lines[n].strip() == "PY"),
            None,
        )
        if end is None:
            raise ValueError("Unterminated Mac acceptance Python heredoc")
        blocks.append(textwrap.dedent("\n".join(lines[index + 1 : end])) + "\n")
    if not blocks:
        raise ValueError("Mac acceptance requires executable JSON assertions")
    for index, source in enumerate(blocks, start=1):
        compile(source, f"mac-whisper-acceptance-{index}", "exec")



def check_precutover_app_workflow(text: str, native_compile: str, main_source: str) -> None:
    """Keep diagnostic packaging native, two-arch, and explicitly non-release."""
    required = {
        "manual trigger": "workflow_dispatch:",
        "Apple Silicon": "arch: arm64",
        "Intel": "arch: x86_64",
        "native source identity": "python3 scripts/check_whisper_provenance.py",
        "required native linkage": 'TALKING_MOOSE_REQUIRE_WHISPER_NATIVE: "1"',
        "test-only stripped bundle": """--config '{"bundle":{"macOS":{"frameworks":[]},"resources":[]}}'""",
        "real executable native probe": "--whisper-native-smoke-test",
        "app build provenance": "scripts/verify_app_build_provenance.sh",
        "read declared app executable": "Print :CFBundleExecutable",
        "use declared app executable": 'bin="$app/Contents/MacOS/$bundle_executable"',
        "actual Mach-O dependencies": 'otool -L "$bin"',
        "no downloaded model in app": "-name 'ggml-small.bin'",
        "no Moonshine runtime in diagnostic app": "-name 'libmoonshine.dylib'",
    }
    for description, needle in required.items():
        if needle not in text:
            raise ValueError(f"Pre-cutover app smoke missing {description}: {needle}")

    for forbidden in ("actions/upload-artifact@", "notarytool", "--bundles dmg"):
        if forbidden in text:
            raise ValueError(f"Pre-cutover diagnostic must not publish a release: {forbidden}")
    if '-type f -perm -111 -print -quit' in text:
        raise ValueError("Pre-cutover smoke must select the Info.plist application executable")
    if text.count("arch: arm64") != 1 or text.count("arch: x86_64") != 1:
        raise ValueError("Pre-cutover smoke must run once per Mac architecture")
    if r"\\.dylib" in text or r"\\.dylib" in native_compile:
        raise ValueError("Dynamic-library audit is overescaped and might silently pass")
    if r"\.dylib" not in text or r"\.dylib" not in native_compile:
        raise ValueError("Dynamic-library audit must match literal .dylib suffixes")
    if "--whisper-native-smoke-test" not in main_source:
        raise ValueError("Packaged app executable lacks real native Whisper probe")
    if "whisper::native_probe::runtime_version()" not in main_source:
        raise ValueError("Packaged app smoke does not invoke real Whisper FFI")


def main() -> int:
    check_workflow(WORKFLOW.read_text(encoding="utf-8"))
    root = WORKFLOW.parents[2]
    check_precutover_app_workflow(
        (root / ".github/workflows/whisper-macos-precutover-app.yml").read_text(encoding="utf-8"),
        (root / ".github/workflows/whisper-macos-native-compile.yml").read_text(encoding="utf-8"),
        (root / "src-tauri/src/main.rs").read_text(encoding="utf-8"),
    )
    print("macOS Whisper real-model acceptance workflow policy OK")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
