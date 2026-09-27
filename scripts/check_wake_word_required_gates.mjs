import { existsSync, readFileSync } from "node:fs";

const read = (path) => readFileSync(path, "utf8");
const fail = (message) => {
  throw new Error(`Wake Word required-gates audit failed: ${message}`);
};
const requireText = (source, needle, label) => {
  if (!source.includes(needle)) fail(`${label} is missing ${needle}`);
};

const manifestPath = "docs/wake-word-required-gates.json";
const manifest = JSON.parse(read(manifestPath));
const gatesDoc = read("docs/WAKE_WORD_V1_CI_GATES.md");

if (manifest.schema_version !== 1) fail("schema_version must be 1");
if (manifest.policy_id !== "wake-word-v1-required-gates") fail("unexpected policy_id");
if (manifest.ordinary_ci_alone_is_final_qualification !== false) {
  fail("ordinary CI must not be final Wake Word qualification");
}
if (manifest.skipped_conclusion_counts_as_pass !== false) {
  fail("skipped workflow conclusions must not count as acceptance evidence");
}
if (!Array.isArray(manifest.gates) || manifest.gates.length < 20) {
  fail("manifest must contain the full implemented and post-closeout Wake gate inventory");
}

const ids = new Set();
const requiredIds = new Set([
  "ordinary_ci",
  "required_gates_manifest",
  "deterministic_corpus_manifest",
  "deterministic_corpus_contract",
  "native_packaging_architecture_policy",
  "lifecycle_stability_policy",
  "settings_listener_lifecycle_acceptance",
  "manual_shared_capture_transfer_acceptance",
  "selected_asr_policy_acceptance",
  "downstream_first_command_word_acceptance",
  "clean_install_artifact_provisioning_acceptance",
  "performance_evidence_policy",
  "production_listener_performance_evidence",
  "privacy_audit",
  "documentation_audit",
  "source_security_audit",
  "linux_real_kws_acceptance",
  "macos_real_kws_acceptance",
  "integrated_production_lifecycle_acceptance",
  "measured_performance_acceptance",
]);
const pendingStatuses = new Set([
  "pending_specialized_runner_acceptance",
  "pending_integrated_acceptance",
  "pending_measurement",
]);
let implemented = 0;
let pending = 0;

for (const gate of manifest.gates) {
  if (!gate || typeof gate !== "object") fail("gate entries must be objects");
  for (const key of [
    "id",
    "title",
    "status",
    "required_for_final_closeout",
    "exact_head_required",
    "skipped_conclusion_counts_as_pass",
    "specialized_runner_required",
    "acceptance_scope",
  ]) {
    if (!(key in gate)) fail(`${gate.id ?? "unknown gate"} is missing ${key}`);
  }
  if (ids.has(gate.id)) fail(`duplicate gate id ${gate.id}`);
  ids.add(gate.id);
  if (gate.required_for_final_closeout !== true) fail(`${gate.id} must be required for final closeout`);
  if (gate.exact_head_required !== true) fail(`${gate.id} must require exact-head evidence`);
  if (gate.skipped_conclusion_counts_as_pass !== false) {
    fail(`${gate.id} must not treat skipped as passed`);
  }
  if (typeof gate.acceptance_scope !== "string" || gate.acceptance_scope.length < 20) {
    fail(`${gate.id} must describe its acceptance scope`);
  }
  requireText(gatesDoc, gate.title, `CI gate documentation for ${gate.id}`);

  if (gate.status === "implemented") {
    implemented += 1;
    if (typeof gate.specialized_runner_required !== "boolean") {
      fail(`${gate.id} must declare whether a specialized runner is required`);
    }
    if (typeof gate.workflow !== "string" || !gate.workflow.startsWith(".github/workflows/")) {
      fail(`${gate.id} must reference a workflow file`);
    }
    if (!existsSync(gate.workflow)) fail(`${gate.id} workflow does not exist: ${gate.workflow}`);
    const workflow = read(gate.workflow);
    requireText(workflow, "pull_request", `${gate.id} workflow trigger`);
    if (/continue-on-error:\s*true/u.test(workflow)) {
      fail(`${gate.id} workflow contains continue-on-error: true`);
    }
  } else if (pendingStatuses.has(gate.status)) {
    pending += 1;
    if (gate.workflow !== null) fail(`${gate.id} pending gate must not point at a passing workflow yet`);
    if (gate.specialized_runner_required !== true) {
      fail(`${gate.id} pending gate must require specialized/integrated acceptance evidence`);
    }
  } else {
    fail(`${gate.id} has unknown status ${gate.status}`);
  }
}

for (const id of requiredIds) {
  if (!ids.has(id)) fail(`required gate inventory is missing ${id}`);
}
if (implemented < 20) fail("expected at least twenty implemented policy/source/post-closeout gates");
if (pending !== 0) fail("all required post-closeout acceptance gates must be implemented before final closeout");

for (const sentence of [
  "ordinary CI alone is not final Wake Word V1 qualification",
  "A workflow with conclusion `skipped` is evidence only that its path filter or condition did not select that workflow",
  "Settings/listener lifecycle acceptance",
  "Manual shared-capture transfer acceptance",
  "Selected ASR policy acceptance",
  "Downstream first-command-word acceptance",
  "Clean-install artifact provisioning acceptance",
  "Production listener performance evidence",
  "Linux x86_64 real KWS acceptance",
  "macOS arm64 real KWS acceptance",
  "Integrated production lifecycle acceptance",
  "Measured performance acceptance",
]) {
  requireText(gatesDoc, sentence, "required-gates truthfulness documentation");
}

console.log(
  `Wake Word required-gates audit passed: ${implemented} implemented gate(s), ${pending} pending acceptance item(s), skipped conclusions do not count as pass.`,
);
