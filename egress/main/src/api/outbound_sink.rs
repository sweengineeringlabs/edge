//! OutputSink trait — writes data to a destination.

use futures::future::BoxFuture;

use crate::api::egress_error::EgressResult;

/// Writes outbound data to a sink (stdout, file, network, etc.).
pub trait OutputSink: Send + Sync {
    fn write(&self, data: Vec<u8>) -> BoxFuture<'_, EgressResult<()>>;
    fn flush(&self) -> BoxFuture<'_, EgressResult<()>>;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_output_sink_is_object_safe() {
        fn _assert_object_safe(_: &dyn OutputSink) {}
    }
}
