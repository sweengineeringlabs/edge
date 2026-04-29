//! SAF layer — public facade.
//!
//! Single source of public re-exports.  api/ and core/ stay
//! crate-private; consumers only see what we surface here.

mod builder;

pub use crate::api::backoff::BackoffSchedule;
pub use crate::api::error::Error;
pub use crate::api::retry_client::GrpcRetryClient;
pub use crate::api::retry_config::GrpcRetryConfig;
pub use crate::api::retry_policy::{classify, RetryDecision};
pub use builder::{create_retry_client, builder, Builder};
