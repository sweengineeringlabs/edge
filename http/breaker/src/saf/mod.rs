//! SAF layer — public facade.

mod builder;

pub use crate::api::breaker_config::BreakerConfig;
pub use crate::api::error::Error;
pub use builder::{builder, Builder};
