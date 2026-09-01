import { readFileSync } from "node:fs";

const bridgePath = "src/lib/tauriBridge.ts";
const rustLibPath = "src-tauri/src/lib.rs";

const bridgeSource = readFileSync(bridgePath, "utf8");
const rustSource = readFileSync(rustLibPath, "utf8");

const extractFrontendCommands = (source) => {
  const commands = new Set();
  for (const match of source.matchAll(
    /\binvoke(?:<[^;]*?>)?\(\s*"([^"]+)"/g,
  )) {
    commands.add(match[1]);
  }
  return commands;
};

const extractRegisteredCommands = (source) => {
  const handlerStart = source.indexOf("tauri::generate_handler![");
  if (handlerStart < 0) {
    throw new Error(`Could not find tauri::generate_handler! in ${rustLibPath}`);
  }

  const openBracket = source.indexOf("[", handlerStart);
  let depth = 0;
  let closeBracket = -1;
  for (let index = openBracket; index < source.length; index += 1) {
    const char = source[index];
    if (char === "[") depth += 1;
    if (char === "]") {
      depth -= 1;
      if (depth === 0) {
        closeBracket = index;
        break;
      }
    }
  }
  if (closeBracket < 0) {
    throw new Error(`Could not parse tauri::generate_handler! in ${rustLibPath}`);
  }

  const handlerBody = source.slice(openBracket + 1, closeBracket);
  return new Set(
    handlerBody
      .split(",")
      .map((entry) => entry.replace(/\/\/.*$/gm, "").trim())
      .filter(Boolean)
      .map((entry) => entry.split("::").at(-1)),
  );
};

const compareCommands = (frontendCommands, registeredCommands) => ({
  missing: [...frontendCommands]
    .filter((command) => !registeredCommands.has(command))
    .sort(),
  registeredOnly: [...registeredCommands]
    .filter((command) => !frontendCommands.has(command))
    .sort(),
});

const frontendCommands = extractFrontendCommands(bridgeSource);
const registeredCommands = extractRegisteredCommands(rustSource);
const { missing, registeredOnly } = compareCommands(
  frontendCommands,
  registeredCommands,
);

if (missing.length > 0) {
  console.error("Frontend invokes Tauri commands that Rust does not register:");
  for (const command of missing) console.error(`  ${command}`);
  process.exit(1);
}

// LLM-102 negative probe: model a deliberate Rust registration rename without changing the
// frontend bridge. The same comparison used by the real gate must reject the original command.
const negativeProbeCommand = "test_local_llm_model";
if (!frontendCommands.has(negativeProbeCommand)) {
  throw new Error(
    `Negative command-contract probe target is not frontend-invoked: ${negativeProbeCommand}`,
  );
}
if (!registeredCommands.has(negativeProbeCommand)) {
  throw new Error(
    `Negative command-contract probe target is not Rust-registered: ${negativeProbeCommand}`,
  );
}
const renamedRegistration = new Set(registeredCommands);
renamedRegistration.delete(negativeProbeCommand);
renamedRegistration.add(`${negativeProbeCommand}_negative_probe_rename`);
const negativeProbe = compareCommands(frontendCommands, renamedRegistration);
if (!negativeProbe.missing.includes(negativeProbeCommand)) {
  throw new Error(
    `Negative command-contract rename probe did not detect ${negativeProbeCommand}`,
  );
}

console.log(
  `Tauri command contract: ${frontendCommands.size}/${frontendCommands.size} frontend command names are registered.`,
);
console.log(
  `Negative rename probe: ${negativeProbeCommand} is rejected when the Rust registration name changes.`,
);
if (registeredOnly.length > 0) {
  console.log("Registered but not frontend-invoked (informational):");
  for (const command of registeredOnly) console.log(`  ${command}`);
}
