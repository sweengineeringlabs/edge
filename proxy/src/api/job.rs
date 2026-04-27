//! Job trait — **Job** concern of the 5-Concern Controller pattern.
//!
//! The single entry point the gateway calls. Each Controller implementation
//! provides one `Job` impl that orchestrates its full request→response flow.

use async_trait::async_trait;

use super::error::JobError;

/// The single entry point for Controller work.
///
/// Gateway holds `Arc<dyn Job<Req, Resp>>` and calls `run` for each request.
///
/// | Domain | Req | Resp |
/// |--------|-----|------|
/// | llmboot | `String` (user prompt) | `AgentOutput` |
/// | vmisolate | `VmConfig` | `VmStatus` |
/// | security/iam | `SecurityRequest` | `SecurityOutcome` |
#[async_trait]
pub trait Job<Req, Resp>: Send + Sync
where
    Req: Send + 'static,
    Resp: Send + 'static,
{
    /// Execute the job with the given request and return the response.
    async fn run(&self, req: Req) -> Result<Resp, JobError>;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_job_trait_is_object_safe() {
        fn _accept(_j: &dyn Job<String, String>) {}
    }
}
