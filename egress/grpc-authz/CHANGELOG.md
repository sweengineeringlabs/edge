# Changelog

## [Unreleased]

### Added
- Initial authz interceptor with pluggable `AuthzPolicy` trait.
- Built-in `MethodAclPolicy` with TOML-driven `MethodAclConfig`.
- Closure impl for `AuthzPolicy` to ease test wiring.
- `PeerIdentity` adapter that reads either the mTLS CN or the
  bearer-extracted subject from `GrpcMetadata`.
