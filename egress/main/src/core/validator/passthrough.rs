//! PassthroughValidator — accepts all output unconditionally.

use crate::api::traits::Validator;

/// Accepts every payload without inspection.
pub(crate) struct PassthroughValidator;

impl Validator for PassthroughValidator {
    fn is_valid(&self, _input: &str) -> bool {
        true
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_is_valid_returns_true_for_any_string() {
        assert!(PassthroughValidator.is_valid("data"));
    }

    #[test]
    fn test_is_valid_returns_true_for_empty_string() {
        assert!(PassthroughValidator.is_valid(""));
    }
}
