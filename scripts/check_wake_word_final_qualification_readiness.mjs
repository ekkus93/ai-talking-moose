import { readFileSync } from "node:fs";

const manifest = JSON.parse(readFileSync("docs/wake-word-required-gates.json", "utf8"));
const fail = (message) => {
  throw new Error(`Wake Word final qualification readiness failed: ${message}`);
};

if (manifest.ordinary_ci_alone_is_final_qualification !== false) {
  fail("ordinary CI is incorrectly allowed to qualify Wake Word V1");
}
if (manifest.skipped_conclusion_counts_as_pass !== false) {
  fail("skipped workflow conclusions are incorrectly allowed as acceptance evidence");
}

const required = manifest.gates.filter((gate) => gate.required_for_final_closeout === true);
if (required.length === 0) fail("required gate inventory is empty");

const pending = required.filter((gate) => gate.status !== "implemented");
if (pending.length > 0) {
  const summary = pending.map((gate) => `${gate.id}=${gate.status}`).join(", ");
  fail(`final feature head is not eligible; pending required acceptance: ${summary}`);
}

for (const gate of required) {
  if (gate.exact_head_required !== true) fail(`${gate.id} does not require exact-head evidence`);
  if (gate.skipped_conclusion_counts_as_pass !== false) {
    fail(`${gate.id} incorrectly permits skipped evidence`);
  }
}

console.log(
  `Wake Word final qualification readiness passed for ${required.length} required gates. ` +
    "This readiness check does not replace recording the exact-head run IDs required by WWR-950.",
);
