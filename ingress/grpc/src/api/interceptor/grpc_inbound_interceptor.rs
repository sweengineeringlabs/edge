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

    /// Whether this interceptor enforces authorisation.
    ///
    /// Implementations of [`AuthorizationInterceptor`] **must** override
    /// this to return `true` so the server-startup default-deny check
    /// can detect them.  The default `false` keeps non-authz
    /// interceptors out of the gate-discovery path.
    fn is_authorization(&self) -> bool { false }
}

/// Marker trait for an inbound interceptor that gates dispatch on
/// authorization.
///
/// Implementors **must** also override
/// [`GrpcInboundInterceptor::is_authorization`] to return `true` so
/// the default-deny startup check (in [`crate::TonicGrpcServer`]) can
/// detect that an authz gate is wired in.
///
/// ## Why a marker trait?
///
/// At server startup we need to ask the chain "is at least one
/// interceptor an authz gate?" without dispatching a request.  A
/// marker trait on its own is not visible through `Arc<dyn
/// GrpcInboundInterceptor>`, so we pair it with the
/// `is_authorization()` method on the base trait — the marker is the
/// declarative contract, the method is the runtime detection hook.
pub trait AuthorizationInterceptor: GrpcInboundInterceptor {}

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

    /// `true` when at least one registered interceptor declares itself
    /// an [`AuthorizationInterceptor`] via
    /// [`GrpcInboundInterceptor::is_authorization`].
    pub fn contains_authorization(&self) -> bool {
        self.interceptors.iter().any(|i| i.is_authorization())
    }

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

    /// @covers: GrpcInboundInterceptor::is_authorization — defaults to false.
    #[test]
    fn test_is_authorization_defaults_to_false_for_plain_interceptors() {
        struct Plain;
        impl GrpcInboundInterceptor for Plain {
            fn before_dispatch(&self, _: &mut GrpcRequest) -> Result<(), GrpcInboundError> { Ok(()) }
            fn after_dispatch(&self, _: &mut GrpcResponse) -> Result<(), GrpcInboundError> { Ok(()) }
        }
        assert!(!Plain.is_authorization());
    }

    /// @covers: GrpcInboundInterceptorChain::contains_authorization — false on empty chain.
    #[test]
    fn test_contains_authorization_returns_false_for_empty_chain() {
        let chain = GrpcInboundInterceptorChain::new();
        assert!(!chain.contains_authorization());
    }

    /// @covers: GrpcInboundInterceptorChain::contains_authorization — false when no authz interceptors.
    #[test]
    fn test_contains_authorization_returns_false_when_chain_has_no_authz_gates() {
        struct Plain;
        impl GrpcInboundInterceptor for Plain {
            fn before_dispatch(&self, _: &mut GrpcRequest) -> Result<(), GrpcInboundError> { Ok(()) }
            fn after_dispatch(&self, _: &mut GrpcResponse) -> Result<(), GrpcInboundError> { Ok(()) }
        }
        let chain = GrpcInboundInterceptorChain::new()
            .push(Arc::new(Plain))
            .push(Arc::new(Plain));
        assert!(!chain.contains_authorization());
    }

    /// @covers: GrpcInboundInterceptorChain::contains_authorization — true when at least one authz gate present.
    #[test]
    fn test_contains_authorization_returns_true_when_at_least_one_authz_gate_is_present() {
        struct Plain;
        impl GrpcInboundInterceptor for Plain {
            fn before_dispatch(&self, _: &mut GrpcRequest) -> Result<(), GrpcInboundError> { Ok(()) }
            fn after_dispatch(&self, _: &mut GrpcResponse) -> Result<(), GrpcInboundError> { Ok(()) }
        }
        struct Authz;
        impl GrpcInboundInterceptor for Authz {
            fn before_dispatch(&self, _: &mut GrpcRequest) -> Result<(), GrpcInboundError> { Ok(()) }
            fn after_dispatch(&self, _: &mut GrpcResponse) -> Result<(), GrpcInboundError> { Ok(()) }
            fn is_authorization(&self) -> bool { true }
        }
        impl AuthorizationInterceptor for Authz {}
        let chain = GrpcInboundInterceptorChain::new()
            .push(Arc::new(Plain))
            .push(Arc::new(Authz));
        assert!(chain.contains_authorization());
    }
}
