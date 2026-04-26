//! Outbound gateway trait contracts.

/// Validates a string payload before outbound dispatch.
pub trait Validator: Send + Sync {
    /// Returns `true` when `input` passes validation.
    fn is_valid(&self, input: &str) -> bool;
}

#[cfg(test)]
mod tests {
    use super::*;

    struct AlwaysValid;
    impl Validator for AlwaysValid {
        fn is_valid(&self, _input: &str) -> bool { true }
    }

    #[test]
    fn test_validator_always_valid_returns_true() {
        assert!(AlwaysValid.is_valid("x"));
    }

    #[test]
    fn test_validator_always_valid_on_empty_returns_true() {
        assert!(AlwaysValid.is_valid(""));
    }
}
