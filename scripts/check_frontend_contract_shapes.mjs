import { readFileSync } from "node:fs";
import ts from "typescript";

const typesPath = "src/types/moose.ts";
const contractPath = "src/generated/backendContract.json";

const sourceText = readFileSync(typesPath, "utf8");
const contract = JSON.parse(readFileSync(contractPath, "utf8"));
const shapes = contract.ipc_shapes;
if (!shapes || typeof shapes !== "object" || Array.isArray(shapes)) {
  throw new Error(`${contractPath} does not contain an ipc_shapes object`);
}

const sourceFile = ts.createSourceFile(
  typesPath,
  sourceText,
  ts.ScriptTarget.Latest,
  true,
  ts.ScriptKind.TS,
);

const interfaces = new Map();
const aliases = new Map();
for (const statement of sourceFile.statements) {
  if (ts.isInterfaceDeclaration(statement)) {
    interfaces.set(statement.name.text, statement);
  } else if (ts.isTypeAliasDeclaration(statement)) {
    aliases.set(statement.name.text, statement.type);
  }
}

const primitiveCategory = (kind) => {
  if (kind === ts.SyntaxKind.StringKeyword) return "string";
  if (kind === ts.SyntaxKind.NumberKeyword) return "number";
  if (kind === ts.SyntaxKind.BooleanKeyword) return "boolean";
  if (kind === ts.SyntaxKind.NullKeyword) return "null";
  return null;
};

const categoryForType = (node, resolving = new Set()) => {
  const primitive = primitiveCategory(node.kind);
  if (primitive) return primitive;

  if (ts.isLiteralTypeNode(node)) {
    if (node.literal.kind === ts.SyntaxKind.NullKeyword) return "null";
    if (ts.isStringLiteral(node.literal)) return "string";
    if (ts.isNumericLiteral(node.literal)) return "number";
    if (
      node.literal.kind === ts.SyntaxKind.TrueKeyword ||
      node.literal.kind === ts.SyntaxKind.FalseKeyword
    ) {
      return "boolean";
    }
  }

  if (ts.isArrayTypeNode(node) || ts.isTupleTypeNode(node)) return "array";

  if (ts.isUnionTypeNode(node)) {
    const categories = new Set(
      node.types
        .map((part) => categoryForType(part, new Set(resolving)))
        .filter((category) => category !== "null"),
    );
    if (categories.size === 1) return [...categories][0];
    throw new Error(
      `Cannot reduce TypeScript union ${node.getText(sourceFile)} to one JSON category`,
    );
  }

  if (ts.isTypeLiteralNode(node)) return "object";

  if (ts.isTypeReferenceNode(node)) {
    const name = node.typeName.getText(sourceFile);
    if (name === "Array" || name === "ReadonlyArray") return "array";
    if (["Exclude", "Extract", "NonNullable"].includes(name)) {
      const [base] = node.typeArguments ?? [];
      if (!base) throw new Error(`${name} is missing its base type`);
      return categoryForType(base, resolving);
    }
    if (interfaces.has(name)) return "object";
    if (aliases.has(name)) {
      if (resolving.has(name)) {
        throw new Error(`Recursive type alias cannot be reduced: ${name}`);
      }
      const next = new Set(resolving);
      next.add(name);
      return categoryForType(aliases.get(name), next);
    }
  }

  throw new Error(
    `Unsupported TypeScript shape category for ${node.getText(sourceFile)}`,
  );
};

const categoryForValue = (value) => {
  if (value === null) return "null";
  if (Array.isArray(value)) return "array";
  return typeof value;
};

const failures = [];
for (const [interfaceName, representative] of Object.entries(shapes)) {
  const declaration = interfaces.get(interfaceName);
  if (!declaration) {
    failures.push(
      `${interfaceName}: no matching TypeScript interface in ${typesPath}`,
    );
    continue;
  }
  if (
    representative === null ||
    typeof representative !== "object" ||
    Array.isArray(representative)
  ) {
    failures.push(`${interfaceName}: Rust representative is not a JSON object`);
    continue;
  }

  const properties = new Map();
  for (const member of declaration.members) {
    if (!ts.isPropertySignature(member) || !member.type || !member.name)
      continue;
    const key = member.name.getText(sourceFile).replace(/^['"]|['"]$/g, "");
    properties.set(key, member.type);
  }

  const rustKeys = Object.keys(representative).sort();
  const tsKeys = [...properties.keys()].sort();
  const missingInTs = rustKeys.filter((key) => !properties.has(key));
  const missingInRust = tsKeys.filter((key) => !(key in representative));
  if (missingInTs.length > 0) {
    failures.push(
      `${interfaceName}: Rust-only keys: ${missingInTs.join(", ")}`,
    );
  }
  if (missingInRust.length > 0) {
    failures.push(
      `${interfaceName}: TypeScript-only keys: ${missingInRust.join(", ")}`,
    );
  }

  for (const key of rustKeys.filter((candidate) => properties.has(candidate))) {
    const rustCategory = categoryForValue(representative[key]);
    const tsCategory = categoryForType(properties.get(key));
    if (rustCategory !== tsCategory) {
      failures.push(
        `${interfaceName}.${key}: Rust JSON is ${rustCategory}, TypeScript expects ${tsCategory}`,
      );
    }
  }
}

const uncheckedInterfaces = [...interfaces.keys()]
  .filter((name) => !(name in shapes))
  .sort();

if (failures.length > 0) {
  console.error("Rust↔TypeScript IPC shape contract failed:");
  for (const failure of failures) console.error(`  ${failure}`);
  process.exit(1);
}

console.log(
  `IPC shape contract: ${Object.keys(shapes).length} Rust representatives match TypeScript interface keys and JSON categories.`,
);
if (uncheckedInterfaces.length > 0) {
  console.log(
    "TypeScript interfaces not covered by this IPC shape gate (informational):",
  );
  for (const name of uncheckedInterfaces) console.log(`  ${name}`);
}
console.log(
  "Residual gaps: numeric narrowing, enum variant completeness, optionality semantics, primitive command arguments, and per-command parameter/type/return associations are not proved by this gate.",
);
