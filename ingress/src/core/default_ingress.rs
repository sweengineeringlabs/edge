//! Default Ingress implementation.

use crate::api::config::Config;
use crate::api::error::Error;
use crate::api::ingress::Ingress;

/// Default implementation of the Ingress trait.
#[derive(Debug, Default)]
pub(crate) struct DefaultIngress;

impl DefaultIngress {
    /// Create a new default instance.
    pub(crate) fn new() -> Self {
        Self
    }
}

impl Ingress for DefaultIngress {
    fn execute(&self, config: &Config) -> Result<(), Error> {
        if config.verbose {
            println!("[ingress] executing with verbose=true");
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_new_creates_default_ingress() {
        let _svc = DefaultIngress::new();
    }

    #[test]
    fn test_execute_succeeds_with_default_config() {
        let svc = DefaultIngress::new();
        let config = Config::default();
        assert!(svc.execute(&config).is_ok());
    }

    #[test]
    fn test_execute_succeeds_in_verbose_mode() {
        let svc = DefaultIngress::new();
        let config = Config::default().with_verbose(true);
        assert!(svc.execute(&config).is_ok());
    }
}
