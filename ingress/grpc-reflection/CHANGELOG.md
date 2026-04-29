# Changelog

## [Unreleased]

### Added
- Initial reflection crate:
  - `ReflectionService` implementing `grpc.reflection.v1alpha.ServerReflection`
  - Hand-rolled protobuf codec for `ServerReflectionRequest` /
    `ServerReflectionResponse` (no `prost` / `tonic-build`
    dependency — keeps the crate dep-light)
  - Optional `Descriptor` registration for `FileByFilename` /
    `FileContainingSymbol` lookups
  - `ListServices` derives the live service set from the Phase 3
    `HandlerRegistry` and always advertises the reflection
    self-name
  - Default-off integration: `GrpcServerConfig::enable_reflection`
    (default `false`) gates registration; `TonicGrpcServer` logs
    `REFLECTION_ENABLED_WARN_MSG` at startup when the flag is `true`
