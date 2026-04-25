//! SAF layer — outbound public facade.

mod builder;

pub use crate::api::database::{DatabaseGateway, DatabaseRead, DatabaseWrite};
pub use crate::api::error::EgressError;
pub use crate::api::http::HttpClient;
pub use crate::api::notification::NotificationSender;
pub use crate::api::output::OutboundSink;
pub use crate::api::payment::PaymentGateway;
pub use builder::{memory_database, Builder};
