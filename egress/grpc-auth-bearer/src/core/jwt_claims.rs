//! Shared JWT claims schema.

use serde::{Deserialize, Serialize};

/// Standard JWT claims set used by the bearer interceptors.  Custom
/// claims are intentionally not modelled — extending this struct is
/// the migration path when a deployment needs additional fields.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub(crate) struct JwtClaims {
    pub iss: String,
    pub aud: String,
    pub sub: String,
    pub exp: u64,
    pub iat: u64,
}
