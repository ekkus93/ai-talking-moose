import { readdirSync, readFileSync, statSync } from "node:fs";

const read = (path) => readFileSync(path, "utf8");

const listRustFiles = (dir) => {
  const entries = readdirSync(dir).sort();
  const files = [];
  for (const entry of entries) {
    const path = `${dir}/${entry}`;
    if (statSync(path).isDirectory()) {
      files.push(...listRustFiles(path));
    } else if (path.endsWith(".rs")) {
      files.push(path);
    }
  }
  return files;
};

const productionRust = (source) =>
  source.replace(/\n#\[cfg\(test\)\]\s*\nmod tests \{[\s\S]*$/u, "");

const rustStringLiterals = (source) =>
  [...source.matchAll(/"(?:\\.|[^"\\])*"/gu)].map((match) => match[0]);

const wakeProductionFiles = [
  ...listRustFiles("src-tauri/src/app").filter((path) => path.includes("/wake_word")),
  ...listRustFiles("src-tauri/src/asr").filter((path) => path.includes("/wake_word")),
  ...listRustFiles("src-tauri/src/commands").filter((path) => path.includes("/wake_word")),
];

const fail = (message) => {
  throw new Error(`Wake Word privacy audit failed: ${message}`);
};

const requireText = (source, needle, label) => {
  if (!source.includes(needle)) fail(`${label} is missing ${needle}`);
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
  "handoff_pre_roll_duration_ms",
  "trigger_count",
  "last_trigger_age_ms",
  "runtime_initialization_ms",
  "measured_idle_cpu_percent",
  "measured_memory_rss_bytes",
  "last_inference_duration_ms",
  "last_handoff_duration_ms",
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
  "native_session_rejects_missing_verified_artifacts_before_creation",
  "missing_native_c_api_library_is_sanitized_before_inference",
  "missing required Wake Word native C API library",
];
for (const token of engineRequirements) {
  if (!engine.includes(token)) {
    fail(`${enginePath} is missing privacy sanitizer evidence ${token}`);
  }
}

const sanitizerTestRequirements = [
  "assert_eq!(error.message, \"failed <path> token <redacted>\")",
  "contains(temp.path().to_string_lossy().as_ref())",
  "assert_eq!(error.message, \"native runtime architecture mismatch\")",
];
for (const token of sanitizerTestRequirements) {
  requireText(engine, token, "Wake Word sanitized error regression coverage");
}

const docRequirements = [
  "Wake Word diagnostics do not serialize or expose raw PCM.",
  "Diagnostics intentionally do not expose raw PCM, transcripts, credentials, or private audio content.",
  "Optional measured CPU, memory, inference, and handoff timing fields remain empty until accepted measurements exist.",
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

const forbiddenProductionLogFragments = [
  "tracing::",
  "trace!",
  "debug!",
  "info!",
  "warn!",
  "error!",
  "println!",
  "eprintln!",
];
const forbiddenErrorLiteralFragments = [
  "credential",
  "secret",
  "api_key",
  "transcript",
  "raw_audio",
  "audio_content",
  "absolute_path",
  "file_path",
];
const forbiddenErrorFormattingFragments = [
  "{path}",
  "{file}",
  "{model_dir}",
  "{runtime_dir}",
  "{:?}",
];

for (const path of wakeProductionFiles) {
  const source = productionRust(read(path));
  for (const fragment of forbiddenProductionLogFragments) {
    if (source.includes(fragment)) {
      fail(`${path} contains production Wake Word logging macro/reference ${fragment}`);
    }
  }
  for (const literal of rustStringLiterals(source)) {
    for (const fragment of forbiddenErrorLiteralFragments) {
      if (literal.toLowerCase().includes(fragment)) {
        fail(`${path} contains sensitive production error/log string literal fragment ${fragment}`);
      }
    }
  }
  for (const fragment of forbiddenErrorFormattingFragments) {
    if (source.includes(fragment)) {
      fail(`${path} contains potentially path-leaking production formatting fragment ${fragment}`);
    }
  }
}

console.log(
  `Wake Word privacy audit passed: ${diagnosticFields.length} diagnostic field(s), ${wakeProductionFiles.length} production Wake Word Rust file(s), sanitizer regression evidence, documentation, and corpus privacy policy are OK.`,
);
