//! SAF layer — inbound public facade.

mod builder;

pub use crate::api::ingress_error::IngressError;
pub use crate::api::inbound_source::InboundSource;
pub use crate::api::http_inbound::HttpInbound;
pub use builder::{file_input, passthrough_validator};
pub use crate::api::builder::build_file_input;
pub use crate::api::builder::Builder;
