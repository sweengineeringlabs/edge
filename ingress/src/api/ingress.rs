//! Ingress trait definition.
//!
//! Implement this trait in core/ to define ingress's primary behavior.

use super::config::Config;
use super::error::Error;

/// Primary service trait for ingress.
pub trait Ingress: Send + Sync {
    /// Execute the primary operation with the given configuration.
    fn execute(&self, config: &Config) -> Result<(), Error>;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_ingress_trait_is_object_safe() {
        fn _accept(_s: &dyn Ingress) {}
    }
}
