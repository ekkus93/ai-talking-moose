import { readFileSync, writeFileSync } from "node:fs";
import { spawnSync } from "node:child_process";

const contractPath = "src/generated/backendContract.json";
const original = readFileSync(contractPath, "utf8");

try {
  const contract = JSON.parse(original);
  const representative = contract.ipc_shapes?.TtsProviderDescriptor;
  if (!representative || typeof representative !== "object") {
    throw new Error(
      "generated contract is missing TtsProviderDescriptor representative",
    );
  }
  if (!("supports_pitch" in representative)) {
    throw new Error(
      "TtsProviderDescriptor representative is already missing supports_pitch",
    );
  }

  delete representative.supports_pitch;
  writeFileSync(contractPath, `${JSON.stringify(contract, null, 2)}\n`);

  const result = spawnSync(
    process.execPath,
    ["scripts/check_frontend_contract_shapes.mjs"],
    { encoding: "utf8" },
  );
  const output = `${result.stdout ?? ""}${result.stderr ?? ""}`;

  if (result.status === 0) {
    throw new Error(
      "frontend shape gate accepted deliberate TtsProviderDescriptor drift",
    );
  }
  if (!output.includes("TtsProviderDescriptor")) {
    throw new Error(`shape gate failed for an unexpected reason:\n${output}`);
  }

  console.log(
    "TTS contract negative probe: deliberate Rust representative drift was rejected.",
  );
} finally {
  writeFileSync(contractPath, original);
}
