//! Error type for the main composition crate.

/// Errors raised during stack assembly.
#[derive(Debug, thiserror::Error)]
pub enum Error {
    /// A required middleware crate failed to build.
    #[error("swe_http_main: middleware build failed — {0}")]
    BuildFailed(String),

    /// Not yet implemented.
    #[error("swe_http_main: not implemented — {0}")]
    NotImplemented(&'static str),
}
