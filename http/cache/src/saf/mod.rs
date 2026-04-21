//! SAF layer — public facade.

mod builder;

pub use crate::api::cache_config::CacheConfig;
pub use crate::api::error::Error;
pub use builder::{builder, Builder};
