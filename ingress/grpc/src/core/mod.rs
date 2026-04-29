//! Core layer — gRPC inbound implementations (pub(crate) only).
pub(crate) mod grpc_timeout;
pub(crate) mod handler_dispatch;
pub(crate) mod health_service;
pub(crate) mod interceptor;
pub(crate) mod peer_identity;
pub(crate) mod server;
pub(crate) mod status_codes;
