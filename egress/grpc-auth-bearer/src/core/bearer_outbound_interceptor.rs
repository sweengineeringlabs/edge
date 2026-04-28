//! Outbound interceptor: signs a JWT and injects it as
//! `authorization: Bearer <jwt>` on every request.

use std::time::{SystemTime, UNIX_EPOCH};

use jsonwebtoken::{Algorithm, EncodingKey, Header};
use swe_edge_egress_grpc::{
    GrpcOutboundError, GrpcOutboundInterceptor, GrpcRequest, GrpcResponse, GrpcStatusCode,
};

use crate::api::{
    bearer_auth_config::BearerSecret, BearerAuthError, BearerOutboundConfig, AUTHORIZATION_HEADER,
};
use crate::core::jwt_claims::JwtClaims;

/// `GrpcOutboundInterceptor` that signs and attaches a JWT bearer
/// token to every outbound request.
pub struct BearerOutboundInterceptor {
    config: BearerOutboundConfig,
}

impl BearerOutboundInterceptor {
    /// Construct from config.
    pub fn from_config(config: BearerOutboundConfig) -> Self { Self { config } }

    fn sign_token(&self) -> Result<String, BearerAuthError> {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map_err(|_| BearerAuthError::InvalidSystemTime)?
            .as_secs();
        let claims = JwtClaims {
            iss: self.config.issuer.clone(),
            aud: self.config.audience.clone(),
            sub: self.config.subject.clone(),
            iat: now,
            exp: now.saturating_add(self.config.lifetime_seconds),
        };
        let (alg, key) = match &self.config.secret {
            BearerSecret::Hs256 { secret } => (
                Algorithm::HS256,
                EncodingKey::from_secret(secret),
            ),
            BearerSecret::Rs256 { private_pem, .. } => (
                Algorithm::RS256,
                EncodingKey::from_rsa_pem(private_pem)
                    .map_err(BearerAuthError::SignFailed)?,
            ),
        };
        jsonwebtoken::encode(&Header::new(alg), &claims, &key)
            .map_err(BearerAuthError::SignFailed)
    }
}

impl GrpcOutboundInterceptor for BearerOutboundInterceptor {
    fn before_call(&self, req: &mut GrpcRequest) -> Result<(), GrpcOutboundError> {
        let token = self.sign_token().map_err(|e| {
            tracing::warn!(error = %e, "failed to mint bearer token");
            GrpcOutboundError::Status(
                GrpcStatusCode::Internal,
                "failed to mint bearer token".into(),
            )
        })?;
        req.metadata.headers.insert(
            AUTHORIZATION_HEADER.to_string(),
            format!("Bearer {token}"),
        );
        Ok(())
    }

    fn after_call(&self, _resp: &mut GrpcResponse) -> Result<(), GrpcOutboundError> {
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use std::time::Duration;

    use swe_edge_egress_grpc::GrpcRequest;

    use super::*;

    fn hs256_config(secret: &[u8]) -> BearerOutboundConfig {
        BearerOutboundConfig {
            secret: BearerSecret::Hs256 { secret: secret.to_vec() },
            issuer: "test-iss".into(),
            audience: "test-aud".into(),
            subject: "test-sub".into(),
            lifetime_seconds: 60,
        }
    }

    /// @covers: before_call — injects a Bearer authorization header.
    #[test]
    fn test_before_call_injects_bearer_authorization_header() {
        let interceptor = BearerOutboundInterceptor::from_config(hs256_config(b"sec"));
        let mut req = GrpcRequest::new("/svc/M", vec![], Duration::from_secs(1));
        interceptor.before_call(&mut req).expect("before_call");
        let auth = req
            .metadata
            .headers
            .get(AUTHORIZATION_HEADER)
            .cloned()
            .expect("header injected");
        assert!(
            auth.starts_with("Bearer "),
            "expected Bearer-prefixed header, got {auth}",
        );
        // Body must be three dot-separated base64url segments.
        let token = auth.trim_start_matches("Bearer ");
        assert_eq!(token.matches('.').count(), 2, "JWT shape: {token}");
    }

    /// @covers: sign_token — round-trips through jsonwebtoken's verifier.
    #[test]
    fn test_sign_token_round_trips_through_jsonwebtoken_verifier() {
        use jsonwebtoken::{decode, DecodingKey, Validation};

        let interceptor = BearerOutboundInterceptor::from_config(hs256_config(b"sec"));
        let token = interceptor.sign_token().expect("sign");

        let mut validation = Validation::new(Algorithm::HS256);
        validation.set_audience(&["test-aud"]);
        validation.set_issuer(&["test-iss"]);
        let decoded = decode::<JwtClaims>(
            &token,
            &DecodingKey::from_secret(b"sec"),
            &validation,
        )
        .expect("verify");
        assert_eq!(decoded.claims.iss, "test-iss");
        assert_eq!(decoded.claims.aud, "test-aud");
        assert_eq!(decoded.claims.sub, "test-sub");
    }
}
