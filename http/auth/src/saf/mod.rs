//! SAF layer — public facade.

mod builder;

pub use crate::api::auth_config::AuthConfig;
pub use crate::api::error::Error;
pub use builder::{builder, Builder};
