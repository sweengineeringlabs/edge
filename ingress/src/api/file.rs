//! File api counterpart for `core/file/`.
//!
//! Public file traits live in [`crate::api::file_inbound`].
//! Core implementations live in `core/file/`.

/// Marker type for the file api module.
#[allow(dead_code)]
pub struct File;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_file_marker_exists() {
        let _ = File;
    }
}
