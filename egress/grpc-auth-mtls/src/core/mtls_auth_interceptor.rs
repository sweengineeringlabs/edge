//! [`GrpcInboundInterceptor`] that requires an mTLS peer identity in
//! request metadata, optionally restricted to a CN/SAN allowlist.
//!
//! Wire-shape contract: relies on the keys that
//! [`swe_edge_ingress_grpc::extract_peer_identity`] injects —
//! `x-edge-peer-cn`, `x-edge-peer-san-dns`, and the always-present
//! `x-edge-peer-cert-fingerprint-sha256`.  The fingerprint key is
//! the **only** one whose presence proves a valid mTLS handshake;
//! CN/SAN are absent on degenerate certs and absence MUST NOT be
//! interpreted as "no mTLS".

use swe_edge_ingress_grpc::{
    GrpcInboundError, GrpcInboundInterceptor, GrpcRequest, GrpcResponse, GrpcStatusCode,
    PEER_CERT_FINGERPRINT_SHA256, PEER_CN, PEER_SAN_DNS,
};

use crate::api::{MtlsAuthConfig, MtlsAuthError};

/// `GrpcInboundInterceptor` that enforces mTLS-derived identity.
#[derive(Debug, Clone)]
pub struct MtlsAuthInterceptor {
    config: MtlsAuthConfig,
}

impl MtlsAuthInterceptor {
    /// Construct from config.
    pub fn from_config(config: MtlsAuthConfig) -> Self { Self { config } }

    /// Convenience: accept any peer that completed mTLS, no allowlist.
    pub fn allow_any_verified_peer() -> Self {
        Self::from_config(MtlsAuthConfig::allow_any_verified_peer())
    }

    fn classify(&self, req: &GrpcRequest) -> Result<(), MtlsAuthError> {
        // Method-level bypass takes precedence — health-checks etc.
        if self.config.allow_unauthenticated_methods
            && self
                .config
                .unauthenticated_methods
                .iter()
                .any(|m| m == &req.method)
        {
            return Ok(());
        }

        // Fingerprint is the ground-truth signal that the server's
        // mTLS code path actually populated identity.  Plaintext /
        // TLS-only conns never carry it.
        if !req
            .metadata
            .headers
            .contains_key(PEER_CERT_FINGERPRINT_SHA256)
        {
            return Err(MtlsAuthError::MissingIdentity);
        }

        // Optional CN allowlist — case-insensitive exact match.
        if !self.config.allowed_cns.is_empty() {
            let cn = req
                .metadata
                .headers
                .get(PEER_CN)
                .map(|s| s.to_ascii_lowercase());
            let allowed = cn
                .as_deref()
                .map(|cn| {
                    self.config
                        .allowed_cns
                        .iter()
                        .any(|allowed| allowed.eq_ignore_ascii_case(cn))
                })
                .unwrap_or(false);
            if !allowed {
                return Err(MtlsAuthError::DisallowedCn(cn.unwrap_or_default()));
            }
        }

        // Optional DNS SAN allowlist — at least one match required.
        if !self.config.allowed_san_dns.is_empty() {
            let sans = req
                .metadata
                .headers
                .get(PEER_SAN_DNS)
                .map(|raw| {
                    raw.split(',')
                        .map(|s| s.trim().to_ascii_lowercase())
                        .collect::<Vec<_>>()
                })
                .unwrap_or_default();
            let allowed = sans.iter().any(|san| {
                self.config
                    .allowed_san_dns
                    .iter()
                    .any(|allowed| allowed.eq_ignore_ascii_case(san))
            });
            if !allowed {
                return Err(MtlsAuthError::DisallowedSan);
            }
        }

        Ok(())
    }
}

impl GrpcInboundInterceptor for MtlsAuthInterceptor {
    fn before_dispatch(&self, req: &mut GrpcRequest) -> Result<(), GrpcInboundError> {
        match self.classify(req) {
            Ok(())                       => Ok(()),
            Err(MtlsAuthError::MissingIdentity) => Err(GrpcInboundError::Status(
                GrpcStatusCode::Unauthenticated,
                "mTLS peer identity required".into(),
            )),
            Err(MtlsAuthError::DisallowedCn(_)) | Err(MtlsAuthError::DisallowedSan) => {
                Err(GrpcInboundError::Status(
                    GrpcStatusCode::PermissionDenied,
                    "peer identity is not on the allowlist".into(),
                ))
            }
        }
    }

    fn after_dispatch(&self, _resp: &mut GrpcResponse) -> Result<(), GrpcInboundError> {
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use std::time::Duration;

    use swe_edge_ingress_grpc::{GrpcMetadata, PEER_CERT_FINGERPRINT_SHA256, PEER_CN, PEER_SAN_DNS};

    use super::*;

    fn req_no_identity() -> GrpcRequest {
        GrpcRequest::new("/svc/M", vec![], Duration::from_secs(1))
    }

    fn req_with_identity(cn: Option<&str>, san_dns: Option<&str>) -> GrpcRequest {
        let mut req = GrpcRequest::new("/svc/M", vec![], Duration::from_secs(1));
        let mut headers = std::collections::HashMap::new();
        headers.insert(
            PEER_CERT_FINGERPRINT_SHA256.to_string(),
            "deadbeef".repeat(8),
        );
        if let Some(cn) = cn {
            headers.insert(PEER_CN.to_string(), cn.to_string());
        }
        if let Some(s) = san_dns {
            headers.insert(PEER_SAN_DNS.to_string(), s.to_string());
        }
        req.metadata = GrpcMetadata { headers };
        req
    }

    /// @covers: before_dispatch — no fingerprint = Unauthenticated.
    #[test]
    fn test_before_dispatch_rejects_request_without_peer_fingerprint() {
        let interceptor = MtlsAuthInterceptor::allow_any_verified_peer();
        let mut r = req_no_identity();
        match interceptor.before_dispatch(&mut r) {
            Err(GrpcInboundError::Status(GrpcStatusCode::Unauthenticated, _)) => {}
            other => panic!("expected Unauthenticated, got {other:?}"),
        }
    }

    /// @covers: before_dispatch — fingerprint present = Ok by default.
    #[test]
    fn test_before_dispatch_accepts_any_verified_peer_when_unrestricted() {
        let interceptor = MtlsAuthInterceptor::allow_any_verified_peer();
        let mut r = req_with_identity(Some("svc-a"), Some("svc-a.local"));
        interceptor.before_dispatch(&mut r).expect("should accept");
    }

    /// @covers: CN allowlist — non-matching CN = PermissionDenied.
    #[test]
    fn test_before_dispatch_rejects_cn_outside_allowlist() {
        let interceptor =
            MtlsAuthInterceptor::from_config(MtlsAuthConfig::restrict_to_cns(["svc-a".into()]));
        let mut r = req_with_identity(Some("svc-evil"), None);
        match interceptor.before_dispatch(&mut r) {
            Err(GrpcInboundError::Status(GrpcStatusCode::PermissionDenied, _)) => {}
            other => panic!("expected PermissionDenied, got {other:?}"),
        }
    }

    /// @covers: CN allowlist — matching CN passes.
    #[test]
    fn test_before_dispatch_accepts_cn_on_allowlist() {
        let interceptor =
            MtlsAuthInterceptor::from_config(MtlsAuthConfig::restrict_to_cns(["svc-a".into()]));
        let mut r = req_with_identity(Some("svc-a"), None);
        interceptor.before_dispatch(&mut r).expect("should accept");
    }

    /// @covers: CN allowlist — case-insensitive match.
    #[test]
    fn test_before_dispatch_cn_match_is_case_insensitive() {
        let interceptor =
            MtlsAuthInterceptor::from_config(MtlsAuthConfig::restrict_to_cns(["SVC-A".into()]));
        let mut r = req_with_identity(Some("svc-a"), None);
        interceptor.before_dispatch(&mut r).expect("should accept");
    }

    /// @covers: SAN allowlist — at least one SAN match passes.
    #[test]
    fn test_before_dispatch_accepts_when_any_san_matches_allowlist() {
        let cfg = MtlsAuthConfig {
            allowed_san_dns: vec!["svc-a.local".into()],
            ..Default::default()
        };
        let interceptor = MtlsAuthInterceptor::from_config(cfg);
        let mut r = req_with_identity(None, Some("other.local,svc-a.local"));
        interceptor.before_dispatch(&mut r).expect("should accept");
    }

    /// @covers: SAN allowlist — no overlap = PermissionDenied.
    #[test]
    fn test_before_dispatch_rejects_when_no_san_matches_allowlist() {
        let cfg = MtlsAuthConfig {
            allowed_san_dns: vec!["svc-a.local".into()],
            ..Default::default()
        };
        let interceptor = MtlsAuthInterceptor::from_config(cfg);
        let mut r = req_with_identity(None, Some("other.local"));
        match interceptor.before_dispatch(&mut r) {
            Err(GrpcInboundError::Status(GrpcStatusCode::PermissionDenied, _)) => {}
            other => panic!("expected PermissionDenied, got {other:?}"),
        }
    }

    /// @covers: method bypass — listed methods skip identity check.
    #[test]
    fn test_before_dispatch_bypasses_listed_unauthenticated_method() {
        let cfg = MtlsAuthConfig {
            allow_unauthenticated_methods: true,
            unauthenticated_methods: vec!["/grpc.health.v1.Health/Check".into()],
            ..Default::default()
        };
        let interceptor = MtlsAuthInterceptor::from_config(cfg);
        let mut r = GrpcRequest::new(
            "/grpc.health.v1.Health/Check",
            vec![],
            Duration::from_secs(1),
        );
        // No peer identity — but bypassed.
        interceptor.before_dispatch(&mut r).expect("should bypass");
    }

    /// @covers: after_dispatch — no-op.
    #[test]
    fn test_after_dispatch_does_not_modify_response() {
        let interceptor = MtlsAuthInterceptor::allow_any_verified_peer();
        let mut resp = GrpcResponse {
            body:     vec![],
            metadata: GrpcMetadata::default(),
        };
        interceptor.after_dispatch(&mut resp).expect("after_dispatch no-op");
        assert!(resp.metadata.headers.is_empty());
    }
}
