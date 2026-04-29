# grpc-breaker

gRPC-aware circuit breaker decorator wrapping any
[`GrpcOutbound`](https://docs.rs/swe-edge-egress-grpc) implementor.

## Build

```bash
cargo build
```

## Test

```bash
cargo test
```

## Project Structure

- `src/api/` - Public types and traits (L2)
- `src/core/` - Implementation layer (L3)
- `src/saf/` - Public facade (L4)
- `src/gateway/` - Public entry boundary
- `config/application.toml` - Default breaker policy
