//! SAF layer — domain public facade.

mod factory;

pub use factory::new_handler_registry;

pub use crate::api::error::HandlerError;
pub use crate::api::handler::Handler;
pub use crate::api::handler_registry::HandlerRegistry;
