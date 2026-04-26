//! PEM-bundle identity provider. Expects a single file
//! containing BOTH the certificate chain and the private key.

use crate::api::error::Error;
use crate::api::http_tls::HttpTls;

pub(crate) struct PemHttpTls {
    pem_bytes: Vec<u8>,
    path: String,
}

impl std::fmt::Debug for PemHttpTls {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("PemHttpTls")
            .field("path", &self.path)
            .field("bytes", &format!("<{} bytes>", self.pem_bytes.len()))
            .finish()
    }
}

impl PemHttpTls {
    /// Construct by reading the .pem file into memory.
    pub(crate) fn new(path: String) -> Result<Self, Error> {
        let pem_bytes = std::fs::read(&path).map_err(|e| Error::FileReadFailed {
            path: path.clone(),
            reason: e.to_string(),
        })?;
        Ok(Self { pem_bytes, path })
    }
}

impl HttpTls for PemHttpTls {
    fn describe(&self) -> &'static str {
        "pem"
    }

    fn identity(&self) -> Result<Option<reqwest::Identity>, Error> {
        let identity = reqwest::Identity::from_pem(&self.pem_bytes).map_err(|e| {
            Error::InvalidCertificate {
                format: "pem",
                reason: e.to_string(),
            }
        })?;
        Ok(Some(identity))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// @covers: PemHttpTls::new
    #[test]
    fn test_new_reads_file_bytes_into_struct() {
        // Write a temp file with known bytes and verify new() loads them.
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("test.pem");
        std::fs::write(&path, b"fake-pem-content").unwrap();
        let p = PemHttpTls::new(path.to_str().unwrap().to_string()).unwrap();
        // The path is stored for Debug output.
        let dbg = format!("{p:?}");
        assert!(dbg.contains("16 bytes"), "debug must show byte count: {dbg}");
    }

    /// @covers: PemHttpTls::new
    #[test]
    fn test_load_missing_file_returns_file_read_failed() {
        let err = PemHttpTls::new("/path/definitely/does/not/exist.pem".into()).unwrap_err();
        assert!(matches!(err, Error::FileReadFailed { .. }));
    }

    /// @covers: PemHttpTls::identity
    #[test]
    fn test_identity_on_invalid_pem_returns_invalid_certificate() {
        let p = PemHttpTls {
            pem_bytes: b"not a real pem file".to_vec(),
            path: "<stub>".into(),
        };
        let err = p.identity().unwrap_err();
        match err {
            Error::InvalidCertificate { format, .. } => assert_eq!(format, "pem"),
            other => panic!("expected InvalidCertificate, got {other:?}"),
        }
    }

    /// @covers: PemHttpTls::describe
    #[test]
    fn test_describe() {
        let p = PemHttpTls {
            pem_bytes: vec![],
            path: "<stub>".into(),
        };
        assert_eq!(p.describe(), "pem");
    }

    #[test]
    fn test_fmt() {
        let p = PemHttpTls {
            pem_bytes: vec![],
            path: "<stub>".into(),
        };
        let _ = format!("{:?}", p);
    }
}
