import { readFileSync } from "node:fs";

const read = (path) => readFileSync(path, "utf8");
const fail = (message) => {
  throw new Error(`Wake Word privacy audit failed: ${message}`);
};

const diagnosticsPath = "src-tauri/src/asr/wake_word_diagnostics.rs";
const enginePath = "src-tauri/src/app/wake_word_engine.rs";
const docsPath = "docs/WAKE_WORD_V1_CURRENT_BEHAVIOR.md";
const corpusPath = "docs/wake-word-corpus.json";

const diagnostics = read(diagnosticsPath);
const engine = read(enginePath);
const docs = read(docsPath);
const corpus = JSON.parse(read(corpusPath));

const diagnosticsStruct = diagnostics.match(
  /pub struct WakeWordDiagnostics \{(?<body>[\s\S]*?)\n\}/,
)?.groups?.body;
if (!diagnosticsStruct) {
  fail(`${diagnosticsPath} does not expose WakeWordDiagnostics in the expected form`);
}

const forbiddenDiagnosticsFragments = [
  "Vec<",
  "&[",
  "pcm",
  "transcript",
  "credential",
  "secret",
  "api_key",
  "file_path",
  "absolute_path",
  "audio_content",
];
for (const fragment of forbiddenDiagnosticsFragments) {
  if (diagnosticsStruct.toLowerCase().includes(fragment.toLowerCase())) {
    fail(`WakeWordDiagnostics contains forbidden serialized fragment ${fragment}`);
  }
}

const diagnosticFields = [...diagnosticsStruct.matchAll(/pub\s+([a-zA-Z0-9_]+):/g)].map(
  (match) => match[1],
);
const requiredPrivacySafeFields = [
  "enabled",
  "runtime_phase",
  "model_id",
  "runtime_id",
  "platform",
  "architecture",
  "canonical_sample_rate_hz",
  "canonical_channels",
  "inference_threads",
  "threshold",
  "score",
  "ring_buffer_capacity_samples",
  "ring_buffer_samples",
  "handoff_pre_roll_samples",
  "trigger_count",
  "last_trigger_age_ms",
  "runtime_initialization_ms",
  "talking_suspended",
  "last_error",
];
for (const field of requiredPrivacySafeFields) {
  if (!diagnosticFields.includes(field)) {
    fail(`WakeWordDiagnostics is missing expected privacy-safe field ${field}`);
  }
}

if (!diagnostics.includes("The Wake Word runtime encountered an internal error.")) {
  fail("diagnostics must use the sanitized runtime error string");
}
if (diagnostics.includes("std::path::Path") || diagnostics.includes("PathBuf")) {
  fail("diagnostics module must not serialize filesystem paths");
}

const engineRequirements = [
  "fn sanitize_error_message",
  "<path>",
  "<redacted>",
  "errors_sanitize_paths_and_token_like_secrets",
  "missing required Wake Word native C API library",
];
for (const token of engineRequirements) {
  if (!engine.includes(token)) {
    fail(`${enginePath} is missing privacy sanitizer evidence ${token}`);
  }
}

const docRequirements = [
  "Wake Word diagnostics do not serialize or expose raw PCM.",
  "Diagnostics intentionally do not expose raw PCM, transcripts, credentials, or private audio content.",
  "Do not describe Wake Word V1 as fully user-ready or fully accepted",
];
for (const sentence of docRequirements) {
  if (!docs.includes(sentence)) {
    fail(`${docsPath} is missing required privacy/truthfulness statement: ${sentence}`);
  }
}

if (!String(corpus.policy?.privacy ?? "").includes("Do not commit private room audio")) {
  fail("corpus manifest must forbid private room audio");
}
if (corpus.acceptance_criteria?.criteria_status !== "pending_real_fixture_calibration") {
  fail("corpus criteria must remain pending until real fixtures calibrate acceptance");
}

console.log(
  `Wake Word privacy audit passed: ${diagnosticFields.length} diagnostic field(s), sanitizer evidence, documentation, and corpus privacy policy are OK.`,
);
