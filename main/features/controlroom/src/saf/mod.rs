//! SAF facade layer (L4) - public surface.
//!
//! Re-export types from api/ and expose core functionality
//! via standalone public functions.


mod facade;

// Re-export public types
pub use crate::api::config::Config;
pub use crate::api::error::Error;

// Re-export facade functions
pub use facade::{execute, run};

