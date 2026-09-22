import { readFileSync } from "node:fs";
import path from "node:path";

const manifestPath = "docs/wake-word-corpus.json";
const manifest = JSON.parse(readFileSync(manifestPath, "utf8"));
const artifactManifest = JSON.parse(readFileSync("wake-word-artifacts.json", "utf8"));
const requiredLabels = new Set([
  "positive_wake_phrase",
  "positive_wake_phrase_with_command",
  "negative_ordinary_speech",
  "negative_near_miss",
]);
const fail = (message) => {
  throw new Error(`${manifestPath}: ${message}`);
};
const requireHexSha256 = (value, label) => {
  if (typeof value !== "string" || !/^[0-9a-f]{64}$/.test(value)) {
    fail(`${label} must be lowercase 64-character hex`);
  }
};
const requiredRuntimeSha = (platformKey) => {
  const platform = artifactManifest.runtime.platforms[platformKey];
  const cApi = platform.files.find((file) =>
    file.path.includes("sherpa-onnx-c-api")
  );
  if (!cApi) fail(`artifact manifest missing ${platformKey} C API file`);
  return cApi.sha256;
};

if (manifest.schema_version !== 2) fail("schema_version must be 2");
if (manifest.wake_phrase !== "Hey, Moose") fail("wake_phrase must be Hey, Moose");
if (!Array.isArray(manifest.required_labels)) fail("required_labels must be an array");
for (const label of requiredLabels) {
  if (!manifest.required_labels.includes(label)) {
    fail(`required_labels is missing ${label}`);
  }
}

const policy = manifest.policy;
if (!policy || typeof policy !== "object" || Array.isArray(policy)) {
  fail("policy must be an object");
}
if (policy.sample_rate_hz !== 16000) fail("policy.sample_rate_hz must be 16000");
if (policy.channels !== 1) fail("policy.channels must be 1");
if (policy.sample_format !== "pcm_s16le") fail("policy.sample_format must be pcm_s16le");
if (policy.score !== 1.0) fail("policy.score must be 1.0");
if (policy.threshold !== 0.25) fail("policy.threshold must be 0.25");
if (policy.generator !== "scripts/generate_wake_word_corpus.py") {
  fail("unexpected corpus generator");
}
if (policy.generation_tool !== "espeak-ng") fail("unexpected speech generation tool");
if (policy.generated_audio_is_ephemeral !== true) {
  fail("generated audio must remain ephemeral");
}
if (!String(policy.privacy ?? "").includes("Do not commit private room audio")) {
  fail("policy.privacy must explicitly forbid private room audio");
}

const identity = manifest.model_runtime_identity;
const model = artifactManifest.artifacts.find(
  (artifact) => artifact.kind === "kws-model"
);
if (!model) fail("artifact manifest missing kws-model artifact");
if (identity.source_manifest !== "wake-word-artifacts.json") {
  fail("identity.source_manifest drifted");
}
if (identity.model_id !== model.id) fail("identity.model_id must match artifact manifest");
if (identity.model_archive_sha256 !== model.archive.sha256) {
  fail("identity.model_archive_sha256 must match artifact manifest");
}
if (identity.keyword_sha256 !== model.keyword.sha256) {
  fail("identity.keyword_sha256 must match artifact manifest");
}
if (identity.runtime_id !== artifactManifest.runtime.id) {
  fail("identity.runtime_id must match artifact manifest");
}
if (identity.runtime_version !== artifactManifest.runtime.version) {
  fail("identity.runtime_version must match artifact manifest");
}
if (identity.linux_x86_64_c_api_sha256 !== requiredRuntimeSha("linux-x86_64")) {
  fail("Linux C API identity drifted");
}
if (identity.macos_arm64_c_api_sha256 !== requiredRuntimeSha("macos-arm64")) {
  fail("macOS C API identity drifted");
}
for (const [key, value] of Object.entries(identity)) {
  if (key.endsWith("sha256")) requireHexSha256(value, `identity.${key}`);
}

const criteria = manifest.acceptance_criteria;
if (criteria.criteria_version !== 2) fail("criteria_version must be 2");
if (criteria.criteria_status !== "active_predeclared") {
  fail("criteria must be predeclared before inference");
}
if (
  typeof criteria.positive_recall_minimum !== "number" ||
  criteria.positive_recall_minimum <= 0 ||
  criteria.positive_recall_minimum > 1
) {
  fail("positive_recall_minimum must be within (0,1]");
}
if (
  !Number.isInteger(criteria.negative_false_accepts_maximum) ||
  criteria.negative_false_accepts_maximum < 0
) {
  fail("negative_false_accepts_maximum must be a non-negative integer");
}

if (!Array.isArray(manifest.fixtures) || manifest.fixtures.length < 8) {
  fail("fixtures must contain a meaningful deterministic corpus");
}
const ids = new Set();
const outputs = new Set();
const labelsSeen = new Set();
const speakers = new Set();
let hasGainVariation = false;
let hasDistanceVariation = false;
let hasNoiseVariation = false;
for (const fixture of manifest.fixtures) {
  for (const field of [
    "id",
    "label",
    "output",
    "speaker_or_source",
    "text",
    "voice",
    "speed_wpm",
    "gain_db",
    "distance_scale",
    "noise_amplitude",
    "expected_detection",
    "provenance",
    "license",
  ]) {
    if (!Object.hasOwn(fixture, field)) {
      fail(`fixture is missing ${field}`);
    }
  }
  if (ids.has(fixture.id)) fail(`duplicate fixture id ${fixture.id}`);
  ids.add(fixture.id);
  if (outputs.has(fixture.output)) {
    fail(`duplicate fixture output ${fixture.output}`);
  }
  outputs.add(fixture.output);
  if (!requiredLabels.has(fixture.label)) {
    fail(`unknown fixture label ${fixture.label}`);
  }
  labelsSeen.add(fixture.label);
  speakers.add(fixture.speaker_or_source);
  if (typeof fixture.text !== "string" || fixture.text.trim() === "") {
    fail(`${fixture.id}: text is required`);
  }
  if (typeof fixture.voice !== "string" || fixture.voice.trim() === "") {
    fail(`${fixture.id}: voice is required`);
  }
  if (
    !Number.isInteger(fixture.speed_wpm) ||
    fixture.speed_wpm < 80 ||
    fixture.speed_wpm > 260
  ) {
    fail(`${fixture.id}: speed_wpm out of bounds`);
  }
  if (typeof fixture.gain_db !== "number" || !Number.isFinite(fixture.gain_db)) {
    fail(`${fixture.id}: gain_db invalid`);
  }
  if (
    typeof fixture.distance_scale !== "number" ||
    fixture.distance_scale <= 0 ||
    fixture.distance_scale > 2
  ) {
    fail(`${fixture.id}: distance_scale invalid`);
  }
  if (
    typeof fixture.noise_amplitude !== "number" ||
    fixture.noise_amplitude < 0 ||
    fixture.noise_amplitude > 0.1
  ) {
    fail(`${fixture.id}: noise_amplitude invalid`);
  }
  if (
    fixture.expected_detection !== true &&
    fixture.expected_detection !== false
  ) {
    fail(`${fixture.id}: expected_detection must be boolean`);
  }
  if (
    !fixture.provenance ||
    fixture.provenance.kind !== "deterministic_synthetic_speech"
  ) {
    fail(`${fixture.id}: deterministic provenance required`);
  }
  if (fixture.provenance.audio_distribution !== "ephemeral-only") {
    fail(`${fixture.id}: generated PCM must remain ephemeral`);
  }
  if (
    !fixture.license ||
    fixture.license.redistributable !== true ||
    !fixture.license.spdx
  ) {
    fail(`${fixture.id}: recipe license evidence required`);
  }
  const normalized = path.posix.normalize(fixture.output);
  if (
    normalized !== fixture.output ||
    normalized.startsWith("../") ||
    path.posix.isAbsolute(normalized)
  ) {
    fail(`${fixture.id}: output path must be normalized and relative`);
  }
  hasGainVariation ||= fixture.gain_db !== 0;
  hasDistanceVariation ||= fixture.distance_scale !== 1;
  hasNoiseVariation ||= fixture.noise_amplitude !== 0;
}
for (const label of requiredLabels) {
  if (!labelsSeen.has(label)) fail(`fixtures are missing required label ${label}`);
}
if (speakers.size < 3) {
  fail("corpus must include at least three reproducible speaker/source variants");
}
if (!hasGainVariation) fail("corpus must vary speaking volume/gain");
if (!hasDistanceVariation) fail("corpus must vary simulated distance");
if (!hasNoiseVariation) fail("corpus must include deterministic noise variants");

console.log(
  `Wake Word corpus manifest: schema OK, ${manifest.fixtures.length} deterministic recipes, ${speakers.size} speaker/source variants, criteria=${criteria.positive_recall_minimum}/${criteria.negative_false_accepts_maximum}.`
);
