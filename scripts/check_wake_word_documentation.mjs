import { readFileSync } from "node:fs";

const read = (path) => readFileSync(path, "utf8");
const fail = (message) => {
  throw new Error(`Wake Word documentation audit failed: ${message}`);
};

const behavior = read("docs/WAKE_WORD_V1_CURRENT_BEHAVIOR.md");
const architecture = read("docs/WAKE_WORD_V1_ARCHITECTURE.md");
const gates = read("docs/WAKE_WORD_V1_CI_GATES.md");
const readme = read("README.md");
const panel = read("src/components/Settings/WakeWordSettingsPanel.tsx");
const runtimePreferences = read("src-tauri/src/app/runtime_preferences.rs");
const wakeWordState = read("src-tauri/src/app/wake_word_state.rs");
const artifacts = JSON.parse(read("wake-word-artifacts.json"));
const performance = JSON.parse(read("docs/wake-word-performance-evidence.json"));

const behaviorRequirements = [
  "Hey, Moose",
  "defaults to disabled",
  "local keyword-spotting",
  "microphone",
  "does **not** implement wake-word barge-in",
  "in-memory ring/pre-roll",
  "diagnostics",
  "Live enable/disable changes are applied to the authoritative runtime",
  "one-stream production microphone routing",
  "WWR-630 accepted performance evidence",
  "Wake-triggered command activation is limited to local Moonshine streaming command ASR",
  "unsupported command ASR modes such as Gemini Live audio remain available to ordinary manual interaction",
  "developer-prepared rather than clean-install user-ready",
  "empty app data fails closed until artifacts are prepared and verified",
];
for (const token of behaviorRequirements) {
  if (!behavior.toLowerCase().includes(token.toLowerCase())) {
    fail(`current-behavior documentation is missing ${token}`);
  }
}

const architectureRequirements = [
  "one `WakeWordApplicationRuntime` in `AppState`",
  "single canonical `WakeWordRuntimeManager`",
  "authoritative application microphone owner remains `AppState::audio_capture`",
  "Wake Word composition deliberately does not open a microphone device",
  "16 kHz mono PCM",
  "one inference thread",
  "two seconds of in-memory pre-roll",
  "does not implement barge-in",
  "Component tests are **not** a substitute",
  "Linux x86_64 and macOS arm64 real-KWS acceptance passed",
  "final closeout still depends on measured performance, documentation/source audits, original TODO reconciliation, and exact final qualification",
  "Wake Word V1 supports this handoff only for local Moonshine streaming command ASR",
  "Unsupported modes such as Gemini Live audio are rejected before listener startup or Wake enablement",
  "local Moonshine command-ASR ingress boundary for Wake-triggered handoff audio",
  "selected provisioning model is developer-prepared",
  "clean app-data install with missing model/runtime files must fail closed",
  "app does not silently download Wake artifacts",
];
for (const token of architectureRequirements) {
  if (!architecture.includes(token)) {
    fail(`architecture documentation is missing truthfulness boundary: ${token}`);
  }
}

const runtimeEvidence = [
  "apply_wake_word_setting_change",
  "app.try_state::<AppState>()",
  "state.wake_word_runtime",
  "rollback_wake_word_setting",
  "previous.wake_word_enabled != next.wake_word_enabled",
  "previous.asr_mode != next.asr_mode",
];
for (const token of runtimeEvidence) {
  if (!runtimePreferences.includes(token)) {
    fail(`live-toggle documentation lacks source evidence ${token}`);
  }
}

const wakePolicyEvidence = [
  "wake_word_asr_mode_supported",
  "AsrMode::MoonshineTinyStreaming",
  "AsrMode::MoonshineSmallStreaming",
  "AsrMode::GeminiLiveAudio",
  "Wake Word V1 requires local Moonshine command ASR",
];
for (const token of wakePolicyEvidence) {
  if (!wakeWordState.includes(token)) {
    fail(`local-Moonshine Wake policy lacks source evidence ${token}`);
  }
}

const uiRequirements = [
  "Wake Word V1 uses local/offline keyword spotting.",
  "The microphone remains locally active while listening.",
  "Wake detection is not full-time cloud transcription.",
  "Wake-triggered commands require local Moonshine command ASR.",
  "Wake model/runtime artifacts are developer-prepared; a clean install",
  "fails closed until the pinned artifacts are prepared and verified.",
  "Wake Word V1 has no barge-in support while Moose talks.",
];
for (const sentence of uiRequirements) {
  if (!panel.includes(sentence)) {
    fail(`Settings disclosure drifted: ${sentence}`);
  }
}

const gateRequirements = [
  "Ordinary CI alone is not final Wake Word V1 qualification",
  "A skipped corpus gate is not evidence that real corpus acceptance passed",
  "Wake Word corpus contract",
  "Passing schema/contract gates do not mean the repository contains real audio fixtures",
  "production Wake Word Rust error/log surfaces",
  "does not by itself prove real KWS inference",
  "`accepted` with Linux x86_64 and macOS arm64 platform baselines",
  "A workflow with conclusion `skipped`",
  "Cross-cutting exact-run evidence records wake→command-ASR latency, pre-roll startup timing, and repeated-cycle resource behavior",
];
for (const sentence of gateRequirements) {
  if (!gates.includes(sentence)) {
    fail(`CI gate documentation is missing truthfulness boundary: ${sentence}`);
  }
}

const provisioningPolicy = artifacts.policy ?? {};
if (provisioningPolicy.provisioning_model !== "developer-prepared") {
  fail("artifact manifest must record developer-prepared provisioning model");
}
if (provisioningPolicy.clean_install_behavior !== "fail-closed-until-prepared") {
  fail("artifact manifest must record fail-closed clean-install behavior");
}
if (provisioningPolicy.silent_network_download !== false) {
  fail("artifact manifest must reject silent app-startup artifact downloads");
}

const readmeMentionsWakeWord = /wake[- ]word/i.test(readme);
const readmeUnqualifiedAcceptanceClaims = [
  /wake[- ]word[^\n.]*fully user[- ]ready/i,
  /wake[- ]word[^\n.]*fully accepted/i,
  /wake[- ]word[^\n.]*production[- ]accepted/i,
  /wake[- ]word[^\n.]*real kws acceptance[^\n.]*passed/i,
  /wake[- ]word[^\n.]*linux x86_64[^\n.]*accepted/i,
  /wake[- ]word[^\n.]*macos arm64[^\n.]*accepted/i,
];
for (const pattern of readmeUnqualifiedAcceptanceClaims) {
  if (pattern.test(readme)) {
    fail(`README contains unqualified Wake Word acceptance claim matching ${pattern}`);
  }
}
if (readmeMentionsWakeWord) {
  const readmeTruthfulBoundary =
    /not yet fully accepted/i.test(readme) ||
    /under qualification/i.test(readme) ||
    /real audio acceptance.*pending/i.test(readme) ||
    /wake word.*pending/i.test(readme);
  if (!readmeTruthfulBoundary) {
    fail("README mentions Wake Word without an explicit pending/qualification boundary");
  }
}

if (performance.status !== "accepted") {
  fail("performance report must remain accepted after WWR-630 closeout");
}
if (!Array.isArray(performance.measurements) || performance.measurements.length !== 2) {
  fail("accepted performance report must contain both platform baselines");
}

const forbiddenClaims = [
  /wake word v1 is fully user[- ]ready/i,
  /wake word v1 is fully accepted/i,
  /production acceptance (?:is )?complete/i,
  /wake-triggered commands? support(?:s)? gemini live audio/i,
  /wake word[^\n.]*provider-neutral command-asr/i,
  /wake word[^\n.]*clean[- ]install user[- ]ready/i,
  /wake word[^\n.]*silently downloads/i,
];
for (const pattern of forbiddenClaims) {
  if (pattern.test(behavior) || pattern.test(architecture) || pattern.test(gates) || pattern.test(panel)) {
    fail(`unqualified or unsupported Wake claim matched ${pattern}`);
  }
}

console.log("Wake Word documentation audit: behavior, architecture, source-backed live toggle, local-Moonshine Wake ASR policy, developer-prepared artifact provisioning, UI disclosures, README truthfulness boundary, gate boundaries, and accepted performance status are consistent.");
