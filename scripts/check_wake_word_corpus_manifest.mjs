import { readFileSync } from "node:fs";
import path from "node:path";

const manifestPath = "docs/wake-word-corpus.json";
const manifest = JSON.parse(readFileSync(manifestPath, "utf8"));
const requiredTopLevel = [
  "schema_version",
  "corpus_id",
  "wake_phrase",
  "policy",
  "required_labels",
  "acceptance_criteria",
  "fixtures",
];
const requiredFixtureFields = [
  "id",
  "label",
  "path",
  "speaker_or_source",
  "provenance",
  "license",
  "bytes",
  "sha256",
  "sample_rate_hz",
  "channels",
  "sample_format",
  "expected_detection",
];
const requiredLabels = new Set([
  "positive_wake_phrase",
  "positive_wake_phrase_with_command",
  "negative_ordinary_speech",
  "negative_near_miss",
]);

const fail = (message) => {
  throw new Error(`${manifestPath}: ${message}`);
};

for (const key of requiredTopLevel) {
  if (!Object.hasOwn(manifest, key)) fail(`missing top-level key ${key}`);
}

if (manifest.schema_version !== 1) fail("schema_version must be 1");
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
if (policy.sample_rate_hz !== 16_000) fail("policy.sample_rate_hz must be 16000");
if (policy.channels !== 1) fail("policy.channels must be 1");
if (policy.sample_format !== "pcm_s16le") {
  fail("policy.sample_format must be pcm_s16le");
}
if (policy.fixture_root !== "docs/fixtures/wake-word-v1") {
  fail("policy.fixture_root must be docs/fixtures/wake-word-v1");
}
if (!String(policy.privacy ?? "").includes("Do not commit private room audio")) {
  fail("policy.privacy must explicitly forbid private room audio");
}

const criteria = manifest.acceptance_criteria;
if (!criteria || typeof criteria !== "object" || Array.isArray(criteria)) {
  fail("acceptance_criteria must be an object");
}
if (criteria.criteria_status !== "pending_real_fixture_calibration") {
  fail("criteria_status must remain pending_real_fixture_calibration until real fixtures exist");
}
if (criteria.positive_recall_minimum !== null) {
  fail("positive_recall_minimum must remain null until calibrated fixtures exist");
}
if (criteria.negative_false_accepts_maximum !== null) {
  fail("negative_false_accepts_maximum must remain null until calibrated fixtures exist");
}

if (!Array.isArray(manifest.fixtures)) fail("fixtures must be an array");
const ids = new Set();
const labelsSeen = new Set();
for (const fixture of manifest.fixtures) {
  if (!fixture || typeof fixture !== "object" || Array.isArray(fixture)) {
    fail("each fixture must be an object");
  }
  for (const field of requiredFixtureFields) {
    if (!Object.hasOwn(fixture, field)) fail(`fixture is missing ${field}`);
  }
  if (ids.has(fixture.id)) fail(`duplicate fixture id ${fixture.id}`);
  ids.add(fixture.id);
  if (!requiredLabels.has(fixture.label)) fail(`unknown fixture label ${fixture.label}`);
  labelsSeen.add(fixture.label);
  if (fixture.sample_rate_hz !== policy.sample_rate_hz) {
    fail(`${fixture.id}: sample_rate_hz must match policy`);
  }
  if (fixture.channels !== policy.channels) fail(`${fixture.id}: channels must match policy`);
  if (fixture.sample_format !== policy.sample_format) {
    fail(`${fixture.id}: sample_format must match policy`);
  }
  if (!Number.isInteger(fixture.bytes) || fixture.bytes <= 0) {
    fail(`${fixture.id}: bytes must be a positive integer`);
  }
  if (typeof fixture.sha256 !== "string" || !/^[0-9a-f]{64}$/.test(fixture.sha256)) {
    fail(`${fixture.id}: sha256 must be lowercase 64-character hex`);
  }
  if (fixture.expected_detection !== true && fixture.expected_detection !== false) {
    fail(`${fixture.id}: expected_detection must be boolean`);
  }
  if (!fixture.provenance || typeof fixture.provenance !== "object") {
    fail(`${fixture.id}: provenance must be an object`);
  }
  if (!fixture.license || typeof fixture.license !== "object") {
    fail(`${fixture.id}: license must be an object`);
  }
  if (!fixture.license.spdx || fixture.license.redistributable !== true) {
    fail(`${fixture.id}: license must include redistributable SPDX evidence`);
  }
  const normalized = path.posix.normalize(fixture.path);
  if (normalized !== fixture.path || normalized.startsWith("../") || path.posix.isAbsolute(normalized)) {
    fail(`${fixture.id}: path must be normalized relative POSIX path`);
  }
  if (!normalized.startsWith(`${policy.fixture_root}/`)) {
    fail(`${fixture.id}: path must stay under ${policy.fixture_root}`);
  }
}

if (manifest.fixtures.length > 0) {
  for (const label of requiredLabels) {
    if (!labelsSeen.has(label)) fail(`fixtures are missing required label ${label}`);
  }
}

console.log(
  `Wake Word corpus manifest: schema OK, ${manifest.fixtures.length} fixture(s), criteria=${criteria.criteria_status}.`,
);
