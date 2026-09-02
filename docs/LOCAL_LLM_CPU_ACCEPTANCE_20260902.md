# Local LLM Real CPU Acceptance — 2026-09-02

**P12 status: COMPLETE.**

## Canonical acceptance

- Workflow: `Local LLM Real CPU Acceptance`
- Run: `33663595759`
- Run URL: <https://github.com/ekkus93/ai-talking-moose/actions/runs/33663595759>
- Event: `workflow_dispatch`
- Branch: `master`
- Accepted source SHA: `28aef16cbeeb91d9570177111560158811730b89`
- Artifact ID: `9859917109`
- Artifact name: `local-llm-real-cpu-28aef16cbeeb91d9570177111560158811730b89`
- Artifact ZIP SHA-256: `b9e075822025dd98deac9e161d6f18281860148d13622e870a1ce75cc1c76b80`
- Host: Linux x86_64, AMD EPYC 9V74 80-Core Processor
- Runtime-reported available parallelism: 4

The job completed successfully through production installer verification, isolated CPU generation with network denied, independent machine-readable evidence validation, and artifact upload. Generation ran only after the workflow entered an isolated network namespace. Both model reports record `network_denial_probe_passed = true`.

The pinned runtime does not expose first-token timing separately. The reports therefore record `first_token_latency_ms = null` with an explicit note that no value is fabricated.

## SmolLM2-360M-Instruct Q4_K_M

Identity and install evidence:

- model ID: `smollm2-360m-instruct-q4-k-m`
- revision: `ab928a97ee49f3a015f35194879f68211291d6ca`
- artifact: `SmolLM2-360M-Instruct-Q4_K_M.gguf`
- SHA-256: `2fa3f013dcdd7b99f9b237717fa0b12d75bbb89984cc1274be1471a465bac9c2`
- expected bytes: 270,590,880
- installed bytes: 270,590,880
- quantization: `Q4_K_M`
- license: `Apache-2.0`
- production installer verified: yes

Measured generation evidence:

- cold probe wall time: 454 ms
- warm probe wall time: 328 ms
- cold-load estimate: 126 ms
- RSS before: 24,592,384 bytes
- RSS after cold probe: 336,408,576 bytes
- RSS delta: 311,816,192 bytes (~297.4 MiB)
- ambient request cap: 60 tokens
- ambient generation: 937 ms
- ambient output: 34 tokens
- measured throughput: 36.255314 tokens/s
- ambient output non-empty: yes
- owner-drop/reload wall time: 452 ms
- owner-drop/reload success: yes

## Qwen3-0.6B Q4_K_M, non-thinking

Identity and install evidence:

- model ID: `qwen3-0-6b-instruct-q4-k-m`
- revision: `7bcae0bc7b0606f1e948f8cdb31b98a2c10635db`
- artifact: `Qwen_Qwen3-0.6B-Q4_K_M.gguf`
- SHA-256: `9acfc1e001311f34b4252001b626f2e466d592a42065f66571bff3790d4e1b14`
- expected bytes: 484,220,320
- installed bytes: 484,220,320
- quantization: `Q4_K_M`
- license: `Apache-2.0`
- production installer verified: yes

Measured generation evidence:

- cold probe wall time: 1,015 ms
- warm probe wall time: 545 ms
- cold-load estimate: 470 ms
- RSS before: 24,453,120 bytes
- RSS after cold probe: 772,386,816 bytes
- RSS delta: 747,933,696 bytes (~713.3 MiB)
- ambient request cap: 60 tokens
- ambient generation: 986 ms
- ambient output: 14 tokens
- measured throughput: 14.187486 tokens/s
- ambient output non-empty: yes
- non-thinking output cleanliness: passed
- owner-drop/reload wall time: 838 ms
- owner-drop/reload success: yes

## LLM-032 decision

Qwen3-0.6B Q4_K_M is accepted as the second supported Local text model. The canonical real-model run proves exact pinned artifact installation, CPU load/generation, offline generation, non-thinking cleanliness, and unload/reload behavior.

## LLM-123 recommended-model decision

Keep **SmolLM2-360M-Instruct Q4_K_M** as the recommended Local model. The acceptance run is a runtime/usability test, not a semantic-quality benchmark, so it does not claim SmolLM2 has better answer quality. It does show materially lower local-resource cost on the same runner: a smaller artifact, much lower measured RSS delta, faster cold/warm probes and reload, and substantially higher measured token throughput. Both models completed the bounded ambient-style request in about one second.

Qwen3 remains available as the larger supported alternative for users willing to trade additional disk/RAM/CPU cost for a larger model.

## LLM-013 new-profile default decision

New profiles should default to **Local** text generation with SmolLM2 selected. Selection must **not** trigger a download; model installation remains an explicit user action. Settings already exposes the install state and explicit Download & Verify action, and onboarding must explain that the selected Local text model is not automatically downloaded.

Existing installations must not be silently reinterpreted. Persisted profiles created before the text-provider selector continue to migrate explicitly to Google, preserving their prior cloud-text behavior.

This decision changes only the new-profile default. It does not change the V1 architecture boundary: Gemini Live remains the voice-conversation provider, and Google TTS remains cloud-based when selected.
