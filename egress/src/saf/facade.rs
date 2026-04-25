//! Facade functions wrapping Egress.

use crate::api::config::Config;
use crate::api::error::Error;
use crate::api::egress::Egress;

/// Execute the primary operation with the given configuration.
pub fn execute(config: &Config) -> Result<(), Error> {
    let svc = crate::core::DefaultEgress::new();
    svc.execute(config)
}

/// Execute with default configuration.
pub fn run() -> Result<(), Error> {
    execute(&Config::default())
}

#[cfg(test)]
mod tests {
    use super::*;

    /// @covers: execute
    #[test]
    fn test_execute_with_default_config() {
        assert!(execute(&Config::default()).is_ok());
    }

    /// @covers: run
    #[test]
    fn test_run_succeeds() {
        assert!(run().is_ok());
    }
}
