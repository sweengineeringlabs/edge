//! SAF layer — public facade.

mod builder;

pub use crate::api::config::BreakerConfig;
pub use crate::api::error::Error;
pub use builder::{builder, Builder};
