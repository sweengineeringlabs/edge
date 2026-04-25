//! SAF layer — HTTP public facade.

pub use crate::api::port::{HttpOutbound, HttpOutboundError, HttpOutboundResult};
pub use crate::api::value_object::{FormPart, HttpAuth, HttpBody, HttpConfig, HttpMethod, HttpRequest, HttpResponse};
