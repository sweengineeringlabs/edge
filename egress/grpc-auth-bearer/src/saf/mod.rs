//! SAF layer — public facade.

pub use crate::api::{
    BearerAuthError, BearerInboundConfig, BearerOutboundConfig, BearerSecret,
    AUTHORIZATION_HEADER, EXTRACTED_BEARER_SUBJECT,
};
pub use crate::core::{BearerInboundInterceptor, BearerOutboundInterceptor};
