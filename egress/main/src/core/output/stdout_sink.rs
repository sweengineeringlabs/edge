//! Stdout output sink — writes to standard output.

use futures::future::BoxFuture;

use crate::api::egress_error::EgressResult;
use crate::api::outbound_sink::OutputSink;

/// Writes data to stdout.
pub(crate) struct StdoutSink;

impl OutputSink for StdoutSink {
    fn write(&self, data: Vec<u8>) -> BoxFuture<'_, EgressResult<()>> {
        Box::pin(async move {
            let text = String::from_utf8_lossy(&data);
            print!("{text}");
            Ok(())
        })
    }

    fn flush(&self) -> BoxFuture<'_, EgressResult<()>> {
        Box::pin(async move {
            use std::io::Write;
            std::io::stdout().flush().map_err(crate::api::egress_error::EgressError::IoError)
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_stdout_sink_write_succeeds() {
        let s = StdoutSink;
        assert!(s.write(b"test\n".to_vec()).await.is_ok());
    }

    #[tokio::test]
    async fn test_stdout_sink_flush_succeeds() {
        assert!(StdoutSink.flush().await.is_ok());
    }
}
