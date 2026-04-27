//! 5-Concern Proxy pattern marker — surfaced in rustdoc for discoverability.

/// Marker type naming the five concerns for discoverability in docs.
///
/// Traits implementing each concern:
/// 1. **Job** — [`crate::Job`]
/// 2. **Routing** — [`crate::Router`]
/// 3. **Handlers** — [`crate::Handler`]
/// 4. **Lifecycle** — [`crate::LifecycleMonitor`]
/// 5. **Gateway (boundary types)** — `crate::gateway` module (internal)
#[allow(dead_code)]
pub struct ProxyPattern;
