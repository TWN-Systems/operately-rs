# Security Policy

## Reporting a vulnerability

**Do not open a public issue for a security problem.**

Report it privately via
[GitHub's private vulnerability reporting](https://github.com/TWN-Systems/operately-rs/security/advisories/new)
on this repository (Security ▸ Report a vulnerability), or by email to the
maintainers listed in [CODEOWNERS](.github/CODEOWNERS).

Please include: the affected version, a minimal reproducer, and what an attacker
gains. We aim to acknowledge within 3 business days and to ship a fix or a
mitigation plan within 30 days of a confirmed report. We will credit you in the
advisory unless you ask us not to.

## Supported versions

| Version | Supported |
|---|---|
| 0.1.x | ✅ |
| < 0.1 | ❌ |

This crate has not reached 1.0. Treat every `0.x` minor as a possible breaking
release: `cargo-semver-checks` runs against the last published version on every
PR (`.github/workflows/semver.yml`), so a breaking change must say so in its
commit message and land in a minor bump.

## Scope

This SDK is an HTTP client. It sends whatever bearer token and request bodies
you give it to whatever `base_url` you configure. It does not store, log, or
transmit credentials anywhere other than the `Authorization` header of requests
you initiate.

In scope:

- Anything that lets a hostile or intercepted response cause a panic,
  unbounded allocation, or unbounded CPU in the caller's process.
- Credential disclosure through an error message, panic, or log line.
- TLS verification being disabled by any code path.
- The vendored `api-catalog.json` and the `build.rs` generator that consumes it
  — these are third-party artifacts running at build time.

Out of scope:

- Vulnerabilities in the Operately server itself — report those to
  [operately/operately](https://github.com/operately/operately).
- Malicious or misconfigured `base_url` values. A client you point at an
  attacker-controlled host talks to that host; that is the contract.
- Denial of service caused by the *caller* handing us an unbounded request body
  to serialize.

## Supply chain

Runtime dependencies are limited to `reqwest` (rustls-tls, not OpenSSL),
`serde`/`serde_json`, `thiserror`, and `tokio`. `deny.toml` is the policy:
`cargo deny check advisories bans licenses sources` runs on every push and
scheduled sweep, `cargo audit --deny warnings` runs the same way, and a
CycloneDX SBOM is published as a build artifact.

Every third-party action in `.github/workflows/` is pinned to a full commit
SHA, not a tag. Releases are published to crates.io by
`.github/workflows/release.yml`, which signs the `.crate` with cosign
(keyless, Sigstore) and attaches an SLSA build provenance attestation:

```bash
cosign verify-blob \
  --certificate operately-sdk-0.1.0.crate.cert \
  --signature operately-sdk-0.1.0.crate.sig \
  --certificate-identity-regexp '^https://github.com/TWN-Systems/operately-rs/\.github/workflows/release\.yml@refs/tags/v[0-9.]+$' \
  --certificate-oidc-issuer 'https://token.actions.githubusercontent.com' \
  operately-sdk-0.1.0.crate
```

`Cargo.lock` is committed on purpose: it is what makes `cargo audit`,
`cargo deny`, and the SBOM describe the exact dependency set that ships.

## Project security posture

- [OpenSSF Scorecard](https://scorecard.dev/viewer/?uri=github.com/TWN-Systems/operately-rs)
- [SECURITY.md](SECURITY.md) — this file
- [CONTRIBUTING.md](CONTRIBUTING.md) — the local verification gate
- [docs/SDLC.md](docs/SDLC.md) — how CI, releases, and compliance fit together
