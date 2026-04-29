//! Public type — the retry decorator that wraps any
//! [`GrpcOutbound`].
//!
//! Per SEA rule 160, the type *declaration* lives in api/.
//! The `GrpcOutbound` impl block lives in `core::retry_client`.

use std::sync::Arc;

use crate::api::retry_config::GrpcRetryConfig;

/// Decorator that wraps an inner [`GrpcOutbound`] with the
/// retry semantics described at the crate root.
///
/// `T` is the inner type; the wrapper is `T + 'static + Send + Sync`
/// so it can flow across `.await` boundaries inside the runtime.
///
/// Construct via the [`builder()`](crate::builder) entry point
/// (loads the SWE baseline) or directly via
/// [`GrpcRetryClient::new`].
pub struct GrpcRetryClient<T> {
    pub(crate) inner:  T,
    pub(crate) config: Arc<GrpcRetryConfig>,
}

impl<T> std::fmt::Debug for GrpcRetryClient<T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("GrpcRetryClient")
            .field("max_attempts",       &self.config.max_attempts)
            .field("initial_backoff_ms", &self.config.initial_backoff_ms)
            .field("backoff_multiplier", &self.config.backoff_multiplier)
            .field("jitter_factor",      &self.config.jitter_factor)
            .field("max_backoff_ms",     &self.config.max_backoff_ms)
            .finish()
    }
}

impl<T> GrpcRetryClient<T> {
    /// Construct a new retry decorator around `inner`.
    pub fn new(inner: T, config: GrpcRetryConfig) -> Self {
        Self { inner, config: Arc::new(config) }
    }

    /// Borrow the active retry policy.
    pub fn config(&self) -> &GrpcRetryConfig {
        &self.config
    }
}
