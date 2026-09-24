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

const assertProvenance = (entry, label) => {
  if (!entry.commit_sha || !entry.runner || !entry.measured_at) {
    fail(`${label} lacks commit/runner/timestamp provenance`);
  }
};

const assertNumericMetric = (metrics, metric, label) => {
  if (!(metric in metrics)) fail(`${label} lacks metric ${metric}`);
  if (typeof metrics[metric] !== "number" || !Number.isFinite(metrics[metric])) {
    fail(`${label} metric ${metric} must be finite number`);
  }
};

const validatePartialMeasurements = () => {
  const partialMeasurements = report.partial_measurements ?? [];
  if (!Array.isArray(partialMeasurements)) fail("partial_measurements must be an array");
  for (const entry of partialMeasurements) {
    if (!platforms.has(entry.platform)) fail(`partial measurement uses unknown platform ${entry.platform}`);
    assertProvenance(entry, `${entry.platform} partial measurement`);
    const metrics = entry.metrics ?? {};
    for (const metric of ["idle_cpu_percent", "runtime_memory_mib", "inference_latency_ms"]) {
      assertNumericMetric(metrics, metric, `${entry.platform} partial measurement`);
    }
    if (metrics.inference_threads !== 1) {
      fail(`${entry.platform} partial measurement must preserve one-thread policy`);
    }
    const pending = new Set(entry.pending_metrics ?? []);
    for (const metric of pending) {
      if (!requiredMetrics.includes(metric)) {
        fail(`${entry.platform} partial measurement has unknown pending metric ${metric}`);
      }
    }
  }
};

if (report.status === "pending_measurement") {
  if ((report.measurements ?? []).length !== 0) {
    fail("pending report must not contain accepted measurements");
  }
  validatePartialMeasurements();
  console.log("Wake Word performance evidence policy is valid; accepted measurements remain pending.");
  process.exit(0);
}

const measurements = report.measurements ?? [];
for (const platform of platforms) {
  const sample = measurements.find((entry) => entry.platform === platform);
  if (!sample) fail(`accepted report lacks ${platform} measurement`);
  assertProvenance(sample, `${platform} measurement`);
  for (const metric of requiredMetrics) {
    assertNumericMetric(sample.metrics ?? {}, metric, platform);
  }
  if (!(sample.metrics.idle_cpu_percent < sample.metrics.continuous_asr_idle_cpu_percent)) {
    fail(`${platform} does not demonstrate KWS lighter than continuous ASR`);
  }
}
console.log("Wake Word performance evidence is structurally complete and accepted.");
