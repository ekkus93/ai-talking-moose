use super::installer::{LocalModelDiagnostics, LocalModelInstallState};
use super::runtime::types::LocalRuntimeDiagnostics;
use serde::Serialize;

#[derive(Debug, Clone, Serialize)]
pub struct LocalLlmDiagnostics {
    pub installer: LocalModelDiagnostics,
    pub selected_install_state: Option<LocalModelInstallState>,
    pub runtime: LocalRuntimeDiagnostics,
}
