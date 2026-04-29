//! `swe-edge-ingress-grpc-reflection` — implementation of the
//! standard `grpc.reflection.v1alpha.ServerReflection` service for
//! grpcurl / evans / BloomRPC.
//!
//! Phase 5 of the edge gRPC enrichment epic.  Plugs into the
//! Phase 3 [`HandlerRegistry`] dispatcher: register the reflection
//! service alongside your normal handlers and grpcurl can list
//! every method you serve.
//!
//! ## Default-off in production
//!
//! Reflection exposes the service surface to anyone who can reach
//! the endpoint.  This crate ships only the implementation; the
//! Phase 2 [`GrpcServerConfig`] carries the `enable_reflection`
//! flag (default `false`) — wiring code that registers
//! [`ReflectionService`] MUST gate on that flag.
//!
//! ## Quick start (server)
//!
//! ```rust,ignore
//! use std::sync::Arc;
//! use edge_domain::HandlerRegistry;
//! use swe_edge_ingress_grpc::{HandlerRegistryDispatcher, TonicGrpcServer};
//! use swe_edge_ingress_grpc_reflection::{ReflectionService, REFLECTION_INFO_METHOD};
//!
//! let registry: Arc<HandlerRegistry<Vec<u8>, Vec<u8>>> = Arc::new(HandlerRegistry::new());
//! let dispatcher = HandlerRegistryDispatcher::new(registry.clone());
//! // ... register your normal handlers ...
//!
//! if config.enable_reflection {
//!     let reflection = ReflectionService::new(registry.clone());
//!     // (Wiring: feed the dispatcher every method path, including REFLECTION_INFO_METHOD.)
//! }
//! ```
//!
//! [`HandlerRegistry`]: edge_domain::HandlerRegistry
//! [`GrpcServerConfig`]: swe_edge_ingress_grpc::GrpcServerConfig

#![warn(missing_docs)]
#![deny(unsafe_code)]
#![warn(clippy::all)]

mod api;
mod core;
mod gateway;
mod saf;

pub use gateway::*;
