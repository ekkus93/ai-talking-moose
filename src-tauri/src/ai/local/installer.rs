use super::catalog::{local_model_entry, validate_local_model_catalog, LocalModelCatalogEntry};
use async_trait::async_trait;
use futures_util::StreamExt;
use parking_lot::Mutex;
use ring::digest::{Context as Sha256Context, SHA256};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fmt;
use std::fs;
use std::io::{BufReader, Read, Write};
use std::path::{Path, PathBuf};
use std::sync::{Arc, OnceLock};
use tokio::io::AsyncWriteExt;
use tokio_util::sync::CancellationToken;
use uuid::Uuid;

const STAGING_DIR: &str = ".staging";
const INSTALL_MARKER: &str = ".talking-moose-local-llm.json";
const INSTALL_MARKER_VERSION: u32 = 1;
const VERIFY_BUFFER_BYTES: usize = 1024 * 1024;

static GLOBAL_LOCAL_MODEL_INSTALLER: OnceLock<Arc<LocalModelInstaller>> = OnceLock::new();

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum LocalModelInstallState {
    NotInstalled,
    Downloading,
    Verifying,
    Installed,
    Failed,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum LocalModelInstallErrorKind {
    InvalidCatalog,
    UnknownModel,
    Busy,
    Network,
    Http,
    Io,
    SizeMismatch,
    Sha256Mismatch,
    Cancelled,
    Promotion,
    CorruptInstall,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct LocalModelInstallError {
    pub kind: LocalModelInstallErrorKind,
    pub message: String,
    pub retryable: bool,
}

impl LocalModelInstallError {
    fn new(kind: LocalModelInstallErrorKind, message: impl Into<String>, retryable: bool) -> Self {
        Self {
            kind,
            message: message.into(),
            retryable,
        }
    }

    fn invalid_catalog() -> Self {
        Self::new(
            LocalModelInstallErrorKind::InvalidCatalog,
            "The bundled local model catalog is invalid.",
            false,
        )
    }

    fn unknown_model() -> Self {
        Self::new(
            LocalModelInstallErrorKind::UnknownModel,
            "The selected local model is not in the supported catalog.",
            false,
        )
    }

    fn busy() -> Self {
        Self::new(
            LocalModelInstallErrorKind::Busy,
            "The selected local model already has an install or delete operation in progress.",
            true,
        )
    }

    fn network() -> Self {
        Self::new(
            LocalModelInstallErrorKind::Network,
            "The local model download failed because of a network error.",
            true,
        )
    }

    fn http(status: u16) -> Self {
        Self::new(
            LocalModelInstallErrorKind::Http,
            format!("The local model server returned HTTP status {status}."),
            status == 408 || status == 429 || status >= 500,
        )
    }

    fn io(operation: &'static str) -> Self {
        Self::new(
            LocalModelInstallErrorKind::Io,
            format!("The local model installer could not {operation}."),
            true,
        )
    }

    fn size_mismatch() -> Self {
        Self::new(
            LocalModelInstallErrorKind::SizeMismatch,
            "The downloaded local model has the wrong byte count.",
            true,
        )
    }

    fn sha256_mismatch() -> Self {
        Self::new(
            LocalModelInstallErrorKind::Sha256Mismatch,
            "The downloaded local model failed SHA-256 verification.",
            true,
        )
    }

    fn cancelled() -> Self {
        Self::new(
            LocalModelInstallErrorKind::Cancelled,
            "The local model download was cancelled.",
            true,
        )
    }

    fn promotion() -> Self {
        Self::new(
            LocalModelInstallErrorKind::Promotion,
            "The verified local model could not be promoted into the model directory.",
            true,
        )
    }

    fn corrupt_install() -> Self {
        Self::new(
            LocalModelInstallErrorKind::CorruptInstall,
            "The local model storage contains an unsafe or corrupt path.",
            false,
        )
    }
}

impl fmt::Display for LocalModelInstallError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(&self.message)
    }
}

impl std::error::Error for LocalModelInstallError {}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct LocalModelInstallProgress {
    pub model_id: String,
    pub install_state: LocalModelInstallState,
    pub downloaded_bytes: u64,
    pub total_bytes: u64,
}

pub type LocalModelInstallProgressCallback = Arc<dyn Fn(LocalModelInstallProgress) + Send + Sync>;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct LocalModelInstallOutcome {
    pub model_id: String,
    pub revision: String,
    pub installed_bytes: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LocalModelDescriptor {
    pub id: String,
    pub display_name: String,
    pub family: String,
    pub parameter_scale: String,
    pub quantization: String,
    pub revision: String,
    pub expected_bytes: u64,
    pub installed_bytes: Option<u64>,
    pub license: String,
    pub context_limit: u32,
    pub recommended_max_output: u32,
    pub install_state: LocalModelInstallState,
    pub active: bool,
    pub error: Option<LocalModelInstallError>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LocalModelDiagnostics {
    pub model_root_ready: bool,
    pub installs_in_progress: usize,
    pub last_error: Option<LocalModelInstallError>,
}

#[derive(Debug, Serialize, Deserialize)]
struct InstallMarker {
    schema_version: u32,
    model_id: String,
    revision: String,
    artifact_filename: String,
    expected_bytes: u64,
    sha256: String,
}

#[async_trait]
trait LocalModelDownloadTransport: Send + Sync {
    async fn download(
        &self,
        entry: &'static LocalModelCatalogEntry,
        destination: &Path,
        cancellation: &CancellationToken,
        progress: Option<&LocalModelInstallProgressCallback>,
    ) -> Result<(), LocalModelInstallError>;
}

struct ReqwestLocalModelDownloadTransport {
    client: reqwest::Client,
}

impl ReqwestLocalModelDownloadTransport {
    fn new() -> Result<Self, LocalModelInstallError> {
        let client = reqwest::Client::builder()
            .redirect(reqwest::redirect::Policy::limited(5))
            .build()
            .map_err(|_| LocalModelInstallError::network())?;
        Ok(Self { client })
    }
}

#[async_trait]
impl LocalModelDownloadTransport for ReqwestLocalModelDownloadTransport {
    async fn download(
        &self,
        entry: &'static LocalModelCatalogEntry,
        destination: &Path,
        cancellation: &CancellationToken,
        progress: Option<&LocalModelInstallProgressCallback>,
    ) -> Result<(), LocalModelInstallError> {
        let response = tokio::select! {
            _ = cancellation.cancelled() => return Err(LocalModelInstallError::cancelled()),
            response = self.client.get(entry.source_url).send() => {
                response.map_err(|_| LocalModelInstallError::network())?
            }
        };
        if !response.status().is_success() {
            return Err(LocalModelInstallError::http(response.status().as_u16()));
        }
        if let Some(length) = response.content_length() {
            if length != entry.expected_bytes {
                return Err(LocalModelInstallError::size_mismatch());
            }
        }

        let mut output = tokio::fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(destination)
            .await
            .map_err(|_| LocalModelInstallError::io("create the staging model file"))?;
        let mut downloaded = 0_u64;
        let mut stream = response.bytes_stream();
        loop {
            let next = tokio::select! {
                _ = cancellation.cancelled() => return Err(LocalModelInstallError::cancelled()),
                next = stream.next() => next,
            };
            let Some(chunk) = next else {
                break;
            };
            let chunk = chunk.map_err(|_| LocalModelInstallError::network())?;
            downloaded = downloaded
                .checked_add(chunk.len() as u64)
                .ok_or_else(LocalModelInstallError::size_mismatch)?;
            if downloaded > entry.expected_bytes {
                return Err(LocalModelInstallError::size_mismatch());
            }
            output
                .write_all(&chunk)
                .await
                .map_err(|_| LocalModelInstallError::io("write the staging model file"))?;
            if let Some(callback) = progress {
                callback(LocalModelInstallProgress {
                    model_id: entry.id.to_string(),
                    install_state: LocalModelInstallState::Downloading,
                    downloaded_bytes: downloaded,
                    total_bytes: entry.expected_bytes,
                });
            }
        }
        output
            .flush()
            .await
            .map_err(|_| LocalModelInstallError::io("flush the staging model file"))?;
        output
            .sync_all()
            .await
            .map_err(|_| LocalModelInstallError::io("sync the staging model file"))?;
        Ok(())
    }
}

pub struct LocalModelInstaller {
    root: PathBuf,
    transport: Arc<dyn LocalModelDownloadTransport>,
    in_flight: Mutex<HashMap<String, CancellationToken>>,
    last_errors: Mutex<HashMap<String, LocalModelInstallError>>,
}

impl LocalModelInstaller {
    pub fn new(root: PathBuf) -> Result<Self, LocalModelInstallError> {
        validate_local_model_catalog().map_err(|_| LocalModelInstallError::invalid_catalog())?;
        prepare_model_root(&root)?;
        let staging = root.join(STAGING_DIR);
        ensure_plain_directory(&staging, "create the model staging directory")?;
        cleanup_stale_staging(&staging)?;
        Ok(Self {
            root,
            transport: Arc::new(ReqwestLocalModelDownloadTransport::new()?),
            in_flight: Mutex::new(HashMap::new()),
            last_errors: Mutex::new(HashMap::new()),
        })
    }

    #[cfg(test)]
    fn with_transport(
        root: PathBuf,
        transport: Arc<dyn LocalModelDownloadTransport>,
    ) -> Result<Self, LocalModelInstallError> {
        validate_local_model_catalog().map_err(|_| LocalModelInstallError::invalid_catalog())?;
        prepare_model_root(&root)?;
        ensure_plain_directory(
            &root.join(STAGING_DIR),
            "create the model staging directory",
        )?;
        Ok(Self {
            root,
            transport,
            in_flight: Mutex::new(HashMap::new()),
            last_errors: Mutex::new(HashMap::new()),
        })
    }

    pub fn root(&self) -> &Path {
        &self.root
    }

    pub fn model_path(&self, model_id: &str) -> Result<PathBuf, LocalModelInstallError> {
        validate_storage_layout(&self.root)?;
        let entry =
            local_model_entry(model_id).ok_or_else(LocalModelInstallError::unknown_model)?;
        Ok(self
            .root
            .join(entry.id)
            .join(entry.revision)
            .join(entry.artifact_filename))
    }

    pub fn descriptors(&self, selected_model_id: &str) -> Vec<LocalModelDescriptor> {
        super::catalog::LOCAL_MODEL_CATALOG
            .iter()
            .map(|entry| self.descriptor(entry, selected_model_id))
            .collect()
    }

    pub fn diagnostics(&self) -> LocalModelDiagnostics {
        let last_error = self.last_errors.lock().values().last().cloned();
        LocalModelDiagnostics {
            model_root_ready: validate_storage_layout(&self.root).is_ok(),
            installs_in_progress: self.in_flight.lock().len(),
            last_error,
        }
    }

    pub fn cancel(&self, model_id: &str) -> bool {
        let token = self.in_flight.lock().get(model_id).cloned();
        if let Some(token) = token {
            token.cancel();
            true
        } else {
            false
        }
    }

    pub async fn install(
        &self,
        model_id: &str,
        progress: Option<LocalModelInstallProgressCallback>,
    ) -> Result<LocalModelInstallOutcome, LocalModelInstallError> {
        validate_storage_layout(&self.root)?;
        let entry =
            local_model_entry(model_id).ok_or_else(LocalModelInstallError::unknown_model)?;
        if self.install_is_valid(entry) {
            return Ok(LocalModelInstallOutcome {
                model_id: entry.id.to_string(),
                revision: entry.revision.to_string(),
                installed_bytes: entry.expected_bytes,
            });
        }

        let cancellation = {
            let mut in_flight = self.in_flight.lock();
            if in_flight.contains_key(model_id) {
                return Err(LocalModelInstallError::busy());
            }
            let token = CancellationToken::new();
            in_flight.insert(model_id.to_string(), token.clone());
            token
        };

        let result = self
            .install_inner(entry, &cancellation, progress.as_ref())
            .await;
        self.in_flight.lock().remove(model_id);
        match &result {
            Ok(_) => {
                self.last_errors.lock().remove(model_id);
            }
            Err(error) => {
                self.last_errors
                    .lock()
                    .insert(model_id.to_string(), error.clone());
            }
        }
        result
    }

    pub fn delete(&self, model_id: &str) -> Result<(), LocalModelInstallError> {
        validate_storage_layout(&self.root)?;
        let entry =
            local_model_entry(model_id).ok_or_else(LocalModelInstallError::unknown_model)?;
        if self.in_flight.lock().contains_key(model_id) {
            return Err(LocalModelInstallError::busy());
        }
        let model_dir = self.root.join(entry.id);
        match fs::symlink_metadata(&model_dir) {
            Ok(metadata) if metadata.file_type().is_symlink() => {
                fs::remove_file(&model_dir)
                    .map_err(|_| LocalModelInstallError::io("remove the local model link"))?;
            }
            Ok(metadata) if metadata.is_dir() => {
                fs::remove_dir_all(&model_dir)
                    .map_err(|_| LocalModelInstallError::io("delete the local model"))?;
            }
            Ok(_) => return Err(LocalModelInstallError::corrupt_install()),
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
            Err(_) => {
                return Err(LocalModelInstallError::io(
                    "inspect the local model directory",
                ))
            }
        }
        self.last_errors.lock().remove(model_id);
        Ok(())
    }

    async fn install_inner(
        &self,
        entry: &'static LocalModelCatalogEntry,
        cancellation: &CancellationToken,
        progress: Option<&LocalModelInstallProgressCallback>,
    ) -> Result<LocalModelInstallOutcome, LocalModelInstallError> {
        validate_storage_layout(&self.root)?;
        let staging_path =
            self.root
                .join(STAGING_DIR)
                .join(format!("{}-{}.partial", entry.id, Uuid::new_v4()));
        let result = async {
            self.transport
                .download(entry, &staging_path, cancellation, progress)
                .await?;
            if cancellation.is_cancelled() {
                return Err(LocalModelInstallError::cancelled());
            }
            if let Some(callback) = progress {
                callback(LocalModelInstallProgress {
                    model_id: entry.id.to_string(),
                    install_state: LocalModelInstallState::Verifying,
                    downloaded_bytes: entry.expected_bytes,
                    total_bytes: entry.expected_bytes,
                });
            }
            verify_artifact(&staging_path, entry.expected_bytes, entry.sha256)?;
            validate_storage_layout(&self.root)?;
            promote_artifact(&self.root, entry, &staging_path)?;
            Ok(LocalModelInstallOutcome {
                model_id: entry.id.to_string(),
                revision: entry.revision.to_string(),
                installed_bytes: entry.expected_bytes,
            })
        }
        .await;
        if staging_path.exists() {
            let _ = fs::remove_file(&staging_path);
        }
        result
    }

    fn descriptor(
        &self,
        entry: &'static LocalModelCatalogEntry,
        selected_model_id: &str,
    ) -> LocalModelDescriptor {
        let error = self.last_errors.lock().get(entry.id).cloned();
        let in_flight = self.in_flight.lock().contains_key(entry.id);
        let installed = self.install_is_valid(entry);
        let install_state = if in_flight {
            LocalModelInstallState::Downloading
        } else if installed {
            LocalModelInstallState::Installed
        } else if error.is_some() {
            LocalModelInstallState::Failed
        } else {
            LocalModelInstallState::NotInstalled
        };
        let installed_bytes = installed.then_some(entry.expected_bytes);
        LocalModelDescriptor {
            id: entry.id.to_string(),
            display_name: entry.display_name.to_string(),
            family: entry.family.to_string(),
            parameter_scale: entry.parameter_scale.to_string(),
            quantization: entry.quantization.to_string(),
            revision: entry.revision.to_string(),
            expected_bytes: entry.expected_bytes,
            installed_bytes,
            license: entry.license.to_string(),
            context_limit: entry.context_limit,
            recommended_max_output: entry.recommended_max_output,
            install_state,
            active: entry.id == selected_model_id,
            error,
        }
    }

    fn install_is_valid(&self, entry: &'static LocalModelCatalogEntry) -> bool {
        if validate_storage_layout(&self.root).is_err() {
            return false;
        }
        let model_dir = self.root.join(entry.id);
        let revision_dir = model_dir.join(entry.revision);
        let artifact = revision_dir.join(entry.artifact_filename);
        let marker_path = revision_dir.join(INSTALL_MARKER);

        for directory in [&model_dir, &revision_dir] {
            let Ok(metadata) = fs::symlink_metadata(directory) else {
                return false;
            };
            if metadata.file_type().is_symlink() || !metadata.is_dir() {
                return false;
            }
        }

        let Ok(artifact_metadata) = fs::symlink_metadata(&artifact) else {
            return false;
        };
        if artifact_metadata.file_type().is_symlink()
            || !artifact_metadata.is_file()
            || artifact_metadata.len() != entry.expected_bytes
        {
            return false;
        }

        let Ok(marker_metadata) = fs::symlink_metadata(&marker_path) else {
            return false;
        };
        if marker_metadata.file_type().is_symlink() || !marker_metadata.is_file() {
            return false;
        }
        let Ok(marker_bytes) = fs::read(marker_path) else {
            return false;
        };
        let Ok(marker): Result<InstallMarker, _> = serde_json::from_slice(&marker_bytes) else {
            return false;
        };
        marker.schema_version == INSTALL_MARKER_VERSION
            && marker.model_id == entry.id
            && marker.revision == entry.revision
            && marker.artifact_filename == entry.artifact_filename
            && marker.expected_bytes == entry.expected_bytes
            && marker.sha256 == entry.sha256
    }
}

pub fn initialize_global_local_model_installer(
    root: PathBuf,
) -> Result<Arc<LocalModelInstaller>, LocalModelInstallError> {
    if let Some(existing) = GLOBAL_LOCAL_MODEL_INSTALLER.get() {
        if existing.root() == root {
            return Ok(existing.clone());
        }
        return Err(LocalModelInstallError::new(
            LocalModelInstallErrorKind::Io,
            "The local model root was already initialized to a different application directory.",
            false,
        ));
    }
    let installer = Arc::new(LocalModelInstaller::new(root)?);
    match GLOBAL_LOCAL_MODEL_INSTALLER.set(installer.clone()) {
        Ok(()) => Ok(installer),
        Err(_) => GLOBAL_LOCAL_MODEL_INSTALLER
            .get()
            .cloned()
            .ok_or_else(|| LocalModelInstallError::io("initialize the local model installer")),
    }
}

pub fn global_local_model_installer() -> Result<Arc<LocalModelInstaller>, LocalModelInstallError> {
    GLOBAL_LOCAL_MODEL_INSTALLER.get().cloned().ok_or_else(|| {
        LocalModelInstallError::new(
            LocalModelInstallErrorKind::Io,
            "The local model installer is not initialized.",
            false,
        )
    })
}

fn prepare_model_root(root: &Path) -> Result<(), LocalModelInstallError> {
    let parent = root.parent().ok_or_else(LocalModelInstallError::corrupt_install)?;
    let anchor = parent
        .parent()
        .ok_or_else(LocalModelInstallError::corrupt_install)?;
    fs::create_dir_all(anchor)
        .map_err(|_| LocalModelInstallError::io("create the local model parent directory"))?;
    ensure_plain_directory(parent, "create the local model parent directory")?;
    ensure_plain_directory(root, "create the model root")
}

fn validate_storage_layout(root: &Path) -> Result<(), LocalModelInstallError> {
    let parent = root.parent().ok_or_else(LocalModelInstallError::corrupt_install)?;
    let staging = root.join(STAGING_DIR);
    for directory in [parent, root, staging.as_path()] {
        let metadata = fs::symlink_metadata(directory)
            .map_err(|_| LocalModelInstallError::corrupt_install())?;
        if metadata.file_type().is_symlink() || !metadata.is_dir() {
            return Err(LocalModelInstallError::corrupt_install());
        }
    }
    Ok(())
}

fn ensure_plain_directory(
    path: &Path,
    create_operation: &'static str,
) -> Result<(), LocalModelInstallError> {
    match fs::symlink_metadata(path) {
        Ok(metadata) => {
            if metadata.file_type().is_symlink() || !metadata.is_dir() {
                return Err(LocalModelInstallError::corrupt_install());
            }
        }
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
            fs::create_dir(path).map_err(|_| LocalModelInstallError::io(create_operation))?;
            let metadata = fs::symlink_metadata(path)
                .map_err(|_| LocalModelInstallError::io(create_operation))?;
            if metadata.file_type().is_symlink() || !metadata.is_dir() {
                return Err(LocalModelInstallError::corrupt_install());
            }
        }
        Err(_) => return Err(LocalModelInstallError::io(create_operation)),
    }
    Ok(())
}

fn verify_artifact(
    path: &Path,
    expected_bytes: u64,
    expected_sha256: &str,
) -> Result<(), LocalModelInstallError> {
    let metadata = fs::symlink_metadata(path)
        .map_err(|_| LocalModelInstallError::io("inspect the downloaded local model"))?;
    if metadata.file_type().is_symlink() || !metadata.is_file() {
        return Err(LocalModelInstallError::corrupt_install());
    }
    let file = fs::File::open(path)
        .map_err(|_| LocalModelInstallError::io("open the downloaded local model"))?;
    if file
        .metadata()
        .map_err(|_| LocalModelInstallError::io("inspect the downloaded local model"))?
        .len()
        != expected_bytes
    {
        return Err(LocalModelInstallError::size_mismatch());
    }
    let mut reader = BufReader::with_capacity(VERIFY_BUFFER_BYTES, file);
    let mut context = Sha256Context::new(&SHA256);
    let mut buffer = vec![0_u8; VERIFY_BUFFER_BYTES];
    loop {
        let read = reader
            .read(&mut buffer)
            .map_err(|_| LocalModelInstallError::io("verify the downloaded local model"))?;
        if read == 0 {
            break;
        }
        context.update(&buffer[..read]);
    }
    let actual = context.finish();
    let actual_hex = actual
        .as_ref()
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect::<String>();
    if actual_hex != expected_sha256 {
        return Err(LocalModelInstallError::sha256_mismatch());
    }
    Ok(())
}

fn write_install_marker_file(
    marker_tmp: &Path,
    marker_path: &Path,
    marker_bytes: &[u8],
) -> Result<(), LocalModelInstallError> {
    let mut marker_file = fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(marker_tmp)
        .map_err(|_| LocalModelInstallError::io("create the local model install marker"))?;
    marker_file
        .write_all(marker_bytes)
        .map_err(|_| LocalModelInstallError::io("write the local model install marker"))?;
    marker_file
        .sync_all()
        .map_err(|_| LocalModelInstallError::io("sync the local model install marker"))?;
    drop(marker_file);

    match fs::symlink_metadata(marker_path) {
        Ok(metadata) if metadata.file_type().is_symlink() || !metadata.is_file() => {
            return Err(LocalModelInstallError::corrupt_install());
        }
        Ok(_) => {
            fs::remove_file(marker_path).map_err(|_| {
                LocalModelInstallError::io("replace the local model install marker")
            })?;
        }
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
        Err(_) => {
            return Err(LocalModelInstallError::io(
                "inspect the local model install marker",
            ))
        }
    }
    fs::rename(marker_tmp, marker_path).map_err(|_| LocalModelInstallError::promotion())
}

fn write_install_marker(
    revision_dir: &Path,
    entry: &'static LocalModelCatalogEntry,
) -> Result<(), LocalModelInstallError> {
    let marker = InstallMarker {
        schema_version: INSTALL_MARKER_VERSION,
        model_id: entry.id.to_string(),
        revision: entry.revision.to_string(),
        artifact_filename: entry.artifact_filename.to_string(),
        expected_bytes: entry.expected_bytes,
        sha256: entry.sha256.to_string(),
    };
    let marker_bytes = serde_json::to_vec_pretty(&marker)
        .map_err(|_| LocalModelInstallError::io("serialize the local model install marker"))?;
    let marker_tmp = revision_dir.join(format!("{INSTALL_MARKER}.{}.tmp", Uuid::new_v4()));
    let marker_path = revision_dir.join(INSTALL_MARKER);
    let result = write_install_marker_file(&marker_tmp, &marker_path, &marker_bytes);
    if result.is_err() {
        let _ = fs::remove_file(&marker_tmp);
    }
    result
}

fn promote_artifact(
    root: &Path,
    entry: &'static LocalModelCatalogEntry,
    staging_path: &Path,
) -> Result<(), LocalModelInstallError> {
    let model_dir = root.join(entry.id);
    ensure_plain_directory(&model_dir, "create the local model directory")?;
    let revision_dir = model_dir.join(entry.revision);
    ensure_plain_directory(&revision_dir, "create the local model revision directory")?;

    let final_path = revision_dir.join(entry.artifact_filename);
    match fs::symlink_metadata(&final_path) {
        Ok(metadata) if metadata.file_type().is_symlink() || !metadata.is_file() => {
            return Err(LocalModelInstallError::corrupt_install());
        }
        Ok(_) => {
            fs::remove_file(&final_path).map_err(|_| {
                LocalModelInstallError::io("replace the previous local model artifact")
            })?;
        }
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
        Err(_) => {
            return Err(LocalModelInstallError::io(
                "inspect the previous local model artifact",
            ))
        }
    }
    fs::rename(staging_path, &final_path).map_err(|_| LocalModelInstallError::promotion())?;

    if let Err(error) = write_install_marker(&revision_dir, entry) {
        let _ = fs::remove_file(&final_path);
        return Err(error);
    }
    Ok(())
}

fn cleanup_stale_staging(staging: &Path) -> Result<(), LocalModelInstallError> {
    let metadata = fs::symlink_metadata(staging)
        .map_err(|_| LocalModelInstallError::io("inspect the model staging directory"))?;
    if metadata.file_type().is_symlink() || !metadata.is_dir() {
        return Err(LocalModelInstallError::corrupt_install());
    }
    for entry in fs::read_dir(staging)
        .map_err(|_| LocalModelInstallError::io("inspect the model staging directory"))?
    {
        let entry = entry.map_err(|_| LocalModelInstallError::io("inspect a staging entry"))?;
        let path = entry.path();
        let metadata = fs::symlink_metadata(&path)
            .map_err(|_| LocalModelInstallError::io("inspect a stale staging entry"))?;
        if metadata.is_dir() && !metadata.file_type().is_symlink() {
            fs::remove_dir_all(path)
                .map_err(|_| LocalModelInstallError::io("remove a stale staging directory"))?;
        } else {
            fs::remove_file(path)
                .map_err(|_| LocalModelInstallError::io("remove a stale staging file"))?;
        }
    }
    Ok(())
}

#[cfg(test)]
mod adversarial_tests;

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicUsize, Ordering};
    use tempfile::tempdir;

    struct BytesTransport {
        bytes: Vec<u8>,
        calls: AtomicUsize,
    }

    #[async_trait]
    impl LocalModelDownloadTransport for BytesTransport {
        async fn download(
            &self,
            _entry: &'static LocalModelCatalogEntry,
            destination: &Path,
            cancellation: &CancellationToken,
            _progress: Option<&LocalModelInstallProgressCallback>,
        ) -> Result<(), LocalModelInstallError> {
            self.calls.fetch_add(1, Ordering::SeqCst);
            if cancellation.is_cancelled() {
                return Err(LocalModelInstallError::cancelled());
            }
            tokio::fs::write(destination, &self.bytes)
                .await
                .map_err(|_| LocalModelInstallError::io("write test transport output"))
        }
    }

    #[test]
    fn verifier_rejects_wrong_size_and_wrong_hash() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("artifact.gguf");
        fs::write(&path, b"abc").unwrap();
        let wrong_size = verify_artifact(&path, 4, "00").unwrap_err();
        assert_eq!(wrong_size.kind, LocalModelInstallErrorKind::SizeMismatch);

        let wrong_hash = verify_artifact(
            &path,
            3,
            "0000000000000000000000000000000000000000000000000000000000000000",
        )
        .unwrap_err();
        assert_eq!(wrong_hash.kind, LocalModelInstallErrorKind::Sha256Mismatch);
    }

    #[test]
    fn stale_staging_files_and_directories_are_removed_on_startup() {
        let dir = tempdir().unwrap();
        let staging = dir.path().join(STAGING_DIR);
        fs::create_dir_all(staging.join("old-dir")).unwrap();
        fs::write(staging.join("old.partial"), b"stale").unwrap();
        let _installer = LocalModelInstaller::new(dir.path().to_path_buf()).unwrap();
        assert_eq!(fs::read_dir(staging).unwrap().count(), 0);
    }

    #[test]
    fn model_paths_are_catalog_owned_and_revision_scoped() {
        let dir = tempdir().unwrap();
        let installer = LocalModelInstaller::new(dir.path().to_path_buf()).unwrap();
        let path = installer
            .model_path(super::super::catalog::DEFAULT_LOCAL_TEXT_MODEL_ID)
            .unwrap();
        let entry = local_model_entry(super::super::catalog::DEFAULT_LOCAL_TEXT_MODEL_ID).unwrap();
        assert!(path.ends_with(
            Path::new(entry.id)
                .join(entry.revision)
                .join(entry.artifact_filename)
        ));
        assert_eq!(
            installer.model_path("../escape").unwrap_err().kind,
            LocalModelInstallErrorKind::UnknownModel
        );
    }

    #[test]
    fn delete_never_follows_a_model_directory_symlink() {
        #[cfg(unix)]
        {
            use std::os::unix::fs::symlink;
            let dir = tempdir().unwrap();
            let outside = tempdir().unwrap();
            let sentinel = outside.path().join("keep.txt");
            fs::write(&sentinel, b"keep").unwrap();
            let installer = LocalModelInstaller::new(dir.path().to_path_buf()).unwrap();
            let entry =
                local_model_entry(super::super::catalog::DEFAULT_LOCAL_TEXT_MODEL_ID).unwrap();
            symlink(outside.path(), dir.path().join(entry.id)).unwrap();
            installer.delete(entry.id).unwrap();
            assert!(sentinel.exists());
            assert!(!dir.path().join(entry.id).exists());
        }
    }

    #[tokio::test]
    async fn duplicate_install_is_rejected_without_silent_parallel_work() {
        let dir = tempdir().unwrap();
        let transport = Arc::new(BytesTransport {
            bytes: vec![],
            calls: AtomicUsize::new(0),
        });
        let installer =
            LocalModelInstaller::with_transport(dir.path().to_path_buf(), transport).unwrap();
        let model_id = super::super::catalog::DEFAULT_LOCAL_TEXT_MODEL_ID;
        installer
            .in_flight
            .lock()
            .insert(model_id.to_string(), CancellationToken::new());
        let error = installer.install(model_id, None).await.unwrap_err();
        assert_eq!(error.kind, LocalModelInstallErrorKind::Busy);
    }

    #[tokio::test]
    async fn cancellation_handle_targets_only_the_requested_model() {
        let dir = tempdir().unwrap();
        let transport = Arc::new(BytesTransport {
            bytes: vec![1, 2, 3],
            calls: AtomicUsize::new(0),
        });
        let installer =
            LocalModelInstaller::with_transport(dir.path().to_path_buf(), transport).unwrap();
        let model_id = super::super::catalog::DEFAULT_LOCAL_TEXT_MODEL_ID;
        let token = CancellationToken::new();
        installer
            .in_flight
            .lock()
            .insert(model_id.to_string(), token.clone());
        assert!(installer.cancel(model_id));
        assert!(token.is_cancelled());
        assert!(!installer.cancel("qwen3-0-6b-instruct-q4-k-m"));
    }
}
