import { readFileSync } from "node:fs";

const read = (path) => readFileSync(path, "utf8");
const fail = (message) => {
  throw new Error(`Wake Word documentation audit failed: ${message}`);
};

const behavior = read("docs/WAKE_WORD_V1_CURRENT_BEHAVIOR.md");
const gates = read("docs/WAKE_WORD_V1_CI_GATES.md");
const panel = read("src/components/Settings/WakeWordSettingsPanel.tsx");
const performance = JSON.parse(read("docs/wake-word-performance-evidence.json"));

const behaviorRequirements = [
  "Hey, Moose",
  "defaults to disabled",
  "local keyword-spotting",
  "microphone",
  "does **not** implement wake-word barge-in",
  "in-memory ring/pre-roll",
  "diagnostics",
  "Live settings-to-runtime application is still a remediation item",
  "one-stream production microphone routing",
];
for (const token of behaviorRequirements) {
  if (!behavior.toLowerCase().includes(token.toLowerCase())) {
    fail(`current-behavior documentation is missing ${token}`);
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
  "does not by itself prove real KWS inference",
  "pending_measurement",
  "A workflow with conclusion `skipped`",
];
for (const sentence of gateRequirements) {
  if (!gates.includes(sentence)) {
    fail(`CI gate documentation is missing truthfulness boundary: ${sentence}`);
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
  if (pattern.test(behavior) || pattern.test(gates)) {
    fail(`unqualified acceptance claim matched ${pattern}`);
  }
}

console.log("Wake Word documentation audit: behavior, UI disclosures, gate boundaries, and pending performance status are consistent.");
