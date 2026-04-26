//! `swe-edge-ingress-tls` — server-side TLS and mTLS for inbound servers.
//!
//! Provides [`IngressTlsConfig`] (loaded from PEM files) and
//! [`build_tls_acceptor`] which constructs a [`tokio_rustls::TlsAcceptor`]
//! backed by the `ring` CryptoProvider without requiring a process-level
//! default to be installed.
#![allow(dead_code)]

mod api;
mod core;
mod saf;

pub use saf::*;
