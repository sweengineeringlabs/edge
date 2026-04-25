//! Stdout output sink.

use futures::future::BoxFuture;

use crate::api::output::OutputSink;
use crate::api::types::GatewayResult;

/// Writes output data to stdout.
pub(crate) struct StdoutSink;

impl OutputSink for StdoutSink {
    fn write(&self, data: &[u8]) -> BoxFuture<'_, GatewayResult<()>> {
        let output = String::from_utf8_lossy(data).into_owned();
        Box::pin(async move {
            print!("{}", output);
            Ok(())
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_stdout_sink_writes_without_error() {
        let sink = StdoutSink;
        let result = sink.write(b"hello stdout\n").await;
        assert!(result.is_ok());
    }
}
