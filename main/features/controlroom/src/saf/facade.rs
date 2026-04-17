//! Facade functions wrapping Service trait.

use crate::api::config::Config;
use crate::api::error::Error;
use crate::api::traits::Service;

/// Execute the primary operation with the given configuration.
///
/// Wraps the `Service` trait as a standalone function.
pub fn execute(config: &Config) -> Result<(), Error> {
    let service = crate::core::service::DefaultService::new();
    service.execute(config)
}

/// Execute the primary operation with default configuration.
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
