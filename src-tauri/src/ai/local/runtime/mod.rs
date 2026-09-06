//! Isolated local llama.cpp runtime.
//!
//! Binding-specific model/context/sampler types are confined below this module boundary.
//! Application state and commands interact only with the neutral runtime manager/types.

mod chat_template;
#[cfg(test)]
mod diagnostics_tests;
mod llama;
mod manager;
mod reasoning;
#[cfg(test)]
mod residual_tests;
#[cfg(test)]
mod tests;
pub(crate) mod types;

pub(crate) use manager::LocalRuntimeManager;
