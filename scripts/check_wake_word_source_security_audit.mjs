import { readFileSync } from "node:fs";

const read = (path) => readFileSync(path, "utf8");
const fail = (message) => {
  throw new Error(`Wake Word source/security audit failed: ${message}`);
};
const requireText = (source, needle, label) => {
  if (!source.includes(needle)) fail(`${label} is missing ${needle}`);
};

const runtime = read("src-tauri/src/asr/wake_word_runtime.rs");
const engine = read("src-tauri/src/app/wake_word_engine.rs");
const composition = read("src-tauri/src/app/wake_word_composition.rs");
const state = read("src-tauri/src/app/state.rs");
const capture = read("src-tauri/src/app/wake_word_authoritative_capture.rs");
const router = read("src-tauri/src/app/wake_word_pcm_router.rs");
const lifecycle = read("src-tauri/src/app/wake_word_command_lifecycle.rs");
const appModule = read("src-tauri/src/app/mod.rs");

const managerDefinitions = [runtime, engine, composition, state, capture, router, lifecycle]
  .map((source) => (source.match(/struct WakeWordRuntimeManager\b/gu) ?? []).length)
  .reduce((sum, count) => sum + count, 0);
if (managerDefinitions !== 1) {
  fail(`expected one WakeWordRuntimeManager definition, found ${managerDefinitions}`);
}

requireText(state, "pub audio_capture: Arc<Mutex<AudioCapture>>", "AppState authoritative capture");
requireText(state, "pub wake_word_runtime: WakeWordApplicationRuntime", "AppState Wake runtime ownership");
requireText(capture, "from_shared_capture", "Wake shared-capture boundary");
requireText(capture, "self.capture.lock().stop();", "Wake command handoff capture stop");
const productionCapture = capture.split("#[cfg(test)]")[0];
if (
  productionCapture.includes("AudioCapture::new()") ||
  productionCapture.includes("AudioCapture::default()")
) {
  fail("production Wake capture constructs a competing AudioCapture owner");
}

requireText(router, "accept_trigger", "one-trigger router boundary");
requireText(router, "return_to_wake_listening", "Wake router reset boundary");
requireText(lifecycle, "SuspendedTalking", "Talking suspension policy");
requireText(lifecycle, "wake_word_enabled", "latest-setting terminal resolution");
requireText(composition, "record_capture_error", "capture-error fail-closed boundary");
requireText(composition, "begin_shutdown", "Wake shutdown boundary");
requireText(composition, "capture_consumer", "AppState Wake capture-consumer composition boundary");
requireText(
  composition,
  "CanonicalWakePcmRouter::new(self.manager.clone(), engine)",
  "shared runtime manager capture routing",
);

requireText(engine, "verify_model_artifacts", "model identity verification");
requireText(engine, "verify_runtime_artifacts", "runtime identity verification");
requireText(engine, "NativeArchitecture::ElfX86_64", "Linux native architecture verification");
requireText(engine, "NativeArchitecture::MachOArm64", "macOS native architecture verification");
requireText(engine, "pub trait SherpaKwsEngine", "local KWS engine boundary");

const productionEngine = engine.split("#[cfg(test)]")[0];
for (const forbidden of [
  "reqwest",
  "ureq",
  "hyper::",
  "tokio_tungstenite",
  "WebSocket",
  "http://",
  "https://",
]) {
  if (productionEngine.includes(forbidden)) {
    fail(`production Wake KWS engine contains network dependency/reference ${forbidden}`);
  }
}
for (const forbidden of ["Google", "Gemini", "OpenAI", "transcribe", "transcription"]) {
  if (productionEngine.includes(forbidden)) {
    fail(`production Wake KWS engine contains full/cloud ASR reference ${forbidden}`);
  }
}

requireText(
  appModule,
  "mod wake_word_authoritative_capture",
  "authoritative Wake capture module registration",
);
console.log(
  "Wake Word source/security audit passed: ownership, capture, lifecycle, artifact, architecture, app composition, and offline/provider-separation invariants are present.",
);
