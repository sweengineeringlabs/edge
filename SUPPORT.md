# Support

## Getting Help

- **Documentation**: See the [docs/](docs/) directory for rationale, conventions, and audit reports
- **Issues**: Open an issue on [GitHub](https://github.com/sweengineeringlabs/edge/issues) for bug reports or feature requests
- **Discussions**: Use [GitHub Discussions](https://github.com/sweengineeringlabs/edge/discussions) for questions and general help

## Frequently Asked Questions

### Why five separate workspaces instead of one?

Each workspace represents an independent SEA layer. Consumers only pull in the layers they need — a service that only serves HTTP inbound has no compile-time dependency on the egress or runtime workspaces.

### Can I use Edge with an existing Axum or Tonic application?

Yes. `AxumHttpServer` and `TonicGrpcServer` in `ingress/` wrap Axum and Tonic respectively. Your business logic implements the `HttpInbound` or `GrpcInbound` trait and remains decoupled from the framework.

### How do I swap the HTTP transport without changing business logic?

Business logic implements an `api/` trait. The SAF factory (`saf::http_server(...)`) returns `impl HttpInbound`. Replacing the concrete server type in `core/` requires no changes to the application layer.

### Where do I configure middleware (auth, retry, rate limits)?

Policy lives in TOML configuration, never as hardcoded Rust literals. See `egress/auth/config/application.toml` for the auth middleware schema as an example.
