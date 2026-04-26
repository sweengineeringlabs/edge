//! Credential-resolution impls — one per source kind.

pub(crate) mod env_credential_resolver;

pub(crate) use env_credential_resolver::EnvCredentialResolver;
