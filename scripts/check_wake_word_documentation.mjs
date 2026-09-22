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
  "component tests are **not** a substitute",
  "Linux x86_64 and macOS arm64 remain subject to their dedicated real-KWS acceptance tasks",
  "implementation under qualification rather than as fully accepted cross-platform production functionality",
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
];
for (const token of runtimeEvidence) {
  if (!runtimePreferences.includes(token)) {
    fail(`live-toggle documentation lacks source evidence ${token}`);
  }
}

const uiRequirements = [
  "Wake Word V1 uses local/offline keyword spotting.",
  "The microphone remains locally active while listening.",
  "Wake detection is not full-time cloud transcription.",
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
  "pending_measurement",
  "A workflow with conclusion `skipped`",
];
for (const sentence of gateRequirements) {
  if (!gates.includes(sentence)) {
    fail(`CI gate documentation is missing truthfulness boundary: ${sentence}`);
  }
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

if (performance.status !== "pending_measurement") {
  fail("performance status changed; update documentation audit with accepted measured evidence");
}
if (!Array.isArray(performance.measurements) || performance.measurements.length !== 0) {
  fail("pending performance report unexpectedly contains measurements");
}

const forbiddenClaims = [
  /wake word v1 is fully user[- ]ready/i,
  /wake word v1 is fully accepted/i,
  /production acceptance (?:is )?complete/i,
  /real kws acceptance (?:has )?passed/i,
];
for (const pattern of forbiddenClaims) {
  if (pattern.test(behavior) || pattern.test(architecture) || pattern.test(gates)) {
    fail(`unqualified acceptance claim matched ${pattern}`);
  }
}

console.log("Wake Word documentation audit: behavior, architecture, source-backed live toggle, UI disclosures, README truthfulness boundary, gate boundaries, and pending performance status are consistent.");
