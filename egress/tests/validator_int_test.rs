use swe_edge_egress::passthrough_validator;

#[test]
fn test_passthrough_validator_returns_implementation() {
    let _ = passthrough_validator();
}

#[test]
fn test_passthrough_validator_is_send_sync() {
    fn assert_send_sync<T: Send + Sync>(_: T) {}
    assert_send_sync(passthrough_validator());
}
