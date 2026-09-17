//! Compatibility path for the pre-remediation Wake Word engine module.
//!
//! New production code must use `crate::wake_word::engine`.  The duplicate app
//! engine abstraction was removed so there is one KWS engine/session policy.

pub use crate::wake_word::engine::*;
