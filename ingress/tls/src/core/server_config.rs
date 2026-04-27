//! Builds a `tokio_rustls::TlsAcceptor` from an [`IngressTlsConfig`].
//!
//! Uses the `ring` CryptoProvider explicitly via
//! `ServerConfig::builder_with_provider` so no process-level default needs to
//! be installed by the caller.

use std::fs;
use std::io::BufReader;
use std::sync::Arc;

use rustls::pki_types::{CertificateDer, PrivateKeyDer};
use rustls::ServerConfig;
use rustls_pemfile::{certs, private_key};

use crate::api::ingress_tls_error::IngressTlsError;
use crate::api::value_object::IngressTlsConfig;

/// Build a [`tokio_rustls::TlsAcceptor`] from `config`.
///
/// Reads cert, key, and (for mTLS) CA files from disk. Returns an error if any
/// file cannot be read or the rustls config cannot be built. Fails fast — all
/// I/O happens here, not on the first accepted connection.
pub fn build_acceptor(
    config: &IngressTlsConfig,
) -> Result<tokio_rustls::TlsAcceptor, IngressTlsError> {
    let server_cfg = build_server_config(config)?;
    Ok(tokio_rustls::TlsAcceptor::from(server_cfg))
}

fn build_server_config(config: &IngressTlsConfig) -> Result<Arc<ServerConfig>, IngressTlsError> {
    let cert_chain = load_certs(&config.cert_pem_path)?;
    let key        = load_key(&config.key_pem_path)?;

    let provider = Arc::new(rustls::crypto::ring::default_provider());

    let builder = ServerConfig::builder_with_provider(provider.clone())
        .with_safe_default_protocol_versions()
        .map_err(|e| IngressTlsError::Config(e.to_string()))?;

    let mut cfg = if let Some(ca_path) = &config.client_ca_pem_path {
        let roots = load_client_ca(ca_path)?;
        let verifier = rustls::server::WebPkiClientVerifier::builder_with_provider(
            Arc::new(roots),
            provider,
        )
        .build()
        .map_err(|e| IngressTlsError::Config(e.to_string()))?;

        builder
            .with_client_cert_verifier(verifier)
            .with_single_cert(cert_chain, key)
            .map_err(|e| IngressTlsError::Config(e.to_string()))?
    } else {
        builder
            .with_no_client_auth()
            .with_single_cert(cert_chain, key)
            .map_err(|e| IngressTlsError::Config(e.to_string()))?
    };

    // Advertise both HTTP/2 and HTTP/1.1 via ALPN so that
    // hyper_util::server::conn::auto::Builder can select the right protocol.
    cfg.alpn_protocols = vec![b"h2".to_vec(), b"http/1.1".to_vec()];

    Ok(Arc::new(cfg))
}

fn load_certs(path: &str) -> Result<Vec<CertificateDer<'static>>, IngressTlsError> {
    let file = fs::File::open(path)
        .map_err(|e| IngressTlsError::CertLoad(path.to_string(), e))?;
    let chain: Result<Vec<_>, _> = certs(&mut BufReader::new(file)).collect();
    let chain = chain.map_err(|e| IngressTlsError::CertParse(e.to_string()))?;
    if chain.is_empty() {
        return Err(IngressTlsError::CertParse(format!(
            "no certificates found in {path}"
        )));
    }
    Ok(chain)
}

fn load_key(path: &str) -> Result<PrivateKeyDer<'static>, IngressTlsError> {
    let file = fs::File::open(path)
        .map_err(|e| IngressTlsError::KeyLoad(path.to_string(), e))?;
    private_key(&mut BufReader::new(file))
        .map_err(|e| IngressTlsError::KeyParse(e.to_string()))?
        .ok_or_else(|| IngressTlsError::KeyParse(format!("no private key found in {path}")))
}

fn load_client_ca(path: &str) -> Result<rustls::RootCertStore, IngressTlsError> {
    let file = fs::File::open(path)
        .map_err(|e| IngressTlsError::CertLoad(path.to_string(), e))?;
    let mut store = rustls::RootCertStore::empty();
    for cert in certs(&mut BufReader::new(file)) {
        let cert = cert.map_err(|e| IngressTlsError::CertParse(e.to_string()))?;
        store
            .add(cert)
            .map_err(|e| IngressTlsError::CertParse(e.to_string()))?;
    }
    if store.is_empty() {
        return Err(IngressTlsError::CertParse(format!(
            "no CA certificates found in {path}"
        )));
    }
    Ok(store)
}

#[cfg(test)]
mod tests {
    use std::io::Write as _;

    use super::*;

    fn write_temp(content: &str) -> tempfile::NamedTempFile {
        let mut f = tempfile::NamedTempFile::new().unwrap();
        f.write_all(content.as_bytes()).unwrap();
        f
    }

    fn self_signed() -> (String, String) {
        let cert = rcgen::generate_simple_self_signed(vec!["localhost".to_string()]).unwrap();
        (cert.cert.pem(), cert.key_pair.serialize_pem())
    }

    #[test]
    fn test_build_acceptor_with_valid_tls_config_succeeds() {
        let (cert_pem, key_pem) = self_signed();
        let cert_f = write_temp(&cert_pem);
        let key_f  = write_temp(&key_pem);
        let cfg = IngressTlsConfig::tls(
            cert_f.path().to_str().unwrap(),
            key_f.path().to_str().unwrap(),
        );
        assert!(build_acceptor(&cfg).is_ok());
    }

    #[test]
    fn test_build_acceptor_with_missing_cert_file_returns_cert_load_error() {
        let cfg = IngressTlsConfig::tls("/nonexistent/cert.pem", "/nonexistent/key.pem");
        let Err(err) = build_acceptor(&cfg) else { panic!("expected error for missing cert") };
        assert!(matches!(err, IngressTlsError::CertLoad(_, _)));
    }

    #[test]
    fn test_build_acceptor_with_missing_key_file_returns_key_load_error() {
        let (cert_pem, _) = self_signed();
        let cert_f = write_temp(&cert_pem);
        let cfg = IngressTlsConfig::tls(
            cert_f.path().to_str().unwrap(),
            "/nonexistent/key.pem",
        );
        let Err(err) = build_acceptor(&cfg) else { panic!("expected error for missing key") };
        assert!(matches!(err, IngressTlsError::KeyLoad(_, _)));
    }

    #[test]
    fn test_build_acceptor_with_empty_cert_file_returns_cert_parse_error() {
        let cert_f = write_temp("");
        let (_, key_pem) = self_signed();
        let key_f = write_temp(&key_pem);
        let cfg = IngressTlsConfig::tls(
            cert_f.path().to_str().unwrap(),
            key_f.path().to_str().unwrap(),
        );
        let Err(err) = build_acceptor(&cfg) else { panic!("expected error for empty cert file") };
        assert!(matches!(err, IngressTlsError::CertParse(_)));
    }

    #[test]
    fn test_ingress_tls_config_tls_has_no_client_ca() {
        let cfg = IngressTlsConfig::tls("a.crt", "a.key");
        assert!(!cfg.is_mtls());
        assert!(cfg.client_ca_pem_path.is_none());
    }

    #[test]
    fn test_ingress_tls_config_mtls_has_client_ca() {
        let cfg = IngressTlsConfig::mtls("a.crt", "a.key", "ca.crt");
        assert!(cfg.is_mtls());
        assert_eq!(cfg.client_ca_pem_path.as_deref(), Some("ca.crt"));
    }
}
