//! Cassette policy schema. Values live in `config/application.toml`.

use serde::Deserialize;

use crate::api::error::Error;

/// Cassette record/replay policy schema.
#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CassetteConfig {
    /// Operating mode: `"replay"` | `"record"` | `"auto"`.
    pub mode: String,
    /// Cassette directory (relative to the test binary's manifest).
    pub cassette_dir: String,
    /// Request components included in the match key.
    pub match_on: Vec<String>,
    /// Headers to strip before writing the cassette.
    pub scrub_headers: Vec<String>,
    /// JSON paths inside the request body to zero out before
    /// hashing (e.g. `"request_id"`, `"metadata.trace_id"`).
    pub scrub_body_paths: Vec<String>,
}

impl CassetteConfig {
    /// Parse from TOML text.
    pub(crate) fn from_config(toml_text: &str) -> Result<Self, Error> {
        toml::from_str(toml_text).map_err(|e| Error::ParseFailed(e.to_string()))
    }

    /// Load the crate-shipped SWE baseline.
    pub(crate) fn swe_default() -> Result<Self, Error> {
        Self::from_config(include_str!("../../config/application.toml"))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// @covers: from_config
    #[test]
    fn test_from_config_parses_full_toml() {
        let toml = r#"
            mode = "auto"
            cassette_dir = "tests/fixtures"
            match_on = ["method", "url"]
            scrub_headers = ["authorization"]
            scrub_body_paths = []
        "#;
        let cfg = CassetteConfig::from_config(toml).unwrap();
        assert_eq!(cfg.mode, "auto");
    }

    /// @covers: swe_default
    #[test]
    fn test_swe_default_loads_crate_baseline() {
        let cfg = CassetteConfig::swe_default().unwrap();
        // `authorization` must be scrubbed by default — it's the
        // header the Anthropic/OpenAI SDKs use to carry the API
        // key, so committing a cassette that leaks it would leak
        // credentials.
        assert!(cfg.scrub_headers.iter().any(|h| h == "authorization"));
    }
}
