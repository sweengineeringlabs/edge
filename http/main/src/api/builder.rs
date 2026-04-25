//! Builder type for the middleware stack (declared in api/, impl in saf/).

use crate::api::stack_config::StackConfig;

/// Builds a configured HTTP middleware stack.
#[derive(Debug)]
pub struct Builder {
    pub(crate) config: StackConfig,
}
