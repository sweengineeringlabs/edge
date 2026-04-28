//! Inbound interceptor trait + chain machinery.
//!
//! Interceptors observe and mutate the request **before** the handler
//! is dispatched, and observe and mutate the response **after** it
//! returns.  They run in registration order; the **first failure
//! short-circuits** — the server returns a gRPC `Status` to the client
//! without invoking the handler.

use std::sync::Arc;

use crate::api::port::grpc_inbound::GrpcInboundError;
use crate::api::value_object::{GrpcRequest, GrpcResponse};

/// An interceptor for inbound gRPC calls.
pub trait GrpcInboundInterceptor: Send + Sync {
    /// Run before the handler is dispatched.  Returning `Err(_)`
    /// aborts the call — the handler is **not** invoked.
    fn before_dispatch(&self, req: &mut GrpcRequest) -> Result<(), GrpcInboundError>;

    /// Run after the handler returns successfully.  Returning
    /// `Err(_)` converts the call result into that error.
    fn after_dispatch(&self, resp: &mut GrpcResponse) -> Result<(), GrpcInboundError>;
}

/// A registered chain of [`GrpcInboundInterceptor`]s.
#[derive(Clone, Default)]
pub struct GrpcInboundInterceptorChain {
    interceptors: Vec<Arc<dyn GrpcInboundInterceptor>>,
}

impl GrpcInboundInterceptorChain {
    /// Construct an empty chain.
    pub fn new() -> Self { Self { interceptors: Vec::new() } }

    /// Register `interceptor` at the end of the chain.
    pub fn push(mut self, interceptor: Arc<dyn GrpcInboundInterceptor>) -> Self {
        self.interceptors.push(interceptor);
        self
    }

    /// Number of registered interceptors.
    pub fn len(&self) -> usize { self.interceptors.len() }

    /// `true` when no interceptors are registered.
    pub fn is_empty(&self) -> bool { self.interceptors.is_empty() }

    /// Run every `before_dispatch` in order until one fails or all succeed.
    pub fn run_before(&self, req: &mut GrpcRequest) -> Result<(), GrpcInboundError> {
        for i in &self.interceptors {
            i.before_dispatch(req)?;
        }
        Ok(())
    }

    /// Run every `after_dispatch` in order until one fails or all succeed.
    pub fn run_after(&self, resp: &mut GrpcResponse) -> Result<(), GrpcInboundError> {
        for i in &self.interceptors {
            i.after_dispatch(resp)?;
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use std::sync::atomic::{AtomicUsize, Ordering};
    use std::time::Duration;

    use crate::api::value_object::{GrpcMetadata, GrpcStatusCode};

    use super::*;

    fn req() -> GrpcRequest {
        GrpcRequest::new("svc/M", vec![], Duration::from_secs(1))
    }

    /// @covers: GrpcInboundInterceptorChain::new — starts empty.
    #[test]
    fn test_new_chain_is_empty() {
        let chain = GrpcInboundInterceptorChain::new();
        assert_eq!(chain.len(), 0);
        assert!(chain.is_empty());
    }

    /// @covers: run_before — short-circuits on first failure.
    #[test]
    fn test_run_before_short_circuits_on_first_failure() {
        struct Deny;
        impl GrpcInboundInterceptor for Deny {
            fn before_dispatch(&self, _: &mut GrpcRequest) -> Result<(), GrpcInboundError> {
                Err(GrpcInboundError::Status(
                    GrpcStatusCode::Unauthenticated,
                    "no creds".into(),
                ))
            }
            fn after_dispatch(&self, _: &mut GrpcResponse) -> Result<(), GrpcInboundError> { Ok(()) }
        }

        struct CountAfter(Arc<AtomicUsize>);
        impl GrpcInboundInterceptor for CountAfter {
            fn before_dispatch(&self, _: &mut GrpcRequest) -> Result<(), GrpcInboundError> {
                self.0.fetch_add(1, Ordering::SeqCst);
                Ok(())
            }
            fn after_dispatch(&self, _: &mut GrpcResponse) -> Result<(), GrpcInboundError> { Ok(()) }
        }

        let counter = Arc::new(AtomicUsize::new(0));
        let chain = GrpcInboundInterceptorChain::new()
            .push(Arc::new(Deny))
            .push(Arc::new(CountAfter(counter.clone())));
        let mut r = req();
        let err = chain.run_before(&mut r).expect_err("must error");
        match err {
            GrpcInboundError::Status(GrpcStatusCode::Unauthenticated, _) => {}
            other => panic!("expected Unauthenticated, got {other:?}"),
        }
        assert_eq!(counter.load(Ordering::SeqCst), 0);
    }

    /// @covers: run_after — runs every after-hook.
    #[test]
    fn test_run_after_invokes_every_after_hook() {
        struct Count(Arc<AtomicUsize>);
        impl GrpcInboundInterceptor for Count {
            fn before_dispatch(&self, _: &mut GrpcRequest) -> Result<(), GrpcInboundError> { Ok(()) }
            fn after_dispatch(&self, _: &mut GrpcResponse) -> Result<(), GrpcInboundError> {
                self.0.fetch_add(1, Ordering::SeqCst);
                Ok(())
            }
        }
        let counter = Arc::new(AtomicUsize::new(0));
        let chain = GrpcInboundInterceptorChain::new()
            .push(Arc::new(Count(counter.clone())))
            .push(Arc::new(Count(counter.clone())));
        let mut resp = GrpcResponse {
            body: vec![],
            metadata: GrpcMetadata::default(),
        };
        chain.run_after(&mut resp).expect("after must pass");
        assert_eq!(counter.load(Ordering::SeqCst), 2);
    }

    /// @covers: GrpcInboundInterceptor — trait is object-safe.
    #[test]
    fn test_grpc_inbound_interceptor_is_object_safe() {
        fn _assert(_: &dyn GrpcInboundInterceptor) {}
    }
}
