import { readFileSync, writeFileSync } from "node:fs";
import { spawnSync } from "node:child_process";

const backendContractPath = "src/generated/backendContract.json";
const localTtsContractPath = "src/generated/localTtsBackendContract.json";
const originals = new Map([
  [backendContractPath, readFileSync(backendContractPath, "utf8")],
  [localTtsContractPath, readFileSync(localTtsContractPath, "utf8")],
]);

const runProbe = (contractPath, shapeName, fieldName, nestedPath = []) => {
  const original = originals.get(contractPath);
  const contract = JSON.parse(original);
  const shapes =
    contractPath === backendContractPath ? contract.ipc_shapes : contract;
  const representative = shapes?.[shapeName];
  if (!representative || typeof representative !== "object") {
    throw new Error(
      `${contractPath} is missing ${shapeName} representative`,
    );
  }

  let target = representative;
  for (const segment of nestedPath) {
    target = target?.[segment];
    if (!target || typeof target !== "object") {
      throw new Error(
        `${shapeName}.${nestedPath.join(".")} is missing from ${contractPath}`,
      );
    }
  }
  if (!(fieldName in target)) {
    throw new Error(
      `${shapeName} representative is already missing ${fieldName}`,
    );
  }

  delete target[fieldName];
  writeFileSync(contractPath, `${JSON.stringify(contract, null, 2)}\n`);

  const result = spawnSync(
    process.execPath,
    ["scripts/check_frontend_contract_shapes.mjs"],
    { encoding: "utf8" },
  );
  const output = `${result.stdout ?? ""}${result.stderr ?? ""}`;

  if (result.status === 0) {
    throw new Error(
      `frontend shape gate accepted deliberate ${shapeName}.${fieldName} drift`,
    );
  }
  if (!output.includes(shapeName)) {
    throw new Error(`shape gate failed for an unexpected reason:\n${output}`);
  }

  writeFileSync(contractPath, original);
};

try {
  runProbe(
    backendContractPath,
    "TtsProviderDescriptor",
    "supports_pitch",
  );
  runProbe(
    localTtsContractPath,
    "LocalTtsDiagnostics",
    "selected_voice_id",
  );
  runProbe(
    localTtsContractPath,
    "LocalTtsRuntimeStatus",
    "last_real_time_factor",
  );

  console.log(
    "TTS contract negative probes: provider, Local TTS diagnostics, and Local TTS runtime field drift were rejected.",
  );
} finally {
  for (const [path, original] of originals) writeFileSync(path, original);
}
