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
  install_state: Extract<
    LocalTtsInstallState,
    "downloading" | "verifying" | "promoting"
  >;
  downloaded_bytes: number;
  total_bytes: number;
}
