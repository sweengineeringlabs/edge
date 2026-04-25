//! Identity-provider impls — one per TLS config variant.

pub(crate) mod noop_http_tls;
pub(crate) mod pem_http_tls;
pub(crate) mod pkcs12_http_tls;
pub(crate) mod tls_factory;

pub(crate) use tls_factory::build_provider;
