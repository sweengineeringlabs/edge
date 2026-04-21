//! Public builder entry point.

use crate::api::error::Error;

/// Start configuring the cassette middleware. Scaffold phase:
/// returns a handle whose `build()` yields
/// [`Error::NotImplemented`]. When the impl lands, `build()`
/// returns a `reqwest_middleware::Middleware` layer.
pub fn builder() -> Builder {
    Builder(())
}

/// Builder handle for the cassette middleware. Opaque during
/// scaffold phase.
#[derive(Debug)]
pub struct Builder(());

impl Builder {
    /// Finalize the middleware layer. Scaffold phase: returns
    /// `NotImplemented`.
    pub fn build(self) -> Result<(), Error> {
        Err(Error::NotImplemented("builder"))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// @covers: builder
    #[test]
    fn test_builder_returns_handle() {
        let _b = builder();
    }

    /// @covers: Builder::build
    #[test]
    fn test_build_returns_not_implemented_during_scaffold_phase() {
        let err = builder().build().unwrap_err();
        assert!(matches!(err, Error::NotImplemented(_)));
    }
}
