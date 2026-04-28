//! Configuration for the bearer interceptors.

use serde::{Deserialize, Serialize};
use subtle::ConstantTimeEq;

/// Symmetric / asymmetric secret material.
///
/// `Hs256` carries a raw byte secret; comparisons MUST go through
/// [`BearerSecret::ct_eq_hs256`] which uses `subtle::ConstantTimeEq`.
/// Asymmetric variants carry PEM-encoded key bytes; their security
/// model is the underlying signature scheme, not byte equality.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum BearerSecret {
    /// HS256 — shared symmetric secret.
    Hs256 {
        /// Raw secret bytes (UTF-8 in TOML; arbitrary bytes via API).
        secret: Vec<u8>,
    },
    /// RS256 — public + private PEM bytes.  Inbound side only needs
    /// the public PEM; outbound side needs the private PEM.
    Rs256 {
        /// PEM-encoded private key (outbound only — leave empty when
        /// loading config for an inbound-only deployment).
        #[serde(default)]
        private_pem: Vec<u8>,
        /// PEM-encoded public key (inbound side; can also be the
        /// matching pair on the outbound side for self-verification).
        #[serde(default)]
        public_pem: Vec<u8>,
    },
}

impl BearerSecret {
    /// Constant-time equality on HS256 secrets.  Returns `false` for
    /// different variants — algorithm-mismatch is never "equal".
    pub fn ct_eq_hs256(&self, other: &Self) -> bool {
        match (self, other) {
            (BearerSecret::Hs256 { secret: a }, BearerSecret::Hs256 { secret: b }) => {
                a.as_slice().ct_eq(b.as_slice()).into()
            }
            _ => false,
        }
    }
}

/// Outbound (client) bearer config.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BearerOutboundConfig {
    /// Algorithm + key material to sign the JWT.  HS256 expects raw
    /// bytes; RS256 expects PEM.
    pub secret: BearerSecret,
    /// JWT `iss` claim.
    pub issuer: String,
    /// JWT `aud` claim.
    pub audience: String,
    /// JWT `sub` claim.
    pub subject: String,
    /// Lifetime of minted tokens in seconds.  Server-side clock-skew
    /// tolerance is set on the inbound side.
    pub lifetime_seconds: u64,
}

/// Inbound (server) bearer config.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BearerInboundConfig {
    /// Verification key material.
    pub secret: BearerSecret,
    /// Required `iss` value — tokens with a different issuer are rejected.
    pub expected_issuer: String,
    /// Required `aud` value — tokens with a different audience are rejected.
    pub expected_audience: String,
    /// Maximum acceptable clock skew when checking `exp`/`nbf`, in seconds.
    pub leeway_seconds: u64,
}

#[cfg(test)]
mod tests {
    use super::*;

    /// @covers: BearerSecret::ct_eq_hs256 — equal secrets compare equal.
    #[test]
    fn test_ct_eq_hs256_returns_true_for_identical_secrets() {
        let a = BearerSecret::Hs256 { secret: b"verysecret".to_vec() };
        let b = BearerSecret::Hs256 { secret: b"verysecret".to_vec() };
        assert!(a.ct_eq_hs256(&b));
    }

    /// @covers: BearerSecret::ct_eq_hs256 — different secrets compare unequal.
    #[test]
    fn test_ct_eq_hs256_returns_false_for_different_secrets() {
        let a = BearerSecret::Hs256 { secret: b"alpha".to_vec() };
        let b = BearerSecret::Hs256 { secret: b"beta".to_vec() };
        assert!(!a.ct_eq_hs256(&b));
    }

    /// @covers: BearerSecret::ct_eq_hs256 — algorithm mismatch is never equal.
    #[test]
    fn test_ct_eq_hs256_returns_false_for_algorithm_mismatch() {
        let a = BearerSecret::Hs256 { secret: b"x".to_vec() };
        let b = BearerSecret::Rs256 { private_pem: vec![], public_pem: vec![] };
        assert!(!a.ct_eq_hs256(&b));
    }
}
