//! Compatibility path for the pre-remediation Wake Word runtime module.
//!
//! New production code must use `crate::wake_word::runtime`.  This module owns
//! no runtime state or implementation; it only preserves source compatibility
//! while callers migrate to the authoritative Wake Word subsystem boundary.

pub use crate::wake_word::runtime::*;
