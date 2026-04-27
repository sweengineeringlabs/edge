//! RuntimeStatus — lifecycle state of the runtime manager.

use serde::{Deserialize, Serialize};
use std::fmt;

/// Lifecycle state of the runtime manager.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RuntimeStatus {
    /// Runtime is initialising — not yet ready to serve.
    Starting,
    /// Runtime is fully started and serving traffic.
    Running,
    /// Runtime is draining in-flight requests before stopping.
    Stopping,
    /// Runtime has fully stopped.
    Stopped,
    /// Runtime is running but one or more components are degraded.
    Degraded,
}

impl fmt::Display for RuntimeStatus {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Starting  => write!(f, "starting"),
            Self::Running   => write!(f, "running"),
            Self::Stopping  => write!(f, "stopping"),
            Self::Stopped   => write!(f, "stopped"),
            Self::Degraded  => write!(f, "degraded"),
        }
    }
}

impl RuntimeStatus {
    /// Returns `true` only when the status is [`Running`](Self::Running).
    pub fn is_healthy(&self) -> bool {
        matches!(self, Self::Running)
    }

    /// Returns `true` only when the status is [`Stopped`](Self::Stopped).
    pub fn is_terminal(&self) -> bool {
        matches!(self, Self::Stopped)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_display_returns_lowercase_name() {
        assert_eq!(RuntimeStatus::Running.to_string(),  "running");
        assert_eq!(RuntimeStatus::Stopping.to_string(), "stopping");
        assert_eq!(RuntimeStatus::Stopped.to_string(),  "stopped");
        assert_eq!(RuntimeStatus::Degraded.to_string(), "degraded");
    }

    #[test]
    fn test_is_healthy_only_true_for_running() {
        assert!(RuntimeStatus::Running.is_healthy());
        assert!(!RuntimeStatus::Starting.is_healthy());
        assert!(!RuntimeStatus::Degraded.is_healthy());
    }

    #[test]
    fn test_is_terminal_only_true_for_stopped() {
        assert!(RuntimeStatus::Stopped.is_terminal());
        assert!(!RuntimeStatus::Running.is_terminal());
    }
}
