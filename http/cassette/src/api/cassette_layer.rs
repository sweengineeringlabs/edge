//! Public type — the VCR-style cassette middleware.

use std::path::PathBuf;
use std::sync::Arc;

use crate::api::cassette_config::CassetteConfig;

/// Cassette middleware. Attach to a
/// `reqwest_middleware::ClientBuilder` via `.with(layer)`.
///
/// Modes:
/// - `"replay"`: read-only — replay fixtures; fail on cache miss
/// - `"record"`: always hit upstream; overwrite fixture on every
///   request (including subsequent to re-record stale data)
/// - `"auto"`: replay on hit; record on miss (local dev default)
pub struct CassetteLayer {
    pub(crate) config: Arc<CassetteConfig>,
    pub(crate) cassette_path: PathBuf,
    pub(crate) fixtures:
        Arc<tokio::sync::Mutex<std::collections::HashMap<String, crate::core::recorded_interaction::RecordedInteraction>>>,
}

impl std::fmt::Debug for CassetteLayer {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("CassetteLayer")
            .field("mode", &self.config.mode)
            .field("cassette_path", &self.cassette_path)
            .finish()
    }
}
