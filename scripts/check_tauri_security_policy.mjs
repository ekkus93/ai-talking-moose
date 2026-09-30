import { readFileSync } from "node:fs";

const readJson = (path) => JSON.parse(readFileSync(path, "utf8"));
const read = (path) => readFileSync(path, "utf8");
const fail = (message) => {
  throw new Error(`Tauri security policy failed: ${message}`);
};

const config = readJson("src-tauri/tauri.conf.json");
const capabilities = readJson("src-tauri/capabilities/default.json");
const mooseSprite = read("src/components/Moose/MooseSprite.tsx");
const sprites = read("src/lib/sprites.ts");

const csp = config.app?.security?.csp;
if (typeof csp !== "string" || csp.trim().length === 0) {
  fail("tauri.conf.json must use an explicit non-null CSP");
}
if (/\*/u.test(csp)) {
  fail("CSP must not contain wildcard source directives");
}
for (const directive of [
  "default-src 'self'",
  "script-src 'self'",
  "style-src 'self' 'unsafe-inline'",
  "img-src 'self' data:",
  "connect-src ipc: http://ipc.localhost",
  "object-src 'none'",
  "frame-src 'none'",
  "base-uri 'self'",
]) {
  if (!csp.includes(directive)) {
    fail(`CSP is missing required directive: ${directive}`);
  }
}

const windows = capabilities.windows ?? [];
if (!Array.isArray(windows) || windows.length !== 1 || windows[0] !== "main") {
  fail("default capability must be scoped to the main window only");
}
if (windows.includes("*")) {
  fail("default capability must not use wildcard window scope");
}

const permissions = capabilities.permissions ?? [];
for (const permission of permissions) {
  if (typeof permission !== "string") fail("capability permissions must be strings");
  if (permission.startsWith("core:webview:")) {
    fail(`unused webview permission remains: ${permission}`);
  }
  if (permission.startsWith("opener:")) {
    fail(`unused opener permission remains: ${permission}`);
  }
}
for (const permission of [
  "core:default",
  "core:window:default",
  "core:window:allow-start-dragging",
  "core:window:allow-set-size",
]) {
  if (!permissions.includes(permission)) {
    fail(`missing required window permission: ${permission}`);
  }
}

if (!mooseSprite.includes("dangerouslySetInnerHTML={{ __html: svgContent }}")) {
  fail("MooseSprite raw SVG insertion invariant changed without policy update");
}
if (!mooseSprite.includes("getMooseSprite(state, mouth, isBlinking)")) {
  fail("MooseSprite raw SVG must come only from getMooseSprite state rendering");
}
if (/fetch\s*\(/u.test(sprites) || /localStorage/u.test(sprites) || /document\./u.test(sprites)) {
  fail("sprite source must remain application-controlled and offline/static");
}
if (!sprites.includes("renderMooseSvg")) {
  fail("sprite source must render from application-controlled structured state");
}

console.log("Tauri security policy passed: CSP is explicit, default capability is main-window scoped, unused opener/webview powers are absent, and MooseSprite raw SVG remains application-controlled.");
