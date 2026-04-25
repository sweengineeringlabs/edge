//! End-to-end tests for the swe_edge_ingress SAF builder surface.

use swe_edge_ingress::{file_input, Builder, InboundSource};

/// @covers: file_input
#[test]
fn test_file_input_builds_and_checks_existing_path() {
    let src = file_input();
    let exists = src.file_exists(std::path::Path::new(".")).unwrap();
    assert!(exists);
}

/// @covers: Builder::new
#[test]
fn test_builder_new_constructs_successfully() {
    let _ = Builder::new();
}

/// @covers: Builder::build_file_input
#[test]
fn test_build_file_input_returns_usable_source() {
    let src = Builder::new().build_file_input();
    let exists = src.file_exists(std::path::Path::new(".")).unwrap();
    assert!(exists);
}
