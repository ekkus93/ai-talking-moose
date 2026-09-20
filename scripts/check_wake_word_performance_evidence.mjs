import { readFileSync } from "node:fs";

const path = "docs/wake-word-performance-evidence.json";
const report = JSON.parse(readFileSync(path, "utf8"));
const fail = (message) => {
  throw new Error(`Wake Word performance evidence failed: ${message}`);
};

if (report.schema_version !== 1) fail("schema_version must be 1");
if (!["pending_measurement", "accepted"].includes(report.status)) {
  fail("status must be pending_measurement or accepted");
}
if (report.policy?.inference_threads !== 1) fail("one-thread policy must remain explicit");
const platforms = new Set(report.policy?.required_platforms ?? []);
for (const platform of ["linux-x86_64", "macos-arm64"]) {
  if (!platforms.has(platform)) fail(`missing required platform ${platform}`);
}
const requiredMetrics = report.policy?.required_metrics ?? [];
for (const metric of [
  "idle_cpu_percent",
  "runtime_memory_mib",
  "inference_latency_ms",
  "wake_to_command_asr_ms",
  "pre_roll_startup_ms",
  "repeated_cycle_resource_delta",
  "continuous_asr_idle_cpu_percent",
]) {
  if (!requiredMetrics.includes(metric)) fail(`missing required metric ${metric}`);
}

if (report.status === "pending_measurement") {
  if ((report.measurements ?? []).length !== 0) {
    fail("pending report must not contain unqualified measurements");
  }
  console.log("Wake Word performance evidence policy is valid; measurements remain pending.");
  process.exit(0);
}

const measurements = report.measurements ?? [];
for (const platform of platforms) {
  const sample = measurements.find((entry) => entry.platform === platform);
  if (!sample) fail(`accepted report lacks ${platform} measurement`);
  if (!sample.commit_sha || !sample.runner || !sample.measured_at) {
    fail(`${platform} measurement lacks commit/runner/timestamp provenance`);
  }
  for (const metric of requiredMetrics) {
    if (!(metric in sample.metrics)) fail(`${platform} lacks metric ${metric}`);
  }
  if (!(sample.metrics.idle_cpu_percent < sample.metrics.continuous_asr_idle_cpu_percent)) {
    fail(`${platform} does not demonstrate KWS lighter than continuous ASR`);
  }
}
console.log("Wake Word performance evidence is structurally complete and accepted.");
