# ASR-015 supported-Mac native acceptance report

Status: **PASS**

## Reference hardware

- Hardware model: `VirtualMac2,1`
- CPU/chip: `Apple M1 (Virtual)`
- Physical/logical CPU count: 3 / 3
- RAM: 7168.0 MiB
- macOS: 15.7.7 (24G720)
- Architecture: `arm64`
- Low-power mode: `unknown`
- Talking Moose commit: `f181a1d18ab65eecfddf875f6706d9bce5f136fb`
- GitHub Actions run: `https://github.com/ekkus93/ai-talking-moose/actions/runs/32665369708`
- Run attempt: `1`

This is the minimum **measured** CPU reference established by this acceptance run. No slower Mac CPU is claimed supported by this evidence.

## Corpus

- Source: `ggml-org/whisper.cpp/samples/jfk.wav` at `45f1593fd326b3435c04392e3151dff65967e523`
- Source Git blob: `3184d372cd2f8b804d3a540c70ec50d927b335d2`
- Source SHA-256: `59dfb9a4acb36fe2a2affc14bacbee2920ff435cb13cc314a08c13f66ba7860e`
- Derived PCM SHA-256: `5d5024881abcb527a43c9b643abed1545627960ac894584167ea510c8a442061`
- Format: 16000 Hz mono signed 16-bit little-endian PCM
- Duration: 13.0 s (11.0 s speech + 2.0 s trailing silence)

## Tiny Streaming

| Run | RTF | First partial ms | First final ms | Native latency ms | CPU % | Peak RSS MiB | Peak Δ MiB | Drops | Final transcript |
| --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | --- |
| warm-up | 0.091 | 1864 | 5262 | 53 | 5.3 | 307.6 | 287.9 | 0 | And so my fellow America, Ask not. What your country can do for you? Ask what you can do for your country. |
| measured 1 | 0.099 | 1522 | 4784 | 76 | 5.7 | 295.0 | 278.7 | 0 | And so my fellow America, Ask not. What your country can do for you? Ask what you can do for your country. |
| measured 2 | 0.106 | 1339 | 4747 | 58 | 6.0 | 292.7 | 276.3 | 0 | And so my fellow America, Ask not. What your country can do for you? Ask what you can do for your country. |
| measured 3 | 0.101 | 1726 | 5356 | 54 | 5.4 | 302.5 | 286.2 | 0 | And so my fellow America, Ask not. What your country can do for you? Ask what you can do for your country. |
| measured 4 | 0.094 | 1793 | 4714 | 53 | 5.4 | 298.6 | 282.2 | 0 | And so my fellow America, Ask not. What your country can do for you? Ask what you can do for your country. |
| measured 5 | 0.091 | 1518 | 4744 | 56 | 4.9 | 306.5 | 290.1 | 0 | And so my fellow America, Ask not. What your country can do for you? Ask what you can do for your country. |

Measured median RTF: **0.099**; worst RTF: **0.106**.
Measured median first-final latency: **4747 ms**; worst: **5356 ms**.
Highest sampled RSS across measured runs: **306.5 MiB**.

## Small Streaming

| Run | RTF | First partial ms | First final ms | Native latency ms | CPU % | Peak RSS MiB | Peak Δ MiB | Drops | Final transcript |
| --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | --- |
| warm-up | 0.194 | 1658 | 4776 | 145 | 11.2 | 766.7 | 745.9 | 0 | And so my fellow Americans, Ask not! What your country can do for you. Ask what you can do for your country. |
| measured 1 | 0.196 | 1620 | 4856 | 151 | 11.7 | 751.7 | 735.3 | 0 | And so my fellow Americans, Ask not! What your country can do for you. Ask what you can do for your country. |
| measured 2 | 0.195 | 1618 | 4522 | 144 | 10.7 | 736.0 | 718.5 | 0 | And so my fellow Americans, Ask not! What your country can do for you. Ask what you can do for your country. |
| measured 3 | 0.187 | 1602 | 5434 | 145 | 10.4 | 733.5 | 716.1 | 0 | And so my fellow Americans, Ask not! What your country can do for you. Ask what you can do for your country. |
| measured 4 | 0.200 | 1923 | 5081 | 174 | 10.9 | 758.6 | 741.2 | 0 | And so my fellow Americans, Ask not! What your country can do for you. Ask what you can do for your country. |
| measured 5 | 0.183 | 1630 | 5353 | 133 | 10.2 | 750.8 | 733.4 | 0 | And so my fellow Americans, Ask not! What your country can do for you. Ask what you can do for your country. |

Measured median RTF: **0.195**; worst RTF: **0.200**.
Measured median first-final latency: **5081 ms**; worst: **5434 ms**.
Highest sampled RSS across measured runs: **758.6 MiB**.

## Acceptance conclusion

Tiny and Small both completed one warm-up and five measured real native streaming runs with:

- a useful partial and final transcript on every run;
- zero bounded-ingress drops;
- no typed ASR error;
- measured RTF strictly below `1.0` on every run; and
- CPU and process-RSS metrics recorded from the production local-ASR pipeline.

The measured reference CPU is **Apple M1 (Virtual) (VirtualMac2,1)**. This report does not claim support for slower CPU models without the same measured gate.
