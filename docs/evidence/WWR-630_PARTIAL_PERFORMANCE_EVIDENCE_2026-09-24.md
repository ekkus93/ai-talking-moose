# WWR-630 — partial Wake Word performance evidence

Date: 2026-09-24
Evidence status: partial, not final acceptance
Exact merged master: `8329804e566fe8087959f058a34f2733805750bf`
Source PR: #438 (`test(wake): measure initialized idle KWS CPU`)

## Scope

This note records the WWR-630 performance measurements that are now objectively captured by the reusable real-KWS acceptance path. It does not close WWR-630 and must not be used as final performance acceptance.

## Exact-master validation

All evidence below is bound to exact merged master `8329804e566fe8087959f058a34f2733805750bf`.

- Ordinary CI: run `36036701116`, success.
- Wake Word source security audit: run `36036701147`, success.
- Wake Word real KWS acceptance: run `36036701173`, success.

`Wake Word real KWS acceptance` run `36036701173` completed both native platform jobs successfully and uploaded these privacy-safe report artifacts:

- `wake-word-real-kws-linux-x86_64`, artifact `10824062953`, 2245 bytes.
- `wake-word-real-kws-macos-arm64`, artifact `10825277636`, 2259 bytes.
- `wake-word-v1-corpus`, artifact `10824123383`, 532756 bytes.

## Measurements now recorded by the harness

The reusable platform report extraction now records, per Linux x86_64 and macOS arm64 real-KWS acceptance target:

- initialized idle KWS CPU utilization before feeding any corpus PCM;
- idle observation duration;
- active corpus inference CPU utilization, explicitly separate from idle CPU;
- peak resident memory converted to MiB;
- mean fixture inference latency;
- p95 fixture inference latency;
- total corpus audio duration and total corpus inference wall time;
- maximum real-time factor across generated fixtures;
- enforced one-thread inference policy;
- exact commit SHA, platform label, runner label, and measurement provenance.

The idle CPU measurement is intentionally captured after the verified native KWS session is initialized and before corpus PCM is fed. This avoids mislabeling active fixture inference CPU as idle wake-listening cost.

## WWR-630 items supported by this evidence

This evidence supports the WWR-630 subitems for representative Linux/macOS idle Wake Word CPU measurement, runtime memory overhead measurement, inference timing/real-time behavior measurement, and preservation of the one-thread policy.

## Remaining WWR-630 gaps

The canonical `docs/wake-word-performance-evidence.json` remains `pending_measurement`. The following WWR-630 requirements are still open:

- wake detection to command-ASR activation latency;
- pre-roll replay/startup timing;
- repeated-cycle resource behavior/delta;
- continuous full-ASR idle CPU comparison;
- accepted reproducible baseline across all required metrics;
- final demonstration that idle KWS is lighter than continuous full ASR.

WWR-950/960 final qualification must not treat this partial evidence as final WWR-630 acceptance.
