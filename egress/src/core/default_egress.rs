//! Default Egress implementation.

use crate::api::config::Config;
use crate::api::error::Error;
use crate::api::egress::Egress;

/// Default implementation of the Egress trait.
#[derive(Debug, Default)]
pub(crate) struct DefaultEgress;

impl DefaultEgress {
    /// Create a new default instance.
    pub(crate) fn new() -> Self {
        Self
    }
}

impl Egress for DefaultEgress {
    fn execute(&self, config: &Config) -> Result<(), Error> {
        if config.verbose {
            println!("[egress] executing with verbose=true");
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_new_creates_default_egress() {
        let _svc = DefaultEgress::new();
    }

    #[test]
    fn test_execute_succeeds_with_default_config() {
        let svc = DefaultEgress::new();
        let config = Config::default();
        assert!(svc.execute(&config).is_ok());
    }

    #[test]
    fn test_execute_succeeds_in_verbose_mode() {
        let svc = DefaultEgress::new();
        let config = Config::default().with_verbose(true);
        assert!(svc.execute(&config).is_ok());
    }
}
