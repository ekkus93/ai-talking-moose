export type CharacterState =
  | "hidden"
  | "appearing"
  | "idle"
  | "listening"
  | "thinking"
  | "talking"
  | "interrupted"
  | "annoyed"
  | "sleeping"
  | "dismissed"
  | "muted"
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
  | "closed"
  | "internal";

export interface ProviderError {
  kind: ProviderErrorKind;
  message: string;
  retryable: boolean;
}

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

export interface AsrModelProgressEvent {
  mode: Exclude<AsrMode, "gemini_live_audio">;
  install_state: Extract<AsrModelInstallState, "downloading" | "verifying">;
  downloaded_bytes: number;
  total_bytes: number;
  current_file: string | null;
}

export type MicrophonePermissionState =
  "not_requested" | "granted" | "denied" | "unavailable";

export interface AudioDeviceInfo {
  id: string;
  name: string;
  is_default: boolean;
}

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

export type ToolPermissionLevel =
  | "safe_read_only"
  | "character_action"
  | "memory_mutation"
  | "denied";

export type ToolPermissionOutcome =
  | "not_evaluated"
  | "allowed"
  | "denied"
  | "confirmation_required";

export type ToolResultCategory =
  | "success"
  | "not_found"
  | "input_too_large"
  | "invalid_arguments"
  | "permission_denied"
  | "confirmation_required"
  | "concurrency_limit"
  | "timeout"
  | "output_too_large"
  | "execution_failed";

export interface ToolAuditRecord {
  tool_name: string;
  timestamp: string;
  duration_ms: number;
  permission: ToolPermissionLevel;
  permission_outcome: ToolPermissionOutcome;
  result_category: ToolResultCategory;
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

export interface OnboardingStatus {
  current_version: number;
  acknowledged_version: number | null;
  needs_acknowledgement: boolean;
}

export interface AppSettings {
  settings_version: number;
  asr_mode: AsrMode;

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

  input_device: string | null;
  output_device: string | null;
  volume: number;
  tts_voice: string;
  speaking_rate: number;
  pitch: number;

  provider: "google" | "fake";
  live_model: string;
  text_model: string;
  tts_model: string;

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
  fact: string;
  category: string;
  source: string;
  confidence: number;
  created_at: string;
  updated_at: string;
}

export interface TranscriptRecord {
  id: number;
  session_id: string;
  role: "user" | "moose";
  text: string;
  created_at: string;
}

export type GoogleModelCapability = "live_audio" | "text_generation";

export interface GoogleModelDescriptor {
  id: string;
  display_name: string;
  capabilities: GoogleModelCapability[];
}

export interface GoogleTtsVoiceDescriptor {
  id: string;
  style: string;
}

export interface ConnectionTestResult {
  success: boolean;
  message: string;
}
