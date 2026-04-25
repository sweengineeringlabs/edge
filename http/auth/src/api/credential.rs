//! Credential-resolution abstraction — counterpart for `core::credential`.
//!
//! Concrete resolvers live in `core::credential`; they implement
//! [`CredentialResolver`](crate::api::credential_resolver::CredentialResolver)
//! using the source kinds declared in
//! [`CredentialSource`](crate::api::credential_source::CredentialSource).
