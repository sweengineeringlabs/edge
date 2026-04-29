//! `AuthzInterceptor` — wraps any [`AuthzPolicy`] as an inbound interceptor.
//!
//! Reads the verified caller identity from `GrpcMetadata` (populated
//! upstream by mTLS / bearer interceptors) and delegates the decision
//! to the wrapped policy.  No identity present == fail-closed
//! `Unauthenticated` (the authz interceptor MUST NOT be wired in
//! front of authn).

use std::sync::Arc;

use swe_edge_ingress_grpc::{
    AuthorizationInterceptor, GrpcInboundError, GrpcInboundInterceptor, GrpcMetadata,
    GrpcRequest, GrpcResponse, GrpcStatusCode, PeerIdentity, PEER_CN, PEER_SAN_DNS,
};

use crate::api::AuthzPolicy;

/// Inbound interceptor that runs an [`AuthzPolicy`] against the
/// authenticated caller's identity.
pub struct AuthzInterceptor {
    policy: Arc<dyn AuthzPolicy>,
}

impl AuthzInterceptor {
    /// Construct from any policy.  Use `Arc::new(MyPolicy::new())`
    /// when the policy is not already shared.
    pub fn from_policy<P: AuthzPolicy + 'static>(policy: P) -> Self {
        Self { policy: Arc::new(policy) }
    }

    /// Construct from an already-shared policy.
    pub fn from_shared_policy(policy: Arc<dyn AuthzPolicy>) -> Self {
        Self { policy }
    }
}

impl AuthzInterceptor {
    /// Build a [`PeerIdentity`] from the metadata bag the upstream
    /// auth interceptors injected.  Returns `None` when no identity
    /// is present (no CN AND no extracted bearer subject AND no SAN).
    pub(crate) fn identity_from_metadata(meta: &GrpcMetadata) -> Option<PeerIdentity> {
        const EXTRACTED_BEARER_SUBJECT: &str = "x-edge-extracted-bearer-subject";

        let cn_from_mtls = meta.headers.get(PEER_CN).cloned();
        let cn_from_bearer = meta
            .headers
            .get(EXTRACTED_BEARER_SUBJECT)
            .cloned();
        let cn = cn_from_mtls.or(cn_from_bearer);

        let san: Vec<String> = meta
            .headers
            .get(PEER_SAN_DNS)
            .map(|raw| raw.split(',').map(|s| s.trim().to_string()).collect())
            .unwrap_or_default();

        let id = PeerIdentity { cn, san, ..Default::default() };
        if id.is_empty() { None } else { Some(id) }
    }
}

impl GrpcInboundInterceptor for AuthzInterceptor {
    fn before_dispatch(&self, req: &mut GrpcRequest) -> Result<(), GrpcInboundError> {
        let identity = match Self::identity_from_metadata(&req.metadata) {
            Some(i) => i,
            None => {
                return Err(GrpcInboundError::Status(
                    GrpcStatusCode::Unauthenticated,
                    "no verified identity for authz".into(),
                ));
            }
        };

        if self.policy.allows(&identity, &req.method) {
            Ok(())
        } else {
            tracing::warn!(
                cn     = identity.cn.as_deref().unwrap_or(""),
                method = %req.method,
                "authz denied",
            );
            Err(GrpcInboundError::Status(
                GrpcStatusCode::PermissionDenied,
                "authorization denied".into(),
            ))
        }
    }

    fn after_dispatch(&self, _resp: &mut GrpcResponse) -> Result<(), GrpcInboundError> {
        Ok(())
    }

    /// Mark this interceptor as the authz gate so the server-startup
    /// default-deny check (in `swe-edge-ingress-grpc::TonicGrpcServer`)
    /// can detect that the chain enforces authorisation.
    fn is_authorization(&self) -> bool { true }
}

/// `AuthzInterceptor` is the canonical authorisation gate for inbound
/// gRPC.  Implementing the marker trait declares its role explicitly
/// alongside the runtime detection hook on `GrpcInboundInterceptor`.
impl AuthorizationInterceptor for AuthzInterceptor {}

#[cfg(test)]
mod tests {
    use std::time::Duration;

    use swe_edge_ingress_grpc::{GrpcMetadata, PEER_CN};

    use super::*;

    fn req_with_cn(cn: Option<&str>, method: &str) -> GrpcRequest {
        let mut headers = std::collections::HashMap::new();
        if let Some(cn) = cn {
            headers.insert(PEER_CN.to_string(), cn.to_string());
        }
        GrpcRequest::new(method, vec![], Duration::from_secs(1))
            .with_metadata(GrpcMetadata { headers })
    }

    /// @covers: before_dispatch — no identity = Unauthenticated.
    #[test]
    fn test_before_dispatch_returns_unauthenticated_when_no_identity_present() {
        let interceptor = AuthzInterceptor::from_policy(|_: &PeerIdentity, _: &str| true);
        let mut req = req_with_cn(None, "/svc/M");
        match interceptor.before_dispatch(&mut req) {
            Err(GrpcInboundError::Status(GrpcStatusCode::Unauthenticated, _)) => {}
            other => panic!("expected Unauthenticated, got {other:?}"),
        }
    }

    /// @covers: before_dispatch — policy allows = Ok.
    #[test]
    fn test_before_dispatch_passes_when_policy_allows_call() {
        let interceptor = AuthzInterceptor::from_policy(|id: &PeerIdentity, _: &str| {
            id.cn.as_deref() == Some("alice")
        });
        let mut req = req_with_cn(Some("alice"), "/svc/M");
        interceptor.before_dispatch(&mut req).expect("should pass");
    }

    /// @covers: before_dispatch — policy denies = PermissionDenied.
    #[test]
    fn test_before_dispatch_returns_permission_denied_when_policy_rejects() {
        let interceptor = AuthzInterceptor::from_policy(|_: &PeerIdentity, _: &str| false);
        let mut req = req_with_cn(Some("alice"), "/svc/M");
        match interceptor.before_dispatch(&mut req) {
            Err(GrpcInboundError::Status(GrpcStatusCode::PermissionDenied, _)) => {}
            other => panic!("expected PermissionDenied, got {other:?}"),
        }
    }

    /// @covers: identity_from_metadata — bearer subject also surfaces as CN.
    #[test]
    fn test_identity_from_metadata_uses_extracted_bearer_subject_when_no_mtls_cn() {
        let mut headers = std::collections::HashMap::new();
        headers.insert(
            "x-edge-extracted-bearer-subject".to_string(),
            "alice".to_string(),
        );
        let meta = GrpcMetadata { headers };
        let id   = AuthzInterceptor::identity_from_metadata(&meta).expect("identity built");
        assert_eq!(id.cn.as_deref(), Some("alice"));
    }

    /// @covers: identity_from_metadata — mTLS CN preferred over bearer subject.
    #[test]
    fn test_identity_from_metadata_prefers_mtls_cn_over_bearer_subject() {
        let mut headers = std::collections::HashMap::new();
        headers.insert(PEER_CN.to_string(), "mtls-cn".into());
        headers.insert(
            "x-edge-extracted-bearer-subject".to_string(),
            "bearer-sub".into(),
        );
        let meta = GrpcMetadata { headers };
        let id   = AuthzInterceptor::identity_from_metadata(&meta).expect("identity built");
        assert_eq!(id.cn.as_deref(), Some("mtls-cn"));
    }

    /// @covers: AuthzInterceptor::is_authorization — declares itself an authz gate.
    #[test]
    fn test_is_authorization_returns_true_for_authz_interceptor() {
        let interceptor = AuthzInterceptor::from_policy(|_: &PeerIdentity, _: &str| true);
        assert!(
            interceptor.is_authorization(),
            "AuthzInterceptor must declare itself as the authz gate so the \
             default-deny startup check can find it"
        );
    }
}
