//! SAF layer — inbound public facade.

mod builder;

pub use crate::api::error::IngressError;
pub use crate::api::input::InboundSource;
pub use crate::api::http::HttpInbound;
pub use builder::{file_input, Builder};
