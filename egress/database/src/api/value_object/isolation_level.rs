//! Transaction isolation levels.

use serde::{Deserialize, Serialize};

/// SQL transaction isolation level.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum IsolationLevel {
    ReadUncommitted,
    #[default]
    ReadCommitted,
    RepeatableRead,
    Serializable,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_isolation_level_default_is_read_committed() {
        assert_eq!(IsolationLevel::default(), IsolationLevel::ReadCommitted);
    }

    #[test]
    fn test_isolation_levels_are_distinct() {
        assert_ne!(IsolationLevel::ReadUncommitted, IsolationLevel::Serializable);
    }
}
