//! TLS identity provider abstraction — counterpart for `core::identity`.
//!
//! Concrete identity providers (noop, PEM, PKCS#12) live in `core::identity`;
//! they implement [`HttpTls`](crate::api::http_tls::HttpTls) and are
//! selected by `core::identity::tls_factory::build_provider`.
