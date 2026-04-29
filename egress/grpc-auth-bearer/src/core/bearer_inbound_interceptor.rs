//! Inbound interceptor: validates `authorization: Bearer <jwt>`,
//! republishes the verified `sub` claim under
//! [`crate::EXTRACTED_BEARER_SUBJECT`].

use jsonwebtoken::{decode, Algorithm, DecodingKey, Validation};
use swe_edge_ingress_grpc::{
    GrpcInboundError, GrpcInboundInterceptor, GrpcRequest, GrpcResponse, GrpcStatusCode,
};

use crate::api::{
    bearer_auth_config::BearerSecret, BearerAuthError, BearerInboundConfig, AUTHORIZATION_HEADER,
    EXTRACTED_BEARER_SUBJECT,
};
use crate::core::jwt_claims::JwtClaims;

/// `GrpcInboundInterceptor` that validates incoming JWT bearer tokens.
pub struct BearerInboundInterceptor {
    config: BearerInboundConfig,
}

impl BearerInboundInterceptor {
    /// Construct from config.
    pub fn from_config(config: BearerInboundConfig) -> Self { Self { config } }

    fn validate(&self, header_value: &str) -> Result<JwtClaims, BearerAuthError> {
        let token = header_value
            .strip_prefix("Bearer ")
            .or_else(|| header_value.strip_prefix("bearer "))
            .ok_or(BearerAuthError::MalformedHeader)?
            .trim();
        if token.is_empty() {
            return Err(BearerAuthError::MalformedHeader);
        }

        let (alg, key) = match &self.config.secret {
            BearerSecret::Hs256 { secret } => {
                (Algorithm::HS256, DecodingKey::from_secret(secret))
            }
            BearerSecret::Rs256 { public_pem, .. } => (
                Algorithm::RS256,
                DecodingKey::from_rsa_pem(public_pem).map_err(BearerAuthError::InvalidToken)?,
            ),
        };

        let mut validation = Validation::new(alg);
        validation.set_audience(&[self.config.expected_audience.as_str()]);
        validation.set_issuer(&[self.config.expected_issuer.as_str()]);
        validation.leeway = self.config.leeway_seconds;
        decode::<JwtClaims>(token, &key, &validation)
            .map(|d| d.claims)
            .map_err(BearerAuthError::InvalidToken)
    }
}

impl GrpcInboundInterceptor for BearerInboundInterceptor {
    fn before_dispatch(&self, req: &mut GrpcRequest) -> Result<(), GrpcInboundError> {
        // Strip any incoming subject key — only this interceptor is
        // allowed to set it, and only after successful verification.
        req.metadata.headers.remove(EXTRACTED_BEARER_SUBJECT);

        let header = req
            .metadata
            .headers
            .get(AUTHORIZATION_HEADER)
            .cloned()
            .ok_or_else(|| GrpcInboundError::Status(
                GrpcStatusCode::Unauthenticated,
                "missing authorization header".into(),
            ))?;

        match self.validate(&header) {
            Ok(claims) => {
                req.metadata
                    .headers
                    .insert(EXTRACTED_BEARER_SUBJECT.to_string(), claims.sub);
                Ok(())
            }
            Err(BearerAuthError::MalformedHeader) | Err(BearerAuthError::MissingHeader) => {
                Err(GrpcInboundError::Status(
                    GrpcStatusCode::Unauthenticated,
                    "malformed authorization header".into(),
                ))
            }
            Err(BearerAuthError::InvalidToken(e)) => {
                tracing::warn!(error = %e, "rejecting invalid bearer token");
                Err(GrpcInboundError::Status(
                    GrpcStatusCode::Unauthenticated,
                    "invalid bearer token".into(),
                ))
            }
            Err(BearerAuthError::SignFailed(_)) | Err(BearerAuthError::InvalidSystemTime) => {
                // Should not occur on the inbound path, but handle for completeness.
                Err(GrpcInboundError::Status(
                    GrpcStatusCode::Internal,
                    "bearer token validation error".into(),
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
    use std::time::{Duration, SystemTime, UNIX_EPOCH};

    use jsonwebtoken::{encode, EncodingKey, Header};
    use swe_edge_ingress_grpc::{GrpcMetadata, GrpcRequest};

    use super::*;

    fn build_token(secret: &[u8], iss: &str, aud: &str, sub: &str, exp_offset: i64) -> String {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs();
        let exp = if exp_offset >= 0 {
            now.saturating_add(exp_offset as u64)
        } else {
            now.saturating_sub((-exp_offset) as u64)
        };
        let claims = JwtClaims {
            iss: iss.into(),
            aud: aud.into(),
            sub: sub.into(),
            iat: now,
            exp,
        };
        encode(&Header::new(Algorithm::HS256), &claims, &EncodingKey::from_secret(secret)).unwrap()
    }

    fn config(secret: &[u8]) -> BearerInboundConfig {
        BearerInboundConfig {
            secret: BearerSecret::Hs256 { secret: secret.to_vec() },
            expected_issuer: "iss".into(),
            expected_audience: "aud".into(),
            leeway_seconds: 0,
        }
    }

    fn req_with_auth(value: &str) -> GrpcRequest {
        let mut headers = std::collections::HashMap::new();
        headers.insert(AUTHORIZATION_HEADER.to_string(), value.to_string());
        GrpcRequest::new("/svc/M", vec![], Duration::from_secs(1))
            .with_metadata(GrpcMetadata { headers })
    }

    /// @covers: before_dispatch — happy path republishes `sub`.
    #[test]
    fn test_before_dispatch_republishes_subject_on_valid_token() {
        let secret = b"verystrongsecret";
        let token  = build_token(secret, "iss", "aud", "alice", 60);
        let interceptor = BearerInboundInterceptor::from_config(config(secret));
        let mut req = req_with_auth(&format!("Bearer {token}"));
        interceptor.before_dispatch(&mut req).expect("valid token");
        assert_eq!(
            req.metadata.headers.get(EXTRACTED_BEARER_SUBJECT).map(String::as_str),
            Some("alice"),
        );
    }

    /// @covers: before_dispatch — missing header returns Unauthenticated.
    #[test]
    fn test_before_dispatch_rejects_request_without_authorization_header() {
        let interceptor = BearerInboundInterceptor::from_config(config(b"sec"));
        let mut req = GrpcRequest::new("/svc/M", vec![], Duration::from_secs(1));
        match interceptor.before_dispatch(&mut req) {
            Err(GrpcInboundError::Status(GrpcStatusCode::Unauthenticated, msg)) => {
                assert!(msg.contains("authorization"));
            }
            other => panic!("expected Unauthenticated, got {other:?}"),
        }
    }

    /// @covers: before_dispatch — non-Bearer header returns Unauthenticated.
    #[test]
    fn test_before_dispatch_rejects_non_bearer_authorization_scheme() {
        let interceptor = BearerInboundInterceptor::from_config(config(b"sec"));
        let mut req = req_with_auth("Basic Zm9vOmJhcg==");
        match interceptor.before_dispatch(&mut req) {
            Err(GrpcInboundError::Status(GrpcStatusCode::Unauthenticated, _)) => {}
            other => panic!("expected Unauthenticated, got {other:?}"),
        }
    }

    /// @covers: before_dispatch — wrong signature key returns Unauthenticated.
    #[test]
    fn test_before_dispatch_rejects_token_signed_with_wrong_secret() {
        let bad   = build_token(b"otherkey", "iss", "aud", "alice", 60);
        let interceptor = BearerInboundInterceptor::from_config(config(b"verystrongsecret"));
        let mut req = req_with_auth(&format!("Bearer {bad}"));
        match interceptor.before_dispatch(&mut req) {
            Err(GrpcInboundError::Status(GrpcStatusCode::Unauthenticated, _)) => {}
            other => panic!("expected Unauthenticated, got {other:?}"),
        }
    }

    /// @covers: before_dispatch — wrong audience returns Unauthenticated.
    #[test]
    fn test_before_dispatch_rejects_token_with_wrong_audience() {
        let secret = b"sec";
        let token  = build_token(secret, "iss", "wrong-aud", "alice", 60);
        let interceptor = BearerInboundInterceptor::from_config(config(secret));
        let mut req = req_with_auth(&format!("Bearer {token}"));
        match interceptor.before_dispatch(&mut req) {
            Err(GrpcInboundError::Status(GrpcStatusCode::Unauthenticated, _)) => {}
            other => panic!("expected Unauthenticated, got {other:?}"),
        }
    }

    /// @covers: before_dispatch — expired token returns Unauthenticated.
    #[test]
    fn test_before_dispatch_rejects_expired_token() {
        let secret = b"sec";
        let token  = build_token(secret, "iss", "aud", "alice", -10);
        let interceptor = BearerInboundInterceptor::from_config(config(secret));
        let mut req = req_with_auth(&format!("Bearer {token}"));
        match interceptor.before_dispatch(&mut req) {
            Err(GrpcInboundError::Status(GrpcStatusCode::Unauthenticated, _)) => {}
            other => panic!("expected Unauthenticated, got {other:?}"),
        }
    }

    /// @covers: before_dispatch — strips spoofed subject before re-injecting.
    #[test]
    fn test_before_dispatch_strips_spoofed_subject_header_before_setting_verified_one() {
        let secret = b"sec";
        let token  = build_token(secret, "iss", "aud", "alice", 60);
        let interceptor = BearerInboundInterceptor::from_config(config(secret));
        let mut req = req_with_auth(&format!("Bearer {token}"));
        // Spoof the subject before the interceptor runs.
        req.metadata
            .headers
            .insert(EXTRACTED_BEARER_SUBJECT.to_string(), "spoofed-admin".into());
        interceptor.before_dispatch(&mut req).expect("valid token");
        assert_eq!(
            req.metadata.headers.get(EXTRACTED_BEARER_SUBJECT).map(String::as_str),
            Some("alice"),
            "interceptor must replace spoofed subject with verified one",
        );
    }
}
