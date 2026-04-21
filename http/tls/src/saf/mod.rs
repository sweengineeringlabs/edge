//! SAF layer — public facade.

mod builder;

pub use crate::api::error::Error;
pub use crate::api::tls_config::TlsConfig;
pub use crate::api::tls_layer::TlsLayer;
pub use builder::{builder, Builder};
