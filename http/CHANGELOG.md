# Changelog

All notable changes to the `swe-edge/http` middleware workspace are documented here.

The format follows [Keep a Changelog](https://keepachangelog.com/en/1.1.0/).
This workspace tracks its releases in lockstep with the parent `swe-edge` tag.

---

## [Unreleased]

### Added
- `swe-http-auth`: Bearer, Basic, Header, Digest (RFC 7616), and AWS SigV4 strategies.
- `swe-http-breaker`: Per-host circuit breaker with Closed/Open/HalfOpen FSM.
- `swe-http-cache`: RFC 7234 response cache; ETag revalidation; RFC 5861 stale-while-revalidate.
- `swe-http-cassette`: VCR record/replay middleware with JSON-path body scrubbing.
- `swe-http-rate`: Token-bucket client-side rate limiter with per-host bucketing.
- `swe-http-retry`: Exponential-backoff retry with method/status filtering.
- `swe-http-tls`: PKCS12 and PEM mTLS client identity via `reqwest::ClientBuilder`.
