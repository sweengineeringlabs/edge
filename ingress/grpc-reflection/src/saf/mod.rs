//! SAF layer — public facade.

pub use crate::api::error::ReflectionError;
pub use crate::api::types::{Descriptor, ReflectionRequest, ReflectionResponse};
pub use crate::core::reflection_service::{
    service_name_from_method_path, ReflectionService, ERROR_CODE_INVALID_ARGUMENT,
    ERROR_CODE_NOT_FOUND, ERROR_CODE_UNIMPLEMENTED, REFLECTION_INFO_METHOD,
    REFLECTION_SERVICE_NAME,
};
pub use crate::core::wire::{decode_request, encode_response};
