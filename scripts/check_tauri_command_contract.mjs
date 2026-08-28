import { readFileSync } from "node:fs";

const bridgePath = "src/lib/tauriBridge.ts";
const rustLibPath = "src-tauri/src/lib.rs";

const bridgeSource = readFileSync(bridgePath, "utf8");
const rustSource = readFileSync(rustLibPath, "utf8");

const frontendCommands = new Set();
for (const match of bridgeSource.matchAll(
  /\binvoke(?:<[^;]*?>)?\(\s*"([^"]+)"/g,
)) {
  frontendCommands.add(match[1]);
}

const handlerStart = rustSource.indexOf("tauri::generate_handler![");
if (handlerStart < 0) {
  throw new Error(`Could not find tauri::generate_handler! in ${rustLibPath}`);
}

const openBracket = rustSource.indexOf("[", handlerStart);
let depth = 0;
let closeBracket = -1;
for (let index = openBracket; index < rustSource.length; index += 1) {
  const char = rustSource[index];
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

const handlerBody = rustSource.slice(openBracket + 1, closeBracket);
const registeredCommands = new Set(
  handlerBody
    .split(",")
    .map((entry) => entry.replace(/\/\/.*$/gm, "").trim())
    .filter(Boolean)
    .map((entry) => entry.split("::").at(-1)),
);

const missing = [...frontendCommands]
  .filter((command) => !registeredCommands.has(command))
  .sort();
const registeredOnly = [...registeredCommands]
  .filter((command) => !frontendCommands.has(command))
  .sort();

if (missing.length > 0) {
  console.error("Frontend invokes Tauri commands that Rust does not register:");
  for (const command of missing) console.error(`  ${command}`);
  process.exit(1);
}

console.log(
  `Tauri command contract: ${frontendCommands.size}/${frontendCommands.size} frontend command names are registered.`,
);
if (registeredOnly.length > 0) {
  console.log("Registered but not frontend-invoked (informational):");
  for (const command of registeredOnly) console.log(`  ${command}`);
}
