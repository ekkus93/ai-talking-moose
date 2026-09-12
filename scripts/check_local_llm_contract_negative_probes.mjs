import {
  mkdirSync,
  mkdtempSync,
  readFileSync,
  rmSync,
  writeFileSync,
} from "node:fs";
import { tmpdir } from "node:os";
import { dirname, join, resolve } from "node:path";
import { spawnSync } from "node:child_process";

const repoRoot = process.cwd();
const workspace = mkdtempSync(join(tmpdir(), "talking-moose-p8-contract-"));

const copyText = (relativePath) => {
  const destination = join(workspace, relativePath);
  mkdirSync(dirname(destination), { recursive: true });
  writeFileSync(
    destination,
    readFileSync(join(repoRoot, relativePath), "utf8"),
    "utf8",
  );
};

const runChecker = (relativeScriptPath, cwd) =>
  spawnSync(process.execPath, [resolve(repoRoot, relativeScriptPath)], {
    cwd,
    encoding: "utf8",
  });

const outputFor = (result) => `${result.stdout ?? ""}\n${result.stderr ?? ""}`;

const requireSuccess = (label, result) => {
  if (result.status !== 0) {
    throw new Error(
      `${label} positive control failed unexpectedly:\n${outputFor(result)}`,
    );
  }
};

const requireFailureContaining = (label, result, fragments) => {
  if (result.status === 0) {
    throw new Error(`${label} negative probe unexpectedly passed`);
  }
  const output = outputFor(result);
  for (const fragment of fragments) {
    if (!output.includes(fragment)) {
      throw new Error(
        `${label} failed, but not for the expected reason (${fragment}):\n${output}`,
      );
    }
  }
};

try {
  requireSuccess(
    "frontend IPC shape checker",
    runChecker("scripts/check_frontend_contract_shapes.mjs", repoRoot),
  );
  requireSuccess(
    "Tauri command checker",
    runChecker("scripts/check_tauri_command_contract.mjs", repoRoot),
  );

  for (const relativePath of [
    "src/types/moose.ts",
    "src/types/localTts.ts",
    "src/generated/backendContract.json",
    "src/generated/localTtsBackendContract.json",
    "src/lib/tauriBridge.ts",
    "src-tauri/src/lib.rs",
  ]) {
    copyText(relativePath);
  }

  const contractPath = join(workspace, "src/generated/backendContract.json");
  const contract = JSON.parse(readFileSync(contractPath, "utf8"));
  const runtimeDiagnostics = contract.ipc_shapes?.LocalRuntimeDiagnostics;
  if (
    !runtimeDiagnostics ||
    typeof runtimeDiagnostics !== "object" ||
    Array.isArray(runtimeDiagnostics) ||
    !("generation_in_progress" in runtimeDiagnostics)
  ) {
    throw new Error(
      "LocalRuntimeDiagnostics.generation_in_progress is missing from the generated IPC representative",
    );
  }

  runtimeDiagnostics.generation_in_progress_negative_probe_rename =
    runtimeDiagnostics.generation_in_progress;
  delete runtimeDiagnostics.generation_in_progress;
  writeFileSync(contractPath, `${JSON.stringify(contract, null, 2)}\n`, "utf8");

  requireFailureContaining(
    "LocalRuntimeDiagnostics field rename",
    runChecker("scripts/check_frontend_contract_shapes.mjs", workspace),
    [
      "LocalRuntimeDiagnostics",
      "Rust-only keys: generation_in_progress_negative_probe_rename",
      "TypeScript-only keys: generation_in_progress",
    ],
  );

  const rustLibPath = join(workspace, "src-tauri/src/lib.rs");
  const rustLib = readFileSync(rustLibPath, "utf8");
  const registration = "            get_local_llm_diagnostics,";
  const renamedRegistration =
    "            get_local_llm_diagnostics_negative_probe_rename,";
  const occurrences = rustLib.split(registration).length - 1;
  if (occurrences !== 1) {
    throw new Error(
      `Expected exactly one get_local_llm_diagnostics registration, found ${occurrences}`,
    );
  }
  writeFileSync(
    rustLibPath,
    rustLib.replace(registration, renamedRegistration),
    "utf8",
  );

  requireFailureContaining(
    "Local LLM diagnostics command registration rename",
    runChecker("scripts/check_tauri_command_contract.mjs", workspace),
    [
      "Frontend invokes Tauri commands that Rust does not register:",
      "get_local_llm_diagnostics",
    ],
  );

  console.log(
    "Local LLM contract negative probes: diagnostics field drift and diagnostics command-name drift are rejected by the production checkers.",
  );
} finally {
  rmSync(workspace, { recursive: true, force: true });
}
