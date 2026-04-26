//! File inbound port.
pub(crate) mod file_inbound;

#[allow(unused_imports)]
pub use file_inbound::{FileInbound, FileInboundError, FileInboundResult, FileHealthCheck};
