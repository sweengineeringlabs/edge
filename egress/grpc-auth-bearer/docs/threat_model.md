# Threat Model — swe-edge-egress-grpc-auth-bearer

STRIDE analysis for the symmetric JWT bearer interceptors.

## Assets

- The signing secret / private key (HS256 raw bytes / RS256 PEM)
- The verifying secret / public key
- The `sub` claim that authz decisions are made on
- The validity window (`exp`, `nbf`)

## Trust boundaries

- **Outbound**: the calling process trusts the signing material it
  loads from config; the network is untrusted.
- **Inbound**: the server trusts the verifying material it loads
  from config; everything else (including the metadata bag itself)
  is untrusted.

## Threats

### S — Spoofing

| Attack                                                    | Mitigation                                                                                                                             |
|----------------------------------------------------------|----------------------------------------------------------------------------------------------------------------------------------------|
| Forged token with crafted `sub`                           | `jsonwebtoken::decode` verifies signature with the configured key.  HS256 keys are validated MAC-style; RS256 keys via RSA verification. |
| Algorithm-confusion (HS256-with-RS256-pubkey)             | The interceptor pins the algorithm based on `BearerSecret` variant — never reads `alg` from the JWT header.                            |
| Spoofed `x-edge-extracted-bearer-subject` over the wire   | Inbound interceptor ALWAYS strips this key before re-inserting the verified value (see `before_dispatch`).                              |
| Token replay across audiences                              | `expected_audience` enforced.                                                                                                          |
| Token replay across issuers                                | `expected_issuer` enforced.                                                                                                            |

### T — Tampering

| Attack                                       | Mitigation                                                  |
|---------------------------------------------|-------------------------------------------------------------|
| Bit-flip in token payload                   | MAC / signature verification rejects.                       |
| Padding-oracle on key material              | Comparisons go through `subtle::ConstantTimeEq` (HS256).    |

### R — Repudiation

JWT carries `iss`, `sub`, `iat` — the server logs the verified
`sub` on every accepted call (consumer-supplied tracing
subscriber required).  Rejection logs include the underlying
`jsonwebtoken` error reason.

### I — Information disclosure

| Attack                              | Mitigation                                                                          |
|-------------------------------------|-------------------------------------------------------------------------------------|
| Error message leaks token contents | Inbound rejection messages are static strings ("invalid bearer token") — not the raw `jsonwebtoken` error.  Detail is logged at WARN with `tracing` instead. |
| TOML config leaks secret           | The secret bytes are stored in the config struct as `Vec<u8>`; deployments are expected to inject from env vars (consumer-side concern).     |

### D — Denial of service

| Attack                                | Mitigation                                                                                  |
|--------------------------------------|---------------------------------------------------------------------------------------------|
| Many invalid tokens                  | Validation is constant-time-bounded; no DB lookups happen on the inbound path.              |
| Time-based oracle on secret length   | `subtle::ConstantTimeEq` is constant-time for inputs of the same length; mismatched lengths leak length only — acceptable since secret length is not high-entropy. |

### E — Elevation of privilege

| Attack                                                          | Mitigation                                                                                                                                                               |
|----------------------------------------------------------------|--------------------------------------------------------------------------------------------------------------------------------------------------------------------------|
| Long-lived tokens                                               | `lifetime_seconds` is a config knob.  Defaults reflect "test deployment" and consumers MUST set to the smallest value tolerable by their refresh cadence.                |
| Clock-skew tolerance set too high                               | `leeway_seconds` is configurable; consumers should keep it ≤ 30s for production deployments.                                                                              |
| Subject claim trusted before signature verified                | `before_dispatch` only writes `EXTRACTED_BEARER_SUBJECT` AFTER `jsonwebtoken::decode` returns Ok.  The pre-strip step blocks re-use of any caller-supplied value.        |
