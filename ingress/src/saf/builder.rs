//! SAF builder — constructs inbound adapters.

use std::sync::Arc;

use crate::api::input::InboundSource;
use crate::core::file::LocalFileSource;

/// Returns the default local-filesystem inbound source.
pub fn file_input() -> impl InboundSource {
    LocalFileSource
}

/// Builder for inbound adapter configuration.
#[derive(Debug, Default)]
pub struct Builder;

impl Builder {
    /// Construct with default configuration.
    pub fn new() -> Self {
        Self
    }

    /// Build the default local-filesystem inbound source.
    pub fn build_file_input(self) -> Arc<dyn InboundSource> {
        Arc::new(LocalFileSource)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// @covers: file_input
    #[test]
    fn test_file_input_returns_inbound_source() {
        let src = file_input();
        // Verify the adapter is callable (compile-time proof of trait impl).
        let _ = src.file_exists(std::path::Path::new(".")).unwrap();
    }

    /// @covers: Builder::new
    #[test]
    fn test_new_constructs_builder() {
        let _ = Builder::new();
    }

    /// @covers: Builder::build_file_input
    #[test]
    fn test_build_file_input_returns_arc_inbound_source() {
        let src = Builder::new().build_file_input();
        let _ = src.file_exists(std::path::Path::new(".")).unwrap();
    }
}
