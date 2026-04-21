//! Factory — turns a [`TlsConfig`] into a concrete
//! [`Box<dyn HttpTls>`].

use secrecy::SecretString;

use crate::api::error::Error;
use crate::api::http_tls::HttpTls;
use crate::api::tls_config::TlsConfig;

use super::noop_http_tls::NoopHttpTls;
use super::pem_http_tls::PemHttpTls;
use super::pkcs12_http_tls::Pkcs12HttpTls;

/// Realize a [`TlsConfig`] into the right provider impl.
///
/// File reads + env-var resolution happen NOW. Missing files
/// return [`Error::FileReadFailed`]; missing passwords (when
/// the config specifies one) return [`Error::MissingEnvVar`].
pub(crate) fn build_provider(config: &TlsConfig) -> Result<Box<dyn HttpTls>, Error> {
    match config {
        TlsConfig::None => Ok(Box::new(NoopHttpTls)),

        TlsConfig::Pkcs12 {
            path,
            password_env,
        } => {
            let password = match password_env {
                Some(var) => {
                    let v = std::env::var(var).map_err(|_| Error::MissingEnvVar {
                        name: var.clone(),
                    })?;
                    Some(SecretString::from(v))
                }
                None => None,
            };
            Ok(Box::new(Pkcs12HttpTls::load(path.clone(), password)?))
        }

        TlsConfig::Pem { path } => Ok(Box::new(PemHttpTls::load(path.clone())?)),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// @covers: build_provider
    #[test]
    fn test_none_variant_builds_noop() {
        let p = build_provider(&TlsConfig::None).unwrap();
        assert_eq!(p.describe(), "noop");
        assert!(p.identity().unwrap().is_none());
    }

    /// @covers: build_provider
    #[test]
    fn test_pkcs12_with_missing_password_env_returns_missing_env_var() {
        std::env::remove_var("EDGE_TEST_TLS_PKCS_PW_ABSENT_01");
        let cfg = TlsConfig::Pkcs12 {
            path: "irrelevant.p12".into(),
            password_env: Some("EDGE_TEST_TLS_PKCS_PW_ABSENT_01".into()),
        };
        let err = build_provider(&cfg).unwrap_err();
        match err {
            Error::MissingEnvVar { name } => {
                assert_eq!(name, "EDGE_TEST_TLS_PKCS_PW_ABSENT_01");
            }
            other => panic!("expected MissingEnvVar, got {other:?}"),
        }
    }

    /// @covers: build_provider
    #[test]
    fn test_pkcs12_with_missing_file_returns_file_read_failed() {
        let cfg = TlsConfig::Pkcs12 {
            path: "/path/definitely/does/not/exist.p12".into(),
            password_env: None,
        };
        let err = build_provider(&cfg).unwrap_err();
        match err {
            Error::FileReadFailed { path, .. } => assert!(path.contains("does/not/exist")),
            other => panic!("expected FileReadFailed, got {other:?}"),
        }
    }

    /// @covers: build_provider
    #[test]
    fn test_pem_with_missing_file_returns_file_read_failed() {
        let cfg = TlsConfig::Pem {
            path: "/path/definitely/does/not/exist.pem".into(),
        };
        let err = build_provider(&cfg).unwrap_err();
        assert!(matches!(err, Error::FileReadFailed { .. }));
    }

    #[test]
    fn test_build_provider() {
        let p = build_provider(&TlsConfig::None);
        assert!(p.is_ok());
    }
}
