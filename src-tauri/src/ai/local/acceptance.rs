use super::{
    local_model_entry, LocalModelInstaller, LocalModelTemplateHint, LocalRuntimeManager,
    LocalTextModel,
};
use crate::ai::traits::TextModel;
use crate::ai::types::TextRequest;
use chrono::Utc;
use serde::Serialize;
use std::fs;
use std::net::{SocketAddr, TcpStream};
use std::path::Path;
use std::sync::Arc;
use std::time::{Duration, Instant};

const REPORT_SCHEMA_VERSION: u32 = 1;
const LOAD_PROBE_MAX_TOKENS: u32 = 8;
const AMBIENT_MAX_TOKENS: u32 = 60;

#[derive(Debug, Serialize)]
pub struct LocalLlmInstallAcceptanceReport {
    pub schema_version: u32,
    pub phase: &'static str,
    pub generated_at_utc: String,
    pub git_sha: Option<String>,
    pub model_id: String,
    pub revision: String,
    pub artifact_filename: String,
    pub sha256: String,
    pub expected_bytes: u64,
    pub installed_bytes: u64,
    pub quantization: String,
    pub license: String,
    pub production_installer_verified: bool,
}

#[derive(Debug, Serialize)]
pub struct LocalLlmGenerateAcceptanceReport {
    pub schema_version: u32,
    pub phase: &'static str,
    pub generated_at_utc: String,
    pub git_sha: Option<String>,
    pub model_id: String,
    pub revision: String,
    pub sha256: String,
    pub expected_bytes: u64,
    pub quantization: String,
    pub template_hint: LocalModelTemplateHint,
    pub host_os: String,
    pub host_arch: String,
    pub host_cpu: Option<String>,
    pub available_parallelism: usize,
    pub network_denied_required: bool,
    pub network_denial_probe_passed: bool,
    pub cold_probe_wall_ms: u64,
    pub warm_probe_wall_ms: u64,
    pub cold_load_estimate_ms: u64,
    pub resident_bytes_before: Option<u64>,
    pub resident_bytes_after_cold_probe: Option<u64>,
    pub resident_bytes_delta: Option<i64>,
    pub first_token_latency_ms: Option<u64>,
    pub first_token_latency_note: &'static str,
    pub ambient_requested_max_output_tokens: u32,
    pub ambient_generation_duration_ms: u64,
    pub ambient_output_tokens: u32,
    pub ambient_tokens_per_second: Option<f32>,
    pub ambient_non_empty: bool,
    pub non_thinking_output_clean: Option<bool>,
    pub owner_drop_reload_wall_ms: u64,
    pub owner_drop_reload_success: bool,
}

fn git_sha() -> Option<String> {
    std::env::var("GITHUB_SHA")
        .ok()
        .filter(|value| !value.trim().is_empty())
}

fn host_cpu() -> Option<String> {
    std::env::var("TALKING_MOOSE_ACCEPTANCE_HOST_CPU")
        .ok()
        .filter(|value| !value.trim().is_empty())
}

fn elapsed_ms(started: Instant) -> u64 {
    u64::try_from(started.elapsed().as_millis()).unwrap_or(u64::MAX)
}

#[cfg(target_os = "linux")]
fn resident_bytes() -> Option<u64> {
    let statm = fs::read_to_string("/proc/self/statm").ok()?;
    let resident_pages = statm.split_whitespace().nth(1)?.parse::<u64>().ok()?;
    let page_size = unsafe { libc::sysconf(libc::_SC_PAGESIZE) };
    let page_size = u64::try_from(page_size).ok()?;
    resident_pages.checked_mul(page_size)
}

#[cfg(not(target_os = "linux"))]
fn resident_bytes() -> Option<u64> {
    None
}

fn resident_delta(before: Option<u64>, after: Option<u64>) -> Option<i64> {
    let before = i128::from(before?);
    let after = i128::from(after?);
    i64::try_from(after - before).ok()
}

fn linux_default_route_present() -> bool {
    #[cfg(target_os = "linux")]
    {
        let Ok(routes) = fs::read_to_string("/proc/net/route") else {
            return true;
        };
        routes.lines().skip(1).any(|line| {
            let mut fields = line.split_whitespace();
            let _interface = fields.next();
            matches!(fields.next(), Some("00000000"))
        })
    }
    #[cfg(not(target_os = "linux"))]
    {
        false
    }
}

fn network_denial_probe() -> bool {
    if linux_default_route_present() {
        return false;
    }
    let address = SocketAddr::from(([1, 1, 1, 1], 443));
    TcpStream::connect_timeout(&address, Duration::from_millis(500)).is_err()
}

fn write_report(path: &Path, report: &impl Serialize) -> Result<(), String> {
    let parent = path
        .parent()
        .ok_or_else(|| "acceptance report path has no parent directory".to_string())?;
    fs::create_dir_all(parent)
        .map_err(|_| "could not create acceptance report directory".to_string())?;
    let bytes = serde_json::to_vec_pretty(report)
        .map_err(|_| "could not serialize acceptance report".to_string())?;
    fs::write(path, bytes).map_err(|_| "could not write acceptance report".to_string())
}

fn request(prompt: &str, max_tokens: u32) -> TextRequest {
    TextRequest {
        prompt: prompt.to_string(),
        system_instruction: Some(
            "This is a bounded CPU acceptance probe. Return only a short visible answer."
                .to_string(),
        ),
        temperature: Some(0.2),
        max_tokens: Some(max_tokens),
    }
}

fn runtime_failure_message(manager: &LocalRuntimeManager, model_id: &str) -> String {
    match manager
        .diagnostics(model_id.to_string())
        .last_error_category
    {
        Some(category) => {
            format!("Local acceptance generation failed (runtime category: {category:?}).")
        }
        None => {
            "Local acceptance generation failed before runtime diagnostics identified a category."
                .to_string()
        }
    }
}

async fn generate_once(
    manager: Arc<LocalRuntimeManager>,
    installer: Arc<LocalModelInstaller>,
    model_id: &str,
    request: TextRequest,
) -> Result<(String, u64), String> {
    let model = LocalTextModel::new(manager.clone(), Ok(installer), model_id.to_string());
    let started = Instant::now();
    let response = match model.generate(request).await {
        Ok(response) => response,
        Err(_) => return Err(runtime_failure_message(&manager, model_id)),
    };
    Ok((response.text, elapsed_ms(started)))
}

pub async fn install_for_acceptance(
    model_id: &str,
    model_root: &Path,
    report_path: &Path,
) -> Result<LocalLlmInstallAcceptanceReport, String> {
    let entry = local_model_entry(model_id)
        .ok_or_else(|| "selected acceptance model is not in the bundled catalog".to_string())?;
    let installer =
        LocalModelInstaller::new(model_root.to_path_buf()).map_err(|error| error.to_string())?;
    let outcome = installer
        .install(model_id, None)
        .await
        .map_err(|error| error.to_string())?;
    if outcome.installed_bytes != entry.expected_bytes {
        return Err("production installer reported an unexpected installed byte count".to_string());
    }

    let report = LocalLlmInstallAcceptanceReport {
        schema_version: REPORT_SCHEMA_VERSION,
        phase: "install",
        generated_at_utc: Utc::now().to_rfc3339(),
        git_sha: git_sha(),
        model_id: entry.id.to_string(),
        revision: entry.revision.to_string(),
        artifact_filename: entry.artifact_filename.to_string(),
        sha256: entry.sha256.to_string(),
        expected_bytes: entry.expected_bytes,
        installed_bytes: outcome.installed_bytes,
        quantization: entry.quantization.to_string(),
        license: entry.license.to_string(),
        production_installer_verified: true,
    };
    write_report(report_path, &report)?;
    Ok(report)
}

pub async fn generate_for_acceptance(
    model_id: &str,
    model_root: &Path,
    report_path: &Path,
    require_network_denied: bool,
) -> Result<LocalLlmGenerateAcceptanceReport, String> {
    let entry = local_model_entry(model_id)
        .ok_or_else(|| "selected acceptance model is not in the bundled catalog".to_string())?;
    let network_denial_probe_passed = network_denial_probe();
    if require_network_denied && !network_denial_probe_passed {
        return Err(
            "real-model generation acceptance requires an OS-level denied network boundary"
                .to_string(),
        );
    }

    let installer = Arc::new(
        LocalModelInstaller::new(model_root.to_path_buf()).map_err(|error| error.to_string())?,
    );
    let descriptor = installer
        .descriptors(model_id)
        .into_iter()
        .find(|descriptor| descriptor.id == model_id)
        .ok_or_else(|| "selected acceptance model descriptor is unavailable".to_string())?;
    if descriptor.install_state != super::LocalModelInstallState::Installed {
        return Err("selected acceptance model is not installed and verified".to_string());
    }

    let resident_bytes_before = resident_bytes();
    let manager = Arc::new(LocalRuntimeManager::new());
    let (cold_text, cold_probe_wall_ms) = generate_once(
        manager.clone(),
        installer.clone(),
        model_id,
        request("Reply with the single word Moose.", LOAD_PROBE_MAX_TOKENS),
    )
    .await?;
    if cold_text.trim().is_empty() {
        return Err("cold Local generation returned empty output".to_string());
    }
    let resident_bytes_after_cold_probe = resident_bytes();

    let (warm_text, warm_probe_wall_ms) = generate_once(
        manager.clone(),
        installer.clone(),
        model_id,
        request("Reply with the single word Moose.", LOAD_PROBE_MAX_TOKENS),
    )
    .await?;
    if warm_text.trim().is_empty() {
        return Err("warm Local generation returned empty output".to_string());
    }

    let (ambient_text, _ambient_wall_ms) = generate_once(
        manager.clone(),
        installer.clone(),
        model_id,
        request(
            "Give one short witty Talking Moose ambient remark about a terminal window opening.",
            AMBIENT_MAX_TOKENS,
        ),
    )
    .await?;
    let diagnostics = manager.diagnostics(model_id.to_string());
    let ambient_generation_duration_ms = diagnostics
        .last_generation_duration_ms
        .ok_or_else(|| "runtime diagnostics did not record generation duration".to_string())?;
    let ambient_output_tokens = diagnostics
        .last_output_tokens
        .ok_or_else(|| "runtime diagnostics did not record output token count".to_string())?;
    let ambient_non_empty = !ambient_text.trim().is_empty();
    if !ambient_non_empty {
        return Err("ambient-style Local generation returned empty output".to_string());
    }
    let non_thinking_output_clean =
        (entry.template_hint == LocalModelTemplateHint::Qwen3NonThinking).then(|| {
            !ambient_text.contains("<think")
                && !ambient_text.contains("</think>")
                && !ambient_text.contains("<analysis")
        });
    if non_thinking_output_clean == Some(false) {
        return Err("Qwen non-thinking acceptance exposed hidden reasoning markers".to_string());
    }

    drop(manager);
    let reload_manager = Arc::new(LocalRuntimeManager::new());
    let (reload_text, owner_drop_reload_wall_ms) = generate_once(
        reload_manager,
        installer,
        model_id,
        request("Reply with the single word Moose.", LOAD_PROBE_MAX_TOKENS),
    )
    .await?;
    let owner_drop_reload_success = !reload_text.trim().is_empty();
    if !owner_drop_reload_success {
        return Err("Local model failed to generate after runtime owner reload".to_string());
    }

    let report = LocalLlmGenerateAcceptanceReport {
        schema_version: REPORT_SCHEMA_VERSION,
        phase: "generate",
        generated_at_utc: Utc::now().to_rfc3339(),
        git_sha: git_sha(),
        model_id: entry.id.to_string(),
        revision: entry.revision.to_string(),
        sha256: entry.sha256.to_string(),
        expected_bytes: entry.expected_bytes,
        quantization: entry.quantization.to_string(),
        template_hint: entry.template_hint,
        host_os: std::env::consts::OS.to_string(),
        host_arch: std::env::consts::ARCH.to_string(),
        host_cpu: host_cpu(),
        available_parallelism: std::thread::available_parallelism()
            .map(std::num::NonZeroUsize::get)
            .unwrap_or(1),
        network_denied_required: require_network_denied,
        network_denial_probe_passed,
        cold_probe_wall_ms,
        warm_probe_wall_ms,
        cold_load_estimate_ms: cold_probe_wall_ms.saturating_sub(warm_probe_wall_ms),
        resident_bytes_before,
        resident_bytes_after_cold_probe,
        resident_bytes_delta: resident_delta(
            resident_bytes_before,
            resident_bytes_after_cold_probe,
        ),
        first_token_latency_ms: None,
        first_token_latency_note: "The pinned runtime does not expose first-token timing separately; no value is fabricated.",
        ambient_requested_max_output_tokens: AMBIENT_MAX_TOKENS,
        ambient_generation_duration_ms,
        ambient_output_tokens,
        ambient_tokens_per_second: diagnostics.last_tokens_per_second,
        ambient_non_empty,
        non_thinking_output_clean,
        owner_drop_reload_wall_ms,
        owner_drop_reload_success,
    };
    write_report(report_path, &report)?;
    Ok(report)
}
