//! Job trait — **Job** concern of the 5-Concern Controller pattern.
//!
//! The single entry point the gateway calls. Each Controller implementation
//! provides one `Job` impl that orchestrates its full request→response flow.

use async_trait::async_trait;

use super::error::JobError;

/// The single entry point for proxy dispatch.
///
/// Runtime holds `Arc<dyn Job<Req, Response>>` and calls `run` for each request.
#[async_trait]
pub trait Job<Req, Response>: Send + Sync
where
    Req: Send + 'static,
    Response: Send + 'static,
{
    /// Dispatch the request and return the response.
    async fn run(&self, req: Req) -> Result<Response, JobError>;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_job_trait_is_object_safe() {
        fn _accept(_j: &dyn Job<String, String>) {} // object-safe with concrete types
    }
}
