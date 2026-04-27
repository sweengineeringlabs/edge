//! SAF layer — TLS public facade.

pub use crate::api::ingress_tls_error::IngressTlsError;
pub use crate::api::value_object::IngressTlsConfig;
pub use crate::core::server_config::build_acceptor as build_tls_acceptor;

/// Re-export so consumers can name the acceptor type without a direct
/// `tokio-rustls` dependency.
pub use tokio_rustls::TlsAcceptor;
