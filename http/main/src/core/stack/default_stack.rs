//! Default implementation of the middleware stack assembler.

use crate::api::stack_config::StackConfig;
use crate::api::traits::StackAssembler;

/// Assembles the full HTTP middleware stack from a resolved config.
pub(crate) struct DefaultStack {
    #[allow(dead_code)]
    config: StackConfig,
}

impl DefaultStack {
    /// Construct from a resolved config.
    pub(crate) fn new(config: StackConfig) -> Self {
        Self { config }
    }
}

impl StackAssembler for DefaultStack {}

#[cfg(test)]
mod tests {
    use super::*;
    use swe_http_auth::AuthConfig;
    use crate::api::stack_config::StackConfig;

    #[test]
    fn test_new_constructs_default_stack() {
        let cfg = StackConfig { auth: AuthConfig::None };
        let _stack = DefaultStack::new(cfg);
    }
}
