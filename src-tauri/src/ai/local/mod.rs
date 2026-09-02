#[cfg(feature = "local-llm-acceptance")]
pub mod acceptance;
pub mod catalog;
#[cfg(test)]
mod compile_proof;
pub mod installer;
pub(crate) mod runtime;
mod text_model;

pub use catalog::{
    local_model_entry, validate_local_model_catalog, LocalModelCatalogEntry,
    LocalModelTemplateHint, DEFAULT_LOCAL_TEXT_MODEL_ID, LOCAL_MODEL_CATALOG,
};
pub use installer::{
    global_local_model_installer, initialize_global_local_model_installer, LocalModelDescriptor,
    LocalModelDiagnostics, LocalModelInstallError, LocalModelInstallErrorKind,
    LocalModelInstallOutcome, LocalModelInstallProgress, LocalModelInstallProgressCallback,
    LocalModelInstallState, LocalModelInstaller,
};
pub(crate) use runtime::LocalRuntimeManager;
pub use text_model::LocalTextModel;
