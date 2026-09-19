export type CharacterState =
  | "hidden"
  | "appearing"
  | "idle"
  | "listening"
  | "thinking"
  | "talking"
  | "interrupted"
  | "dismissed"
  | "error";

export type ConversationLifecycle =
  "idle" | "connecting" | "listening" | "responding" | "stopping" | "failed";

export type ProviderErrorKind =
  | "auth"
  | "quota"
  | "network"
  | "protocol"
  | "setup"
  | "model"
  | "cancelled"
  | "closed"
  | "internal";

export interface ProviderError {
  kind: ProviderErrorKind;
  message: string;
  retryable: boolean;
}

export type TextProvider = "google" | "local";
export type TtsProvider = "google" | "local";

export type MouthShape = "closed" | "small" | "medium" | "wide";

export type AsrMode =
  | "moonshine_tiny_streaming"
  | "moonshine_small_streaming"
  | "gemini_live_audio";

export type AsrErrorKind =
  | "model_not_installed"
  | "model_corrupt"
  | "runtime_unavailable"
  | "model_load_failed"
  | "audio_input"
  | "inference"
  | "invalid_state"
  | "cancelled"
  | "internal";

export interface AsrError {
  kind: AsrErrorKind;
  message: string;
  retryable: boolean;
}

export type AsrModelInstallState =
  | "not_installed"
  | "downloading"
  | "verifying"
  | "installed"
  | "corrupt"
  | "incompatible"
  | "failed";

export interface AsrModelDescriptor {
  id: string;
  display_name: string;
  mode: Exclude<AsrMode, "gemini_live_audio">;
  install_state: AsrModelInstallState;
  revision: string;
  runtime_release: string;
  installed_bytes: number | null;
  expected_bytes: number;
  active: boolean;
  error_message: string | null;
}

export interface AsrDiagnostics {
  selected_mode: AsrMode;
  engine_name: string;
  model_id: string | null;
  model_revision: string | null;
  install_state: AsrModelInstallState | null;
  input_sample_rate_hz: number;
  streaming: boolean;
  metrics_snapshot: boolean;
  cpu_threads: number | null;
  queue_depth: number;
  queue_capacity: number;
  dropped_chunks: number;
  last_error: AsrError | null;
  first_partial_latency_ms: number | null;
  first_final_latency_ms: number | null;
  last_transcription_latency_ms: number | null;
  processed_audio_ms: number;
  inference_wall_time_ms: number;
  real_time_factor: number | null;
  process_cpu_time_ms: number | null;
  average_cpu_utilization_percent: number | null;
  baseline_resident_memory_bytes: number | null;
  resident_memory_bytes: number | null;
  peak_resident_memory_bytes: number | null;
}

export type WakeWordRuntimePhase =
  | "disabled"
  | "loading"
  | "listening"
  | "triggered"
  | "suspended_talking"
  | "error"
  | "shutting_down";

export interface WakeWordDiagnostics {
  enabled: boolean;
  runtime_phase: WakeWordRuntimePhase;
  engine_id: string;
  model_id: string;
  model_archive_sha256: string;
  model_license: string;
  keyword_sha256: string;
  runtime_id: string;
  runtime_license: string;
  runtime_c_api_sha256: string | null;
  platform: string;
  architecture: string;
  canonical_sample_rate_hz: number;
  canonical_channels: number;
  inference_threads: number;
  threshold: number;
  score: number;
  ring_buffer_capacity_samples: number;
  ring_buffer_capacity_ms: number;
  ring_buffer_samples: number;
  handoff_pre_roll_samples: number;
  trigger_count: number;
  last_trigger_age_ms: number | null;
  runtime_initialization_ms: number | null;
  talking_suspended: boolean;
  last_error: string | null;
}

export interface AsrModelProgressEvent {
  mode: Exclude<AsrMode, "gemini_live_audio">;
  install_state: Extract<AsrModelInstallState, "downloading" | "verifying">;
  downloaded_bytes: number;
  total_bytes: number;
  current_file: string | null;
}

export type LocalModelInstallState =
  | "not_installed"
  | "downloading"
  | "verifying"
  | "promoting"
  | "installed"
  | "failed";

export type LocalModelInstallErrorKind =
  | "invalid_catalog"
  | "unknown_model"
  | "busy"
  | "network"
  | "http"
  | "io"
  | "size_mismatch"
  | "sha256_mismatch"
  | "cancelled"
  | "promotion"
  | "corrupt_install";

export interface LocalModelInstallError {
  kind: LocalModelInstallErrorKind;
  message: string;
  retryable: boolean;
}

export interface LocalModelDescriptor {
  id: string;
  display_name: string;
  family: string;
  parameter_scale: string;
  quantization: string;
  revision: string;
  expected_bytes: number;
  installed_bytes: number | null;
  license: string;
  context_limit: number;
  recommended_max_output: number;
  install_state: LocalModelInstallState;
  active: boolean;
  error: LocalModelInstallError | null;
}

export interface LocalModelDiagnostics {
  model_root_ready: boolean;
  installs_in_progress: number;
  last_error: LocalModelInstallError | null;
}

export type LocalRuntimePhase = "ready" | "shutting_down";

export type LocalRuntimeErrorKind =
  | "shutting_down"
  | "unknown_model"
  | "model_not_installed"
  | "unsafe_artifact"
  | "initialization"
  | "model_load"
  | "model_not_loaded"
  | "invalid_request"
  | "prompt_too_long"
  | "context_creation"
  | "tokenization"
  | "chat_template"
  | "decode"
  | "output_decode"
  | "cancelled"
  | "model_delete";

export interface LocalRuntimeDiagnostics {
  selected_model_id: string;
  loaded_model_id: string | null;
  loaded_revision: string | null;
  loaded_quantization: string | null;
  loaded: boolean;
  phase: LocalRuntimePhase;
  thread_count: number;
  context_size: number;
  generation_in_progress: boolean;
  last_error_category: LocalRuntimeErrorKind | null;
  last_generation_duration_ms: number | null;
  last_prompt_tokens: number | null;
  last_output_tokens: number | null;
  last_tokens_per_second: number | null;
}

export interface LocalLlmDiagnostics {
  installer: LocalModelDiagnostics;
  selected_install_state: LocalModelInstallState;
  runtime: LocalRuntimeDiagnostics;
}

export interface LocalModelInstallProgress {
  model_id: string;
  install_state: Extract<
    LocalModelInstallState,
    "downloading" | "verifying" | "promoting"
  >;
  downloaded_bytes: number;
  total_bytes: number;
  current_file: string | null;
}

export interface GoogleModelDescriptor {
  id: string;
  display_name: string;
  capabilities: string[];
}

export interface GoogleTtsVoiceDescriptor {
  id: string;
  display_name: string;
  style: string;
}

export interface TtsVoiceDescriptor {
  id: string;
  display_name: string;
  style: string;
}

export interface TtsModelDescriptor {
  id: string;
  display_name: string;
  voices: TtsVoiceDescriptor[];
}

export interface TtsProviderDescriptor {
  id: TtsProvider;
  display_name: string;
  local: boolean;
  models: TtsModelDescriptor[];
}

export interface TtsCatalog {
  providers: TtsProviderDescriptor[];
}

export interface AudioDeviceInfo {
  name: string;
  is_default: boolean;
}

export type MicrophonePermissionState =
  | "not_determined"
  | "granted"
  | "denied"
  | "restricted"
  | "unavailable";

export interface AudioCaptureDiagnostics {
  selected_device: string | null;
  sample_rate_hz: number | null;
  sample_format: string | null;
  channels: number | null;
  active: boolean;
  input_level: number;
  dropped_chunks: number;
  last_error: string | null;
}

export interface AudioPlaybackDiagnostics {
  selected_device: string | null;
  sample_rate_hz: number | null;
  sample_format: string | null;
  channels: number | null;
  playing: boolean;
  output_level: number;
  queue_depth_samples: number;
  queue_limit_samples: number;
  dropped_samples: number;
  last_error: string | null;
}

export interface AudioDiagnostics {
  configured_input_device: string | null;
  configured_output_device: string | null;
  microphone_permission: MicrophonePermissionState;
  capture: AudioCaptureDiagnostics;
  playback: AudioPlaybackDiagnostics;
}

export interface MicrophoneTestResult {
  peak_level: number;
  diagnostics: AudioDiagnostics;
}

export interface ConnectionTestResult {
  success: boolean;
  message: string;
}

export interface OnboardingStatus {
  current_version: number;
  acknowledged_version: number | null;
  needs_acknowledgement: boolean;
}

export interface AppSettings {
  settings_version: number;
  asr_mode: AsrMode;
  wake_word_enabled: boolean;
  wake_word_phrase: string;
  launch_at_login: boolean;
  show_in_menu_bar: boolean;
  always_on_top: boolean;
  restore_position: boolean;
  unsolicited_comments: boolean;
  talkativeness: number;
  quiet_hours_enabled: boolean;
  quiet_hours_start: number;
  quiet_hours_end: number;
  max_comments_per_hour: number;
  hide_delay_seconds: number;
  idle_banter_enabled: boolean;
  idle_banter_initial_delay_minutes: number;
  idle_banter_repeat_interval_minutes: number;
  idle_banter_seed_topics: string[];
  input_device: string | null;
  output_device: string | null;
  volume: number;
  tts_provider: TtsProvider;
  google_tts_voice: string;
  local_tts_voice: string;
  live_voice: string;
  speaking_rate: number;
  pitch: number;
  text_provider: TextProvider;
  live_model: string;
  google_text_model: string;
  local_text_model: string;
  google_tts_model: string;
  local_tts_model: string;
  active_app_observation: boolean;
  window_title_observation: boolean;
  memory_enabled: boolean;
  save_transcripts: boolean;
  dry: number;
  sarcastic: number;
  friendly: number;
  absurd: number;
  helpful: number;
  verbosity: number;
}

export interface MemoryRecord {
  id: number;
  content: string;
  category: string;
  importance: number;
  created_at: string;
  last_accessed: string;
}

export interface TranscriptRecord {
  id: number;
  session_id: string;
  role: "user" | "assistant";
  content: string;
  timestamp: string;
}

export interface ToolAuditRecord {
  tool_name: string;
  started_at_ms: number;
  duration_ms: number;
  outcome: string;
}

export type InstallState =
  | "not_installed"
  | "downloading"
  | "verifying"
  | "installed"
  | "failed";

export interface LocalLlmModelInfo {
  id: string;
  name: string;
  size: number;
  installed: boolean;
  install_state: InstallState;
  progress: number;
}

export interface InstallProgress {
  model_id: string;
  install_state: InstallState;
  downloaded_bytes: number;
  total_bytes: number;
  current_file: string | null;
}

export interface ToolDefinition {
  name: string;
  description: string;
  parameters: Record<string, unknown>;
}

export interface ToolCall {
  name: string;
  arguments: Record<string, unknown>;
}

export interface ToolResult {
  success: boolean;
  content: string;
}

export interface MemoryContext {
  relevant_memories: MemoryRecord[];
  recent_transcripts: TranscriptRecord[];
}

export interface SessionInfo {
  id: string;
  started_at: string;
  ended_at: string | null;
  message_count: number;
}

export interface ConversationMessage {
  role: "user" | "assistant";
  content: string;
  timestamp: string;
}

export interface ConversationHistory {
  session: SessionInfo;
  messages: ConversationMessage[];
}

export interface AppInfo {
  version: string;
  platform: string;
  arch: string;
}
