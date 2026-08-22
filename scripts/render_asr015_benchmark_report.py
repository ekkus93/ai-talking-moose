#!/usr/bin/env python3
from __future__ import annotations

import argparse
import json
import os
import pathlib
import statistics
from collections import defaultdict


def mib(value: int | None) -> str:
    return "n/a" if value is None else f"{value / (1024 * 1024):.1f}"


def number(value: float | int | None, digits: int = 3) -> str:
    if value is None:
        return "n/a"
    if isinstance(value, int):
        return str(value)
    return f"{value:.{digits}f}"


def median(records: list[dict], key: str) -> float:
    return float(statistics.median(float(record[key]) for record in records))


def maximum(records: list[dict], key: str) -> float:
    return max(float(record[key]) for record in records)


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--records", type=pathlib.Path, required=True)
    parser.add_argument("--hardware", type=pathlib.Path, required=True)
    parser.add_argument("--corpus", type=pathlib.Path, required=True)
    parser.add_argument("--output", type=pathlib.Path, required=True)
    parser.add_argument("--commit", required=True)
    args = parser.parse_args()

    records = [
        json.loads(line)
        for line in args.records.read_text(encoding="utf-8").splitlines()
        if line.strip()
    ]
    hardware = json.loads(args.hardware.read_text(encoding="utf-8"))
    corpus = json.loads(args.corpus.read_text(encoding="utf-8"))
    grouped: dict[str, list[dict]] = defaultdict(list)
    for record in records:
        grouped[record["architecture"]].append(record)

    expected_architectures = {"tiny_streaming", "small_streaming"}
    if set(grouped) != expected_architectures:
        raise SystemExit(f"missing benchmark architecture records: {set(grouped)}")
    for architecture, architecture_records in grouped.items():
        phases = [record["phase"] for record in architecture_records]
        if phases.count("warmup") != 1 or phases.count("measured") != 5:
            raise SystemExit(f"{architecture} does not contain 1 warmup + 5 measured runs")
        for record in architecture_records:
            if record.get("dropped_chunks") != 0:
                raise SystemExit(f"{architecture} run dropped bounded-ingress audio")
            if record.get("last_error") is not None:
                raise SystemExit(f"{architecture} run reported a typed ASR error")
            rtf = record.get("real_time_factor")
            if rtf is None or float(rtf) >= 1.0:
                raise SystemExit(f"{architecture} run did not sustain RTF < 1.0")
            if record.get("first_partial_latency_ms") is None or record.get("first_final_latency_ms") is None:
                raise SystemExit(f"{architecture} run did not emit both useful partial and final transcripts")
            if not str(record.get("final_transcript", "")).strip():
                raise SystemExit(f"{architecture} run has no useful final transcript text")

    run_id = os.environ.get("GITHUB_RUN_ID")
    repository = os.environ.get("GITHUB_REPOSITORY")
    run_attempt = os.environ.get("GITHUB_RUN_ATTEMPT")
    run_identity: list[str] = []
    if run_id and repository:
        server_url = os.environ.get("GITHUB_SERVER_URL", "https://github.com").rstrip("/")
        run_identity.append(
            f"- GitHub Actions run: `{server_url}/{repository}/actions/runs/{run_id}`"
        )
        if run_attempt:
            run_identity.append(f"- Run attempt: `{run_attempt}`")

    lines = [
        "# ASR-015 supported-Mac native acceptance report",
        "",
        "Status: **PASS**",
        "",
        "## Reference hardware",
        "",
        f"- Hardware model: `{hardware['hardware_model']}`",
        f"- CPU/chip: `{hardware['cpu_brand']}`",
        f"- Physical/logical CPU count: {hardware['physical_cpu_count']} / {hardware['logical_cpu_count']}",
        f"- RAM: {mib(hardware['memory_bytes'])} MiB",
        f"- macOS: {hardware['macos_version']} ({hardware['macos_build']})",
        f"- Architecture: `{hardware['architecture']}`",
        f"- Low-power mode: `{hardware['low_power_mode']}`",
        f"- Talking Moose commit: `{args.commit}`",
        *run_identity,
        "",
        "This is the minimum **measured** CPU reference established by this acceptance run. "
        "No slower Mac CPU is claimed supported by this evidence.",
        "",
        "## Corpus",
        "",
        f"- Source: `ggml-org/whisper.cpp/{corpus['source_path']}` at `{corpus['source_commit']}`",
        f"- Source Git blob: `{corpus['source_git_blob_sha1']}`",
        f"- Source SHA-256: `{corpus['source_sha256']}`",
        f"- Derived PCM SHA-256: `{corpus['corpus_sha256']}`",
        f"- Format: {corpus['sample_rate_hz']} Hz mono signed 16-bit little-endian PCM",
        f"- Duration: {corpus['corpus_duration_ms'] / 1000:.1f} s "
        f"({corpus['source_duration_ms'] / 1000:.1f} s speech + "
        f"{corpus['trailing_silence_ms'] / 1000:.1f} s trailing silence)",
        "",
    ]

    for architecture in ("tiny_streaming", "small_streaming"):
        architecture_records = grouped[architecture]
        measured = [record for record in architecture_records if record["phase"] == "measured"]
        lines.extend(
            [
                f"## {architecture.replace('_', ' ').title()}",
                "",
                "| Run | RTF | First partial ms | First final ms | Native latency ms | CPU % | Peak RSS MiB | Peak Δ MiB | Drops | Final transcript |",
                "| --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | --- |",
            ]
        )
        for record in architecture_records:
            peak_delta = record["peak_resident_memory_bytes"] - record["baseline_resident_memory_bytes"]
            run_label = "warm-up" if record["phase"] == "warmup" else f"measured {record['run']}"
            transcript = record["final_transcript"].replace("|", "\\|")
            lines.append(
                "| "
                + " | ".join(
                    [
                        run_label,
                        number(record["real_time_factor"]),
                        number(record["first_partial_latency_ms"], 0),
                        number(record["first_final_latency_ms"], 0),
                        number(record["last_transcription_latency_ms"], 0),
                        number(record["average_cpu_utilization_percent"], 1),
                        mib(record["peak_resident_memory_bytes"]),
                        mib(peak_delta),
                        str(record["dropped_chunks"]),
                        transcript,
                    ]
                )
                + " |"
            )
        lines.extend(
            [
                "",
                f"Measured median RTF: **{median(measured, 'real_time_factor'):.3f}**; "
                f"worst RTF: **{maximum(measured, 'real_time_factor'):.3f}**.",
                f"Measured median first-final latency: **{median(measured, 'first_final_latency_ms'):.0f} ms**; "
                f"worst: **{maximum(measured, 'first_final_latency_ms'):.0f} ms**.",
                f"Highest sampled RSS across measured runs: **{mib(int(maximum(measured, 'peak_resident_memory_bytes')))} MiB**.",
                "",
            ]
        )

    lines.extend(
        [
            "## Acceptance conclusion",
            "",
            "Tiny and Small both completed one warm-up and five measured real native streaming runs with:",
            "",
            "- a useful partial and final transcript on every run;",
            "- zero bounded-ingress drops;",
            "- no typed ASR error;",
            "- measured RTF strictly below `1.0` on every run; and",
            "- CPU and process-RSS metrics recorded from the production local-ASR pipeline.",
            "",
            f"The measured reference CPU is **{hardware['cpu_brand']} ({hardware['hardware_model']})**. "
            "This report does not claim support for slower CPU models without the same measured gate.",
            "",
        ]
    )

    args.output.parent.mkdir(parents=True, exist_ok=True)
    args.output.write_text("\n".join(lines), encoding="utf-8")
    print(f"ASR015_ACCEPTANCE=PASS report={args.output}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
