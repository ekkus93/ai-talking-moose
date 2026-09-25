import { readFileSync } from "node:fs";

// WWR-900 final audit evidence uses this checker as the executable guardrail for
// Wake Word ownership, capture, lifecycle, command activation, provider separation,
// artifact/runtime verification, native architecture verification, and memory-only
// PCM retention invariants.
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
const activation = read("src-tauri/src/app/wake_word_command_activation.rs");
const ingress = read("src-tauri/src/app/wake_word_command_asr_ingress.rs");
const handoffAudio = read("src-tauri/src/app/wake_word_command_handoff.rs");
const appModule = read("src-tauri/src/app/mod.rs");

const managerDefinitions = [runtime, engine, composition, state, capture, router, lifecycle]
  .map((source) => (source.match(/struct WakeWordRuntimeManager\b/gu) ?? []).length)
  .reduce((sum, count) => sum + count, 0);
if (managerDefinitions !== 1) {
  fail(`expected one WakeWordRuntimeManager definition, found ${managerDefinitions}`);
}

requireText(state, "pub audio_capture: Arc<Mutex<AudioCapture>>", "AppState authoritative capture");
requireText(state, "pub wake_word_runtime: WakeWordApplicationRuntime", "AppState Wake runtime ownership");
requireText(composition, "capture_consumer", "AppState Wake capture composition boundary");
requireText(composition, "CanonicalWakePcmRouter::new", "AppState Wake router composition boundary");
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
requireText(
  router,
  "handoff_audio_preserves_wake_phrase_tail_and_first_command_word_contiguously",
  "wake-to-ASR chronological handoff regression",
);
requireText(
  router,
  "repeated_positive_frames_after_trigger_do_not_duplicate_command_activation",
  "repeated-trigger debounce regression",
);
requireText(
  router,
  "later_phrase_after_return_to_listening_yields_second_trigger_without_cooldown",
  "post-resume trigger regression",
);
requireText(
  router,
  "disabled_runtime_never_feeds_kws",
  "disabled Wake no-feed regression",
);
requireText(lifecycle, "SuspendedTalking", "Talking suspension policy");
requireText(lifecycle, "wake_word_enabled", "latest-setting terminal resolution");
requireText(composition, "record_capture_error", "capture-error fail-closed boundary");
requireText(composition, "begin_shutdown", "Wake shutdown boundary");

requireText(activation, "activate_wake_command_once", "single command activation boundary");
requireText(activation, "deliver_once", "single-use Wake command handoff");
requireText(
  activation,
  "suspend_for_command_interaction",
  "Wake suspension before command ASR activation",
);
requireText(
  activation,
  "command_asr_startup_failure_returns_to_listening_without_stale_replay",
  "command ASR startup recovery regression",
);
requireText(ingress, "trait WakeCommandAsrIngress", "provider-neutral command ASR ingress");
requireText(ingress, "LocalAsrPipeline", "normal local command ASR ingress implementation");
requireText(ingress, "struct WakeCommandAsrHandoff", "single-use handoff coordinator");
requireText(ingress, "self.audio.take()", "single-use handoff consumption");
requireText(
  ingress,
  "accepted_trigger_payload_is_delivered_exactly_once",
  "single-use command activation regression",
);

const productionActivation = activation.split("#[cfg(test)]")[0];
const productionIngress = ingress.split("#[cfg(test)]")[0];
for (const [label, source] of [
  ["Wake command activation", productionActivation],
  ["Wake command ASR ingress", productionIngress],
]) {
  for (const forbidden of ["GoogleLiveProvider", "GoogleAuth", "reqwest", "http://", "https://"]) {
    if (source.includes(forbidden)) {
      fail(`${label} contains cloud/network bypass reference ${forbidden}`);
    }
  }
}

for (const [label, source] of [
  ["Wake runtime", runtime.split("#[cfg(test)]")[0]],
  ["Wake PCM router", router.split("#[cfg(test)]")[0]],
  ["Wake command activation", productionActivation],
  ["Wake command ASR ingress", productionIngress],
  ["Wake command handoff audio", handoffAudio.split("#[cfg(test)]")[0]],
]) {
  for (const forbidden of ["File::create", "OpenOptions", "std::fs::write", "fs::write("]) {
    if (source.includes(forbidden)) {
      fail(`${label} can persist retained Wake PCM through ${forbidden}`);
    }
  }
}

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
  "Wake Word source/security audit passed: ownership, capture, lifecycle, memory-only PCM retention, command activation/ASR ingress, router handoff/debounce, artifact, architecture, and offline/provider-separation invariants are present.",
);
