//! Default impl of [`HttpAuth`](crate::api::http_auth::HttpAuth).

use crate::api::auth_config::AuthConfig;
use crate::api::http_auth::HttpAuth;

#[derive(Debug)]
pub(crate) struct DefaultHttpAuth {
    #[allow(dead_code)]
    config: AuthConfig,
}

impl DefaultHttpAuth {
    pub(crate) fn new(config: AuthConfig) -> Self {
        Self { config }
    }
}

impl HttpAuth for DefaultHttpAuth {
    fn describe(&self) -> &'static str {
        "swe_http_auth"
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// @covers: describe
    #[test]
    fn test_describe_returns_crate_name() {
        let cfg = AuthConfig::swe_default().expect("baseline parses");
        let d = DefaultHttpAuth::new(cfg);
        assert_eq!(d.describe(), "swe_http_auth");
    }
}
