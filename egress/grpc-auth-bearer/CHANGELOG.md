# Changelog

## [Unreleased]

### Added
- Initial bearer interceptors:
  - `BearerOutboundInterceptor` (HS256 / RS256 sign + inject)
  - `BearerInboundInterceptor` (validate + republish `sub`)
  - Reserved metadata key `x-edge-extracted-bearer-subject`
  - Constant-time HS256 secret comparison via `subtle::ConstantTimeEq`
