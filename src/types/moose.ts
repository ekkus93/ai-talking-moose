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

  // Legacy compatibility cache. Runtime permission state is authoritative and is
  // queried through getMicrophonePermission/getAudioDiagnostics.
  microphone_permission_granted: boolean;
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
  created_at: string;
}

export interface TranscriptRecord {
  id: number;
  session_id: string;
  role: "user" | "moose";
  text: string;
  created_at: string;
}

export interface ConnectionTestResult {
  success: boolean;
  message: string;
}
