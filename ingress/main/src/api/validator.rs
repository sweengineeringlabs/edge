//! Validator api counterpart for `core/validator/`.
//!
//! The [`Validator`] trait is defined in [`crate::api::traits`].
//! Core implementations live in `core/validator/`.

/// Marker type for the validator api module.
#[allow(dead_code)]
pub struct Validator;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_validator_marker_exists() {
        let _ = Validator;
    }
}
