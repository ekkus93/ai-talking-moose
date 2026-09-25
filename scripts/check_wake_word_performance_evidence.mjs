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

const assertNonNegativeMetric = (metrics, metric, label) => {
  assertNumericMetric(metrics, metric, label);
  if (metrics[metric] < 0) fail(`${label} metric ${metric} must be non-negative`);
};

const validatePartialMeasurements = () => {
  const partialMeasurements = report.partial_measurements ?? [];
  if (!Array.isArray(partialMeasurements)) fail("partial_measurements must be an array");
  for (const entry of partialMeasurements) {
    if (!platforms.has(entry.platform)) fail(`partial measurement uses unknown platform ${entry.platform}`);
    assertProvenance(entry, `${entry.platform} partial measurement`);
    const metrics = entry.metrics ?? {};
    for (const metric of ["idle_cpu_percent", "runtime_memory_mib", "inference_latency_ms"]) {
      assertNonNegativeMetric(metrics, metric, `${entry.platform} partial measurement`);
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

const requireCrossCutting = (metric) => {
  const entry = (report.cross_cutting_measurements ?? []).find((item) => item.metric === metric);
  if (!entry) fail(`missing cross-cutting measurement ${metric}`);
  assertProvenance(entry, `${metric} cross-cutting measurement`);
  if (!platforms.has(entry.platform)) {
    fail(`${metric} cross-cutting measurement uses unknown platform ${entry.platform}`);
  }
  return entry;
};

const validateCrossCuttingMeasurements = () => {
  const crossCutting = report.cross_cutting_measurements ?? [];
  if (!Array.isArray(crossCutting)) fail("cross_cutting_measurements must be an array");
  for (const entry of crossCutting) {
    if (!entry.metric) fail("cross-cutting measurement lacks metric name");
    assertProvenance(entry, `${entry.metric} cross-cutting measurement`);
    if (!platforms.has(entry.platform)) {
      fail(`${entry.metric} cross-cutting measurement uses unknown platform ${entry.platform}`);
    }
  }

  const repeated = requireCrossCutting("repeated_cycle_resource_delta");
  for (const metric of [
    "cycles",
    "ring_buffer_delta_samples",
    "handoff_pre_roll_delta_samples",
    "trigger_count_delta",
    "capacity_samples",
  ]) {
    assertNumericMetric(repeated.metrics ?? {}, metric, "repeated_cycle_resource_delta");
  }
  if ((repeated.metrics ?? {}).cycles < 1) fail("repeated-cycle evidence must record at least one cycle");
  if ((repeated.metrics ?? {}).ring_buffer_delta_samples !== 0) {
    fail("repeated-cycle evidence must keep ring buffer delta at zero");
  }
  if ((repeated.metrics ?? {}).handoff_pre_roll_delta_samples !== 0) {
    fail("repeated-cycle evidence must keep handoff pre-roll delta at zero");
  }
  if ((repeated.metrics ?? {}).final_phase !== "Listening") {
    fail("repeated-cycle evidence must end in Listening");
  }

  const activation = requireCrossCutting("wake_command_activation_timing");
  for (const metric of [
    "wake_to_command_asr_ms",
    "pre_roll_startup_ms",
    "command_start_ms",
    "total_activation_ms",
    "handoff_samples",
    "ingress_samples",
  ]) {
    assertNonNegativeMetric(activation.metrics ?? {}, metric, "wake_command_activation_timing");
  }
  if ((activation.metrics ?? {}).handoff_samples !== (activation.metrics ?? {}).ingress_samples) {
    fail("wake command activation timing must deliver all handoff samples to ASR ingress");
  }
  if ((activation.metrics ?? {}).wake_to_command_asr_ms < (activation.metrics ?? {}).pre_roll_startup_ms) {
    fail("wake_to_command_asr_ms must include pre_roll_startup_ms");
  }

  const continuous = requireCrossCutting("continuous_asr_idle_cpu_percent");
  assertNonNegativeMetric(
    continuous.metrics ?? {},
    "continuous_asr_idle_cpu_percent",
    "continuous_asr_idle_cpu_percent",
  );
  assertNonNegativeMetric(continuous.metrics ?? {}, "observation_ms", "continuous_asr_idle_cpu_percent");
  if ((continuous.metrics ?? {}).observation_ms < 1000) {
    fail("continuous-ASR idle CPU observation window is too short");
  }
  if ((continuous.metrics ?? {}).queue_depth !== 0) {
    fail("continuous-ASR idle measurement must be idle with empty queue");
  }

  const samePlatformKws = (report.partial_measurements ?? []).find(
    (entry) => entry.platform === continuous.platform,
  );
  if (samePlatformKws?.metrics?.idle_cpu_percent !== undefined) {
    if (!(samePlatformKws.metrics.idle_cpu_percent < continuous.metrics.continuous_asr_idle_cpu_percent)) {
      fail("same-platform partial KWS idle CPU must be lower than continuous-ASR idle CPU diagnostic");
    }
  }
};

if (report.status === "pending_measurement") {
  if ((report.measurements ?? []).length !== 0) {
    fail("pending report must not contain accepted measurements");
  }
  validatePartialMeasurements();
  validateCrossCuttingMeasurements();
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
