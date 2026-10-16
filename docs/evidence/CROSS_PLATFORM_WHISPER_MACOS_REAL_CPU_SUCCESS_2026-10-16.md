# macOS Whisper Small genuine CPU-only transcription — two-architecture pass

**Recorded:** 2026-10-16 (UTC)  
**Source SHA:** `999a04e3aefaa9b93f30ad0c53b188009f9f228c`  
**Workflow:** [Whisper macOS Real CPU Acceptance #38886320925](https://github.com/ekkus93/ai-talking-moose/actions/runs/38886320925) — `workflow_dispatch`, **success**  
**Model:** Whisper Small `ggml-small.bin`, 487,601,967 bytes, SHA-256 `1be3a9b2063867b937e64e2ec7483364a79917e157fa98c5d94b5c1fffea987b`  
**Native source:** pinned whisper.cpp `60c0be6ac8fa71b1a2ae2dd938a31a34a508e774`

| Real-model job | Result | Transcript wall | Reported peak RSS | Artifact |
| --- | --- | --- | --- | --- |
| `116924832423` Apple Silicon arm64 | **success** | **2,319 ms** | **865,730,560 bytes** | `12040071161` |
| `116924832215` Intel x86_64 | **success** | **11,502 ms** | **843,063,296 bytes** | `12040957105` |

Both jobs installed the model through the production verified installer, linked and probed the pinned native runtime, transcribed the real pinned `jfk.wav` corpus, asserted meaningful transcript content (`ask`, `country`, `you`), deleted the model, and uploaded JSON evidence rather than weights. Their logs explicitly showed **`whisper_init_with_params_no_state: use gpu = 0`**, **`flash attn = 0`**, and **`whisper_backend_init_gpu: no GPU found`**. The workflow's CPU-backend assertions passed. Real, CPU-only inference is thus proven on both macOS architectures.

Related `pipeline.inference_wall_time_ms` measurements from the job logs: ARM64 **7,715 ms**, Intel **16,536 ms**. These are distinct from the overall `transcribe_wall_ms` field and should not be conflated.

**Explicit remaining limitations:** This workflow installs over HTTPS before transcription; it does **not** enforce OS-level network denial during inference. It exercises the native acceptance CLI, **not** a signed/notarized packaged application or a real microphone. It does not prove the complete model lifecycle under fault injection on Mac, wake-word handoff, frontend settings migration, user TTS playback, or prolonged thermal/latency behavior. WSP-200's other checkboxes and WSP-300/400/600/700 remain subject to separate evidence and must not be marked complete from this run alone.

**Decision boundary:** The approved two-architecture genuine transcription prerequisite is met, allowing development of the settings/provider migration. Do **not** remove Moonshine until the new app routing and relevant replacement tests are qualified. Preserve the independent KittenTTS ONNX Runtime integration.
