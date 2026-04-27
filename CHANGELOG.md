# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

## [0.1.0] - 2026-04-27

### Added

- `swe-edge-ingress-http` — `HttpInbound` trait, `AxumHttpServer` with body-limit and graceful shutdown
- `swe-edge-ingress-grpc` — `GrpcInbound` trait, `TonicGrpcServer` with server-side TLS and mTLS
- `swe-edge-ingress-file` — `FileInbound` trait and value objects
- `swe-edge-egress-http` — `HttpOutbound` trait, `DefaultHttpOutbound` backed by reqwest-middleware
- `swe-edge-egress-grpc` — `GrpcOutbound` trait, `TonicGrpcClient` over hyper HTTP/2
- `swe-edge-egress-database` — `DatabaseGateway` trait with postgres, mysql, sqlite feature flags
- `swe-edge-egress-file` — `FileOutbound` trait and value objects
- `swe-edge-egress-notification` — `NotificationSender` trait with email and tauri feature flags
- `swe-edge-egress-payment` — `PaymentGateway` trait with stripe feature flag
- `swe-edge-egress-auth` — Bearer, Basic, API key, and AWS SigV4 middleware
- `swe-edge-egress-retry` — exponential backoff with jitter
- `swe-edge-egress-rate` — token-bucket rate limiting
- `swe-edge-egress-breaker` — circuit breaker
- `swe-edge-egress-cache` — response caching
- `swe-edge-egress-cassette` — record/replay middleware for testing
- `swe-edge-egress-tls` — mTLS client certificate middleware
- `swe-edge-proxy` — `Job → Router → LifecycleMonitor` dispatch facade
- `swe-edge-domain` — `Handler → HandlerRegistry` business logic contracts
- `swe-edge-runtime` — `RuntimeManager`, `DefaultInput`, `DefaultOutput`, graceful shutdown, systemd notify
- SEA module layout enforced across all crates (`api/`, `core/`, `saf/`)
- Full clippy `-D warnings` compliance across all workspaces
