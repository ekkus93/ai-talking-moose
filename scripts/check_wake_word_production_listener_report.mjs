import { readFileSync } from "node:fs";

const args = process.argv.slice(2);
const valueFor = (flag) => {
  const index = args.indexOf(flag);
  if (index < 0 || index + 1 >= args.length) {
    throw new Error(`missing required argument ${flag}`);
  }
  return args[index + 1];
};

const path = valueFor("--report");
const expectedPlatform = valueFor("--platform");
const expected = {
  "linux-x86_64": { platform: "linux", architecture: "x86_64" },
  "macos-arm64": { platform: "macos", architecture: "aarch64" },
}[expectedPlatform];
if (!expected) throw new Error(`unsupported expected platform ${expectedPlatform}`);

const report = JSON.parse(readFileSync(path, "utf8"));
const fail = (message) => {
  throw new Error(`Wake Word production-listener report failed: ${message}`);
};
const finite = (field, { min = 0, positive = false } = {}) => {
  const value = report[field];
  if (typeof value !== "number" || !Number.isFinite(value)) fail(`${field} must be finite number`);
  if (positive ? value <= min : value < min) fail(`${field} is below required bound`);
};

if (report.schema_version !== 1) fail("schema_version must be 1");
if (report.measurement_path !== "production_wake_listener_thread") {
  fail("measurement_path must be production_wake_listener_thread");
}
if (report.capture_transport !== "explicit_mock_pcm_injection") {
  fail("capture_transport must identify explicit mock PCM injection");
}
if (report.platform !== expected.platform) fail(`platform must be ${expected.platform}`);
if (report.architecture !== expected.architecture) fail(`architecture must be ${expected.architecture}`);

finite("startup_duration_ms", { positive: true });
finite("idle_cpu_percent");
finite("idle_observation_ms");
if (report.idle_observation_ms < 1000) fail("idle_observation_ms must be at least 1000");
for (const field of [
  "peak_resident_memory_before_bytes",
  "peak_resident_memory_after_start_bytes",
  "startup_peak_memory_delta_bytes",
  "mean_inference_latency_ms",
  "p95_inference_latency_ms",
  "wake_to_command_asr_ms",
  "pre_roll_startup_ms",
  "command_start_ms",
  "total_activation_ms",
]) {
  finite(field);
}
finite("inference_frames", { positive: true });
finite("handoff_samples", { positive: true });
finite("repeated_wake_command_resume_cycles", { positive: true });
finite("repeated_enable_disable_cycles", { positive: true });
if (report.peak_active_native_sessions !== 1) fail("peak_active_native_sessions must equal one");
if (report.final_active_native_sessions !== 0) fail("final_active_native_sessions must equal zero");
if (report.final_capture_active !== false) fail("final_capture_active must be false");
if (report.final_runtime_phase !== "Disabled") fail("final_runtime_phase must be Disabled");
if (report.ring_buffer_delta_samples !== 0) fail("ring_buffer_delta_samples must be zero");
if (report.handoff_pre_roll_delta_samples !== 0) fail("handoff_pre_roll_delta_samples must be zero");
if (report.inference_threads !== 1) fail("inference_threads must remain one");
if (report.passed !== true) fail("report passed must be true");

console.log(`Wake Word production-listener report passed for ${expectedPlatform}: ${path}`);
