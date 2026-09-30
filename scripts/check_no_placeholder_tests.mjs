import { readdirSync, readFileSync } from "node:fs";
import { join } from "node:path";

const roots = ["src"];
const testFilePattern = /\.(?:test|spec)\.[cm]?[jt]sx?$/u;
const placeholderPatterns = [
  {
    name: "expect(true).toBe(true)",
    regex: /expect\s*\(\s*true\s*\)\s*\.toBe\s*\(\s*true\s*\)/u,
  },
  {
    name: "expect(false).toBe(false)",
    regex: /expect\s*\(\s*false\s*\)\s*\.toBe\s*\(\s*false\s*\)/u,
  },
];

const files = [];
const walk = (directory) => {
  for (const entry of readdirSync(directory, { withFileTypes: true })) {
    const path = join(directory, entry.name);
    if (entry.isDirectory()) {
      walk(path);
    } else if (testFilePattern.test(entry.name)) {
      files.push(path);
    }
  }
};

for (const root of roots) walk(root);

const violations = [];
for (const path of files) {
  const source = readFileSync(path, "utf8");
  for (const pattern of placeholderPatterns) {
    if (pattern.regex.test(source)) {
      violations.push(`${path}: pass-by-construction placeholder ${pattern.name}`);
    }
  }
}

if (violations.length > 0) {
  throw new Error(
    `Placeholder test policy failed:\n${violations.map((item) => `- ${item}`).join("\n")}`,
  );
}

console.log(`Placeholder test policy passed for ${files.length} frontend test file(s).`);
