//! swe_edge_http_tls — client-side mTLS identity for reqwest.
//!
//! Sibling crate to `swe_edge_http_auth`. Different integration
//! surface: this crate augments a `reqwest::ClientBuilder` with
//! a client identity (PKCS12 or PEM) *before* the TLS handshake,
//! whereas `swe_edge_http_auth` attaches HTTP headers *after* the
//! handshake. Both "auth" semantically; different layers
//! mechanically.
//!
//! ## Usage
//!
//! ```ignore
//! let tls = swe_edge_http_tls::builder()?.with_config(cfg).build()?;
//! let client = tls.apply_to(reqwest::Client::builder())?.build()?;
//! ```

#![warn(missing_docs)]
#![deny(unsafe_code)]
#![warn(clippy::all)]

mod api;
mod core;
mod gateway;
mod saf;

pub use gateway::*;
