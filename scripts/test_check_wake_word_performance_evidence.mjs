import { mkdtempSync, readFileSync, rmSync, writeFileSync } from "node:fs";
import { tmpdir } from "node:os";
import { join } from "node:path";
import { spawnSync } from "node:child_process";

const canonicalPath = "docs/wake-word-performance-evidence.json";
const checkerPath = "scripts/check_wake_word_performance_evidence.mjs";
const canonical = JSON.parse(readFileSync(canonicalPath, "utf8"));
const tempRoot = mkdtempSync(join(tmpdir(), "wake-performance-evidence-"));

const run = (value, label) => {
  const path = join(tempRoot, `${label}.json`);
  writeFileSync(path, JSON.stringify(value, null, 2));
  return spawnSync(process.execPath, [checkerPath, path], { encoding: "utf8" });
};
const clone = () => structuredClone(canonical);
const expectPass = (value, label) => {
  const result = run(value, label);
  if (result.status !== 0) {
    throw new Error(`${label} unexpectedly failed:\n${result.stderr}\n${result.stdout}`);
  }
};
const expectFail = (value, label) => {
  const result = run(value, label);
  if (result.status === 0) {
    throw new Error(`${label} unexpectedly passed`);
  }
};

try {
  expectPass(clone(), "canonical");

  const stalePending = clone();
  stalePending.wpcr500_scope.production_listener_status = "pending_measurement";
  expectFail(stalePending, "stale-pending-production-listener");

  const missingMetric = clone();
  missingMetric.production_listener_evidence[0].required_report_metrics =
    missingMetric.production_listener_evidence[0].required_report_metrics.filter(
      (metric) => metric !== "startup_duration_ms",
    );
  expectFail(missingMetric, "missing-production-report-metric");

  const badSha = clone();
  badSha.production_listener_evidence[0].commit_sha = "not-an-exact-sha";
  expectFail(badSha, "bad-production-evidence-sha");

  const wrongPath = clone();
  wrongPath.production_listener_evidence[0].measurement_path = "standalone_real_kws_session";
  expectFail(wrongPath, "standalone-masquerades-as-production");

  const missingPlatform = clone();
  missingPlatform.production_listener_evidence =
    missingPlatform.production_listener_evidence.filter((entry) => entry.platform !== "macos-arm64");
  expectFail(missingPlatform, "missing-production-platform");

  console.log("Wake Word performance evidence negative-policy tests passed.");
} finally {
  rmSync(tempRoot, { recursive: true, force: true });
}
