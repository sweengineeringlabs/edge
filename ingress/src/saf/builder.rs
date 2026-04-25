//! SAF builder — constructs inbound adapters.

use crate::api::inbound_source::InboundSource;
use crate::api::traits::Validator;
use crate::core::file::LocalFileSource;
use crate::core::validator::PassthroughValidator;

/// Returns the default local-filesystem inbound source.
pub fn file_input() -> impl InboundSource {
    LocalFileSource
}

/// Returns the default passthrough validator (accepts all input).
pub fn passthrough_validator() -> impl Validator {
    PassthroughValidator
}

#[cfg(test)]
mod tests {
    use super::*;

    /// @covers: file_input
    #[test]
    fn test_file_input_returns_inbound_source() {
        let src = file_input();
        let _ = src.file_exists(std::path::Path::new(".")).unwrap();
    }

    /// @covers: passthrough_validator
    #[test]
    fn test_passthrough_validator_accepts_all_input() {
        let v = passthrough_validator();
        assert!(v.is_valid("hello"));
        assert!(v.is_valid(""));
    }
}
