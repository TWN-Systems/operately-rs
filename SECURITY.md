# Security

## Reporting a vulnerability

Please report security issues privately rather than as a public GitHub issue — open a
[private security advisory](https://github.com/TWN-Systems/operately-rs/security/advisories/new)
on this repository, or email the maintainers directly.

## Scope

This SDK is an HTTP client: it sends whatever bearer token and request bodies you give it
to whatever `base_url` you configure. It does not store, log, or transmit credentials
anywhere other than the `Authorization` header of requests you initiate. Treat your
Operately API token the same way you'd treat any other bearer credential — do not commit
it, and prefer an environment variable or secret manager over a literal in source.

## Supply chain

Runtime dependencies are limited to `reqwest` (rustls-tls, not OpenSSL), `serde`/
`serde_json`, `thiserror`, and `tokio`. No dependency reaches out to any network endpoint
other than the one you configure via `OperatelyClient::new`.
