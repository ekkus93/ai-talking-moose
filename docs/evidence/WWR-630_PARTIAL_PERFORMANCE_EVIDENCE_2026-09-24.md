# WWR-630 — partial Wake Word performance evidence

Date: 2026-09-24
Evidence status: partial, not final acceptance
Latest evidence master: `34ad6fafa71e73903bd16af81b2d6015fd4d3f59`
Earlier measurement master: `8329804e566fe8087959f058a34f2733805750bf`
Source PRs: #438 (`test(wake): measure initialized idle KWS CPU`), #443 (`feat(wake): measure command activation timing`), #444 (`docs(wake): record WWR-630 activation timing evidence`)

## Scope

This note records the WWR-630 performance measurements that are now objectively captured by the reusable real-KWS acceptance path and the command-activation timing seam. It does not close WWR-630 and must not be used as final performance acceptance.

## Exact-master validation

The real-KWS platform measurement values below are bound to exact merged master `8329804e566fe8087959f058a34f2733805750bf`.

- Ordinary CI: run `36036701116`, success.
- Wake Word source security audit: run `36036701147`, success.
- Wake Word real KWS acceptance: run `36036701173`, success.

`Wake Word real KWS acceptance` run `36036701173` completed both native platform jobs successfully and uploaded these privacy-safe report artifacts:

- `wake-word-real-kws-linux-x86_64`, artifact `10824062953`, 2245 bytes.
- `wake-word-real-kws-macos-arm64`, artifact `10825277636`, 2259 bytes.
- `wake-word-v1-corpus`, artifact `10824123383`, 532756 bytes.

The command-activation timing seam is bound to exact merged master `2d35a6662f3ed6c01d526bc438676b9ed1ce1971` and evidence master `34ad6fafa71e73903bd16af81b2d6015fd4d3f59`:

- PR #443 exact head `a4cd0ffd636536b5b2a422cdc757ce7422cf5050`: ordinary CI `36063972014`, lifecycle stability `36063972064`, and source-security audit `36063972020` passed.
- PR #443 exact merged master `2d35a6662f3ed6c01d526bc438676b9ed1ce1971`: ordinary CI `36067453325`, lifecycle stability `36067453316`, and source-security audit `36067453264` passed.
- PR #444 exact merged master `34ad6fafa71e73903bd16af81b2d6015fd4d3f59`: ordinary CI `36068959547` passed.

## Measured platform values recovered from real-KWS logs

### Linux x86_64

Source: Wake Word real KWS acceptance run `36036701173`, job `107758609860`, `Extract reusable performance measurement` step.

```json
{
  "platform": "linux-x86_64",
  "commit_sha": "8329804e566fe8087959f058a34f2733805750bf",
  "runner": "GitHub Actions 1000171442",
  "metrics": {
    "corpus_active_cpu_percent": 102.591,
    "idle_cpu_percent": 0.1,
    "idle_observation_ms": 2000,
    "runtime_memory_mib": 60.625,
    "inference_latency_ms": 68.929,
    "inference_p95_ms": 99,
    "corpus_audio_ms": 30178,
    "corpus_inference_wall_ms": 965,
    "max_real_time_factor": 0.31884984025559104,
    "inference_threads": 1
  }
}
```

### macOS arm64

Source: Wake Word real KWS acceptance run `36036701173`, job `107758609893`, `Extract reusable performance measurement` step.

```json
{
  "platform": "macos-arm64",
  "commit_sha": "8329804e566fe8087959f058a34f2733805750bf",
  "runner": "GitHub Actions 1000171443",
  "metrics": {
    "corpus_active_cpu_percent": 94.044,
    "idle_cpu_percent": 0.0,
    "idle_observation_ms": 2037,
    "runtime_memory_mib": 77.922,
    "inference_latency_ms": 62.357,
    "inference_p95_ms": 103,
    "corpus_audio_ms": 30178,
    "corpus_inference_wall_ms": 873,
    "max_real_time_factor": 0.2702875399361022,
    "inference_threads": 1
  }
}
```

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

The command-activation timing seam now records bounded privacy-safe metadata for:

- wake handoff to command-ASR ingress timing (`wake_to_command_asr_ms`);
- normal command-start timing (`command_start_ms`);
- total activation wrapper timing (`total_activation_ms`).

## WWR-630 items supported by this evidence

This evidence supports the WWR-630 subitems for representative Linux/macOS idle Wake Word CPU measurement, runtime memory overhead measurement, inference timing/real-time behavior measurement, the command-activation timing instrumentation path, and preservation of the one-thread policy.

## Remaining WWR-630 gaps

The canonical `docs/wake-word-performance-evidence.json` remains `pending_measurement`. The following WWR-630 requirements are still open:

- accepted command-activation timing values in the final cross-platform baseline;
- accepted pre-roll replay/startup timing values in the final cross-platform baseline;
- repeated-cycle resource behavior/delta;
- continuous full-ASR idle CPU comparison;
- accepted reproducible baseline across all required metrics;
- final demonstration that idle KWS is lighter than continuous full ASR.

WWR-950/960 final qualification must not treat this partial evidence as final WWR-630 acceptance.
