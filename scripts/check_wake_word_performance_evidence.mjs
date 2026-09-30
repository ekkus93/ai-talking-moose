import { readFileSync } from "node:fs";

const path = process.argv[2] ?? "docs/wake-word-performance-evidence.json";
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

const productionRequiredMetrics = report.policy?.production_listener_required_report_metrics ?? [];
for (const metric of [
  "startup_duration_ms",
  "idle_cpu_percent",
  "idle_observation_ms",
  "peak_resident_memory_before_bytes",
  "peak_resident_memory_after_start_bytes",
  "startup_peak_memory_delta_bytes",
  "mean_inference_latency_ms",
  "p95_inference_latency_ms",
  "inference_frames",
  "wake_to_command_asr_ms",
  "pre_roll_startup_ms",
  "command_start_ms",
  "total_activation_ms",
  "handoff_samples",
  "repeated_wake_command_resume_cycles",
  "repeated_enable_disable_cycles",
  "peak_active_native_sessions",
  "final_active_native_sessions",
  "final_capture_active",
  "final_runtime_phase",
  "ring_buffer_delta_samples",
  "handoff_pre_roll_delta_samples",
  "inference_threads",
  "passed",
]) {
  if (!productionRequiredMetrics.includes(metric)) {
    fail(`missing production-listener required report metric ${metric}`);
  }
}

const scope = report.wpcr500_scope ?? {};
if (scope.production_listener_measurement_required !== true) {
  fail("WPCR-500 must explicitly require production-listener measurement");
}
if (scope.accepted_wwr630_is_full_wpcr500_closeout !== false) {
  fail("accepted WWR-630 evidence must remain distinct from full WPCR-500 closeout");
}
if (typeof scope.accepted_wwr630_scope !== "string" || scope.accepted_wwr630_scope.length === 0) {
  fail("WPCR-500 scope must describe the accepted WWR-630 evidence boundary");
}

const assertMeasurementPath = (entry, label) => {
  if (typeof entry.measurement_path !== "string" || entry.measurement_path.length === 0) {
    fail(`${label} lacks measurement_path scope`);
  }
};

const assertProvenance = (entry, label) => {
  if (!entry.commit_sha || !entry.runner || !entry.measured_at) {
    fail(`${label} lacks commit/runner/timestamp provenance`);
  }
  assertMeasurementPath(entry, label);
};

const assertExactSha = (sha, label) => {
  if (typeof sha !== "string" || !/^[0-9a-f]{40}$/u.test(sha)) {
    fail(`${label} must contain an exact 40-character lowercase commit SHA`);
  }
};

const assertPositiveInteger = (value, label) => {
  if (!Number.isInteger(value) || value <= 0) fail(`${label} must be a positive integer`);
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
    if (entry.measurement_path === "production_wake_listener_thread") {
      fail("partial measurements must not masquerade as production listener measurements");
    }
    const metrics = entry.metrics ?? {};
    for (const metric of ["idle_cpu_percent", "runtime_memory_mib", "inference_latency_ms"]) {
      assertNonNegativeMetric(metrics, metric, `${entry.platform} partial measurement`);
    }
    if (metrics.inference_threads !== 1) {
      fail(`${entry.platform} partial measurement must preserve one-thread policy`);
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
};

const validateProductionListenerEvidence = () => {
  const entries = report.production_listener_evidence ?? [];
  if (!Array.isArray(entries)) fail("production_listener_evidence must be an array");
  for (const platform of platforms) {
    const matching = entries.filter((entry) => entry.platform === platform);
    if (matching.length !== 1) {
      fail(`accepted report must contain exactly one production listener evidence entry for ${platform}`);
    }
    const entry = matching[0];
    if (entry.measurement_path !== "production_wake_listener_thread") {
      fail(`${platform} production listener evidence has wrong measurement_path`);
    }
    assertExactSha(entry.commit_sha, `${platform} production listener evidence`);
    if (entry.workflow_name !== "Wake Word real KWS acceptance") {
      fail(`${platform} production listener evidence must identify the authoritative workflow`);
    }
    for (const [value, label] of [
      [entry.workflow_run_id, "workflow_run_id"],
      [entry.job_id, "job_id"],
      [entry.artifact_id, "artifact_id"],
    ]) {
      assertPositiveInteger(value, `${platform} production listener ${label}`);
    }
    if (entry.report_name !== `${platform}-production-listener.json`) {
      fail(`${platform} production listener report_name is not canonical`);
    }
    if (typeof entry.artifact_name !== "string" || !entry.artifact_name.includes(platform)) {
      fail(`${platform} production listener artifact_name must identify the platform`);
    }
    if (typeof entry.runner !== "string" || entry.runner.length === 0 || !entry.measured_at) {
      fail(`${platform} production listener evidence lacks runner/timestamp`);
    }
    const metricSet = new Set(entry.required_report_metrics ?? []);
    for (const metric of productionRequiredMetrics) {
      if (!metricSet.has(metric)) {
        fail(`${platform} production listener evidence does not require report metric ${metric}`);
      }
    }
  }
};

validatePartialMeasurements();
validateCrossCuttingMeasurements();

if (report.status === "pending_measurement") {
  if (scope.production_listener_status !== "pending_measurement") {
    fail("pending report must keep production-listener status pending");
  }
  if ((report.production_listener_evidence ?? []).length !== 0) {
    fail("pending report must not contain accepted production-listener evidence");
  }
  console.log("Wake Word performance evidence policy is valid; production-listener evidence remains pending.");
  process.exit(0);
}

if (scope.production_listener_status !== "accepted") {
  fail("accepted report must mark WPCR-500 production-listener status accepted");
}
if (typeof scope.production_listener_evidence_source !== "string" || scope.production_listener_evidence_source.length < 20) {
  fail("accepted report must identify the production-listener evidence source");
}
validateProductionListenerEvidence();

const platformRequiredMetrics = report.policy?.platform_required_metrics ?? [];
for (const metric of ["idle_cpu_percent", "runtime_memory_mib", "inference_latency_ms", "continuous_asr_idle_cpu_percent"]) {
  if (!platformRequiredMetrics.includes(metric)) fail(`missing platform-required metric ${metric}`);
}
const crossCuttingRequiredMetrics = report.policy?.cross_cutting_required_metrics ?? [];
for (const metric of ["wake_to_command_asr_ms", "pre_roll_startup_ms", "repeated_cycle_resource_delta"]) {
  if (!crossCuttingRequiredMetrics.includes(metric)) fail(`missing cross-cutting-required metric ${metric}`);
}

const measurements = report.measurements ?? [];
for (const platform of platforms) {
  const sample = measurements.find((entry) => entry.platform === platform);
  if (!sample) fail(`accepted report lacks ${platform} baseline measurement`);
  assertProvenance(sample, `${platform} measurement`);
  if (sample.measurement_path === "production_wake_listener_thread") {
    fail("WWR-630 platform baselines must remain distinct from production-listener evidence");
  }
  for (const metric of platformRequiredMetrics) {
    assertNumericMetric(sample.metrics ?? {}, metric, platform);
  }
  if (sample.metrics.inference_threads !== 1) fail(`${platform} must preserve one-thread policy`);
  if (!(sample.metrics.idle_cpu_percent < sample.metrics.continuous_asr_idle_cpu_percent)) {
    fail(`${platform} does not demonstrate KWS lighter than continuous ASR`);
  }
}

console.log("Wake Word performance evidence is accepted: WWR-630 baselines remain distinct and WPCR-500 production-listener evidence is bound to exact native artifacts.");
