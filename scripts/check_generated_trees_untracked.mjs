import { spawnSync } from "node:child_process";

const generatedTrees = ["node_modules", "dist"];
const result = spawnSync("git", ["ls-files", "-z", "--", ...generatedTrees], {
  encoding: "utf8",
});

if (result.error) {
  console.error(`Failed to run git ls-files: ${result.error.message}`);
  process.exit(2);
}
if (result.status !== 0) {
  process.stderr.write(result.stderr ?? "");
  process.exit(result.status ?? 2);
}

const tracked = (result.stdout ?? "")
  .split("\0")
  .filter(Boolean)
  .sort();

if (tracked.length > 0) {
  console.error(
    "Generated frontend trees must not be committed. Remove these paths from the Git index:",
  );
  for (const path of tracked) {
    console.error(`  ${path}`);
  }
  process.exit(1);
}

console.log("Generated frontend trees are absent from the Git index.");
