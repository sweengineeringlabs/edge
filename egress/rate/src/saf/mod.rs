//! SAF layer — public facade.

mod builder;

pub use crate::api::rate_config::RateConfig;
pub use crate::api::rate_layer::RateLayer;
pub use crate::api::error::Error;
pub use builder::{builder, Builder};
