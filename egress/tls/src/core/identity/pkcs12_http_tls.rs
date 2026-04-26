//! PKCS#12-bundle identity provider.

use secrecy::{ExposeSecret, SecretString};

use crate::api::error::Error;
use crate::api::http_tls::HttpTls;

pub(crate) struct Pkcs12HttpTls {
    /// Pre-loaded DER bytes.
    der_bytes: Vec<u8>,
    /// Optional password (None for passwordless bundles).
    password: Option<SecretString>,
    /// Source path — kept for Debug / diagnostics.
    path: String,
}

impl std::fmt::Debug for Pkcs12HttpTls {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Pkcs12HttpTls")
            .field("path", &self.path)
            .field("bytes", &format!("<{} bytes>", self.der_bytes.len()))
            .field("password", &if self.password.is_some() { "<set>" } else { "<none>" })
            .finish()
    }
}

impl Pkcs12HttpTls {
    /// Construct by reading the .p12 / .pfx file into memory.
    /// Password (if any) is pre-resolved from env var before
    /// this call; see `tls_factory::build_provider`.
    pub(crate) fn new(path: String, password: Option<SecretString>) -> Result<Self, Error> {
        let der_bytes = std::fs::read(&path).map_err(|e| Error::FileReadFailed {
            path: path.clone(),
            reason: e.to_string(),
        })?;
        Ok(Self {
            der_bytes,
            password,
            path,
        })
    }
}

impl HttpTls for Pkcs12HttpTls {
    fn describe(&self) -> &'static str {
        "pkcs12"
    }

    fn identity(&self) -> Result<Option<reqwest::Identity>, Error> {
        let password = self
            .password
            .as_ref()
            .map(|p| p.expose_secret().to_string())
            .unwrap_or_default();
        let identity = reqwest::Identity::from_pkcs12_der(&self.der_bytes, &password)
            .map_err(|e| Error::InvalidCertificate {
                format: "pkcs12",
                reason: e.to_string(),
            })?;
        Ok(Some(identity))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// @covers: Pkcs12HttpTls::new
    #[test]
    fn test_new_reads_file_bytes_into_struct() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("test.p12");
        std::fs::write(&path, b"fake-pkcs12-bytes").unwrap();
        let p = Pkcs12HttpTls::new(path.to_str().unwrap().to_string(), None).unwrap();
        // The bytes were loaded (Debug shows byte count).
        let dbg = format!("{p:?}");
        assert!(dbg.contains("17 bytes"), "debug must show byte count: {dbg}");
        // No password set.
        assert!(dbg.contains("<none>"), "debug must show password absent: {dbg}");
    }

    /// @covers: Pkcs12HttpTls::new
    #[test]
    fn test_new_with_password_stores_password_flag() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("test2.p12");
        std::fs::write(&path, b"bytes").unwrap();
        let p = Pkcs12HttpTls::new(
            path.to_str().unwrap().to_string(),
            Some(secrecy::SecretString::from("secret".to_string())),
        )
        .unwrap();
        let dbg = format!("{p:?}");
        // Password is set but not leaked.
        assert!(dbg.contains("<set>"), "debug must show password is set: {dbg}");
        assert!(!dbg.contains("secret"), "password must not leak into debug: {dbg}");
    }

    /// @covers: Pkcs12HttpTls::new
    #[test]
    fn test_load_missing_file_returns_file_read_failed() {
        let err = Pkcs12HttpTls::new("/path/definitely/does/not/exist.p12".into(), None)
            .unwrap_err();
        match err {
            Error::FileReadFailed { path, .. } => assert!(path.contains("does/not/exist")),
            other => panic!("expected FileReadFailed, got {other:?}"),
        }
    }

    /// @covers: Pkcs12HttpTls::identity
    #[test]
    fn test_identity_on_invalid_der_bytes_returns_invalid_certificate() {
        // Construct with bogus DER bytes (we can't easily make a
        // real p12 in a unit test; assert that malformed data
        // hits the InvalidCertificate branch).
        let p = Pkcs12HttpTls {
            der_bytes: b"not-really-pkcs12".to_vec(),
            password: Some(SecretString::from("whatever".to_string())),
            path: "<stub>".into(),
        };
        let err = p.identity().unwrap_err();
        match err {
            Error::InvalidCertificate { format, .. } => assert_eq!(format, "pkcs12"),
            other => panic!("expected InvalidCertificate, got {other:?}"),
        }
    }

    /// @covers: Pkcs12HttpTls::describe
    #[test]
    fn test_describe() {
        let p = Pkcs12HttpTls {
            der_bytes: vec![],
            password: None,
            path: "<stub>".into(),
        };
        assert_eq!(p.describe(), "pkcs12");
    }

    #[test]
    fn test_fmt() {
        let p = Pkcs12HttpTls {
            der_bytes: vec![],
            password: None,
            path: "<stub>".into(),
        };
        let _ = format!("{:?}", p);
    }
}
