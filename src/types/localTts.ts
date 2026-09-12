import type { TtsProvider } from "./moose";

export type LocalTtsInstallState =
  | "not_installed"
  | "downloading"
  | "verifying"
  | "promoting"
  | "installed"
  | "failed";

export type LocalTtsInstallErrorKind =
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

export type LocalTtsRuntimePhase =
  "unloaded" | "loading" | "generating" | "ready" | "failed" | "shutting_down";

export type LocalTtsRuntimeErrorKind =
  | "shutting_down"
  | "cancelled"
  | "unknown_model"
  | "model_not_installed"
  | "verification"
  | "unsupported_platform"
  | "runtime_unavailable"
  | "invalid_input"
  | "invalid_voice"
  | "unsupported_config"
  | "inference"
  | "model_load"
  | "model_delete";

export interface LocalTtsModelError {
  kind: LocalTtsInstallErrorKind;
  message: string;
  retryable: boolean;
}

export interface LocalTtsModelDescriptor {
  id: string;
  display_name: string;
  version: string;
  expected_bytes: number;
  installed_bytes: number | null;
  license: string;
  install_state: LocalTtsInstallState;
  active: boolean;
  error: LocalTtsModelError | null;
}

export interface LocalTtsInstallProgress {
  model_id: string;
  artifact_filename: string | null;
  install_state: "downloading" | "verifying" | "promoting";
  downloaded_bytes: number;
  total_bytes: number;
}

export interface LocalTtsRuntimeStatus {
  selected_model_id: string;
  loaded_model_id: string | null;
  loaded_revision: string | null;
  runtime_compatibility_version: number | null;
  phase: LocalTtsRuntimePhase;
  sample_rate_hz: number | null;
  inference_thread_count: number | null;
  last_model_load_duration_ms: number | null;
  last_synthesis_duration_ms: number | null;
  last_generated_audio_duration_ms: number | null;
  last_real_time_factor: number | null;
  last_error_category: LocalTtsRuntimeErrorKind | null;
}

export interface LocalTtsDiagnostics {
  provider: TtsProvider;
  selected_model_id: string;
  selected_voice_id: string;
  install_state: LocalTtsInstallState;
  expected_bytes: number;
  installed_bytes: number | null;
  installer_error_category: LocalTtsInstallErrorKind | null;
  installer_error_retryable: boolean | null;
  runtime: LocalTtsRuntimeStatus;
}
