//! Public type — the breaker decorator.
//!
//! Per SEA rule 160 the type *declaration* is here; the
//! `GrpcOutbound` impl block lives in `core::breaker_client`.

use std::sync::Arc;

use tokio::sync::Mutex;

use crate::api::breaker_config::GrpcBreakerConfig;
use crate::api::breaker_state::BreakerState;

/// Decorator that wraps an inner [`GrpcOutbound`] with a
/// three-state circuit breaker.
///
/// Construct via [`builder()`](crate::builder) (loads SWE
/// baseline) or [`GrpcBreakerClient::new`] (caller-supplied
/// config).
pub struct GrpcBreakerClient<T> {
    pub(crate) inner:  T,
    pub(crate) config: Arc<GrpcBreakerConfig>,
    pub(crate) node:   Arc<Mutex<BreakerNode>>,
}

/// Internal state container.  Crate-private; consumers observe
/// state via [`GrpcBreakerClient::state`].
#[derive(Debug)]
pub(crate) struct BreakerNode {
    pub(crate) state:                 BreakerState,
    pub(crate) consecutive_failures:  u32,
    pub(crate) consecutive_successes: u32,
}

impl BreakerNode {
    pub(crate) fn new() -> Self {
        Self {
            state:                 BreakerState::Closed,
            consecutive_failures:  0,
            consecutive_successes: 0,
        }
    }
}

impl<T> std::fmt::Debug for GrpcBreakerClient<T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("GrpcBreakerClient")
            .field("failure_threshold",     &self.config.failure_threshold)
            .field("cool_down_seconds",     &self.config.cool_down_seconds)
            .field("half_open_probe_count", &self.config.half_open_probe_count)
            .finish()
    }
}

impl<T> GrpcBreakerClient<T> {
    /// Construct a new breaker decorator around `inner`.
    pub fn new(inner: T, config: GrpcBreakerConfig) -> Self {
        Self {
            inner,
            config: Arc::new(config),
            node:   Arc::new(Mutex::new(BreakerNode::new())),
        }
    }

    /// Borrow the active breaker policy.
    pub fn config(&self) -> &GrpcBreakerConfig {
        &self.config
    }

    /// Observe the current breaker state.  Returns a snapshot;
    /// the breaker may transition immediately after this call.
    pub async fn state(&self) -> BreakerState {
        self.node.lock().await.state
    }
}
