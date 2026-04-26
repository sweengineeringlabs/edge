//! Public builder entry point.
//!
//! Cassette is slightly different from the other middleware
//! crates — each test case typically has its own named
//! cassette file, so `build()` takes a `cassette_name` that
//! becomes part of the on-disk path.

use crate::api::cassette_config::CassetteConfig;
use crate::api::cassette_layer::CassetteLayer;
use crate::api::error::Error;

use crate::core::cassette_layer as _;

/// Start configuring the cassette with the SWE baseline.
pub fn builder() -> Result<Builder, Error> {
    let cfg = CassetteConfig::swe_default()?;
    Ok(Builder::with_config(cfg))
}

pub use crate::api::builder::Builder;

impl Builder {
    /// Construct from a caller-supplied config.
    pub fn with_config(config: CassetteConfig) -> Self {
        Self {
            config,
            cassette_name: String::new(),
        }
    }

    /// Borrow the current policy.
    pub fn config(&self) -> &CassetteConfig {
        &self.config
    }

    /// Finalize into the [`CassetteLayer`].
    ///
    /// `cassette_name` identifies the on-disk fixture file
    /// (usually one per test case). Appended to
    /// `config.cassette_dir` as `<cassette_name>.yaml`.
    pub fn build(self, cassette_name: &str) -> Result<CassetteLayer, Error> {
        CassetteLayer::new(self.config, cassette_name)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// @covers: builder
    #[test]
    fn test_builder_loads_swe_default() {
        let b = builder().expect("baseline parses");
        assert_eq!(b.config().mode, "replay");
    }

    /// @covers: Builder::build
    #[test]
    fn test_build_with_nonexistent_cassette_file_starts_empty() {
        // Use a path that definitely doesn't exist to verify
        // the "no fixture file yet" path.
        let tmpdir = tempfile::tempdir().unwrap();
        // Windows backslash → forward slash for TOML safety.
        let dir_toml_safe = tmpdir.path().to_str().unwrap().replace('\\', "/");
        let toml = format!(
            r#"
                mode = "auto"
                cassette_dir = "{dir_toml_safe}"
                match_on = ["method", "url"]
                scrub_headers = []
                scrub_body_paths = []
            "#
        );
        let cfg = CassetteConfig::from_config(&toml).unwrap();
        let _layer = Builder::with_config(cfg).build("fresh_case").expect("build ok");
    }
}
