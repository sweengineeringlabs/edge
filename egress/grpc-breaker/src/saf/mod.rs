//! SAF layer — public facade.

mod builder;

pub use crate::api::breaker_client::GrpcBreakerClient;
pub use crate::api::breaker_config::GrpcBreakerConfig;
pub use crate::api::breaker_state::BreakerState;
pub use crate::api::error::Error;
pub use builder::{create_breaker_client, builder, Builder};
