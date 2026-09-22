import { existsSync, readFileSync } from "node:fs";

const read = (path) => readFileSync(path, "utf8");
const fail = (message) => {
  throw new Error(`Wake Word final gate policy failed: ${message}`);
};
const requireText = (source, needle, label) => {
  if (!source.includes(needle)) fail(`${label} is missing ${needle}`);
};

const requiredWorkflows = [
  ".github/workflows/wake-word-corpus.yml",
  ".github/workflows/wake-word-corpus-contract.yml",
  ".github/workflows/wake-word-native-packaging.yml",
  ".github/workflows/wake-word-lifecycle-stability.yml",
  ".github/workflows/wake-word-performance-evidence.yml",
  ".github/workflows/wake-word-privacy-audit.yml",
  ".github/workflows/wake-word-documentation-audit.yml",
  ".github/workflows/wake-word-source-security-audit.yml",
];

for (const path of requiredWorkflows) {
  if (!existsSync(path)) fail(`required specialized workflow is missing: ${path}`);
  const workflow = read(path);
  requireText(workflow, "pull_request:", path);
  requireText(workflow, "push:", path);
}

const gates = read("docs/WAKE_WORD_V1_CI_GATES.md");
for (const phrase of [
  "Ordinary CI alone is not final Wake Word V1 qualification.",
  "A skipped corpus gate is not evidence that real corpus acceptance passed.",
  "A skipped lifecycle workflow is not lifecycle acceptance evidence.",
  "A final Wake Word V1 feature head is not eligible based on ordinary CI alone.",
  "A workflow with conclusion `skipped` is evidence only that its path filter or condition did not select that workflow.",
  "Linux x86_64 real KWS acceptance requires",
  "macOS arm64 real KWS acceptance requires",
]) {
  requireText(gates, phrase, "Wake Word gate documentation");
}

const remediation = read("docs/WAKE_WORD_V1_REMEDIATION_TODO_2026-09-17.md");
for (const requirement of [
  "Linux x86_64 real KWS gate passes.",
  "macOS arm64 real KWS gate passes.",
  "Native package/architecture gate passes.",
  "Integrated lifecycle stability gate passes.",
  "Performance evidence is recorded and accepted.",
  "Privacy/security audit passes.",
  "Documentation audit passes.",
]) {
  requireText(remediation, requirement, "final qualification checklist");
}

console.log(
  `Wake Word final gate policy passed: ${requiredWorkflows.length} specialized workflows are present and final qualification cannot be represented by ordinary CI alone.`,
);
