# swe-edge-ingress-grpc-reflection

Implementation of the standard `grpc.reflection.v1alpha.ServerReflection`
service for the edge gRPC stack. Lets `grpcurl`, `evans`, and BloomRPC
discover registered services and download FileDescriptorProto bytes
without compiled client stubs.

Phase 5 of the gRPC enrichment epic. Depends on Phase 3's
`HandlerRegistry` dispatcher.

## Default-off

Reflection exposes the live service surface to anyone reaching the
endpoint. The Phase 2 `GrpcServerConfig` carries an
`enable_reflection: bool` (default `false`); the Phase 3
`TonicGrpcServer` reads that flag at startup and emits a WARN when it
is `true`.

| Flag                              | Reflection RPC | Startup log               |
|-----------------------------------|----------------|---------------------------|
| `enable_reflection = false` (default) | Not registered | none                      |
| `enable_reflection = true`        | Registered     | `WARN: gRPC reflection enabled — exposes service surface to anyone reaching this endpoint. Disable in production deployments.` |

Wiring code that registers `ReflectionService` MUST gate on the flag.

## Quick start

```rust,ignore
use std::sync::Arc;
use edge_domain::HandlerRegistry;
use swe_edge_ingress_grpc::{
    HandlerRegistryDispatcher, TonicGrpcServer, GrpcServerConfig,
};
use swe_edge_ingress_grpc_reflection::{
    Descriptor, ReflectionService, REFLECTION_INFO_METHOD,
};

let registry: Arc<HandlerRegistry<Vec<u8>, Vec<u8>>> =
    Arc::new(HandlerRegistry::new());
let dispatcher = HandlerRegistryDispatcher::new(registry.clone());
// ... register your normal handlers under their gRPC method paths ...

if config.enable_reflection {
    let reflection = ReflectionService::new(registry.clone())
        .add_descriptor(Descriptor {
            filename: "myproto/svc.proto".into(),
            symbols:  vec!["pkg.Svc".into(), "pkg.Svc.DoIt".into()],
            bytes:    include_bytes!("../proto/svc.fds").to_vec(),
        });
    // Wire `reflection` into the dispatcher under REFLECTION_INFO_METHOD
    // — see `tests/reflection_int_test.rs` for an example wrapper.
}

let server = TonicGrpcServer::from_config(&config, Arc::new(dispatcher))
    .expect("config valid");
```

## Supported requests

| `ServerReflectionRequest` | Behaviour |
|---|---|
| `list_services`                  | Returns every registered method's service segment plus `grpc.reflection.v1alpha.ServerReflection`. |
| `file_by_filename`               | Returns the matching descriptor's bytes when registered; `ErrorResponse(NOT_FOUND)` otherwise. |
| `file_containing_symbol`         | Searches each registered descriptor's symbol list and returns the first match; `ErrorResponse(NOT_FOUND)` otherwise. |
| `file_containing_extension`      | `ErrorResponse(UNIMPLEMENTED)`. |
| `all_extension_numbers_of_type`  | `ErrorResponse(UNIMPLEMENTED)`. |
| absent / unknown oneof           | `ErrorResponse(INVALID_ARGUMENT)`. |

The `original_request` field is always echoed in the response so
clients can correlate request/response pairs in the bidi stream.

## Threat model

See `docs/threat_model.md`.
