# Attributions & Disclaimer

## Disclaimer

**operately-rs is an independent project. It is not affiliated with, endorsed by,
sponsored by, or supported by the Operately project or its maintainers.**

"Operately" is the name of the upstream open-source product this SDK is a client for.
Unlike a clean-room reverse-engineering effort against an undocumented API, Operately
itself is MIT-licensed (`github.com/operately/operately`, see their own `LICENSE`) and
publishes a machine-readable description of its external API surface —
`cli/src/generated/api-catalog.json` — specifically so third-party clients can be
generated from it. This SDK's `build.rs` does exactly that: it does not include or derive
from Operately's own server-side source code, only from that published catalog.

## References

- **[operately/operately](https://github.com/operately/operately)** — the upstream
  project. `cli/src/generated/api-catalog.json` is vendored here (see
  `crates/operately-sdk/api-catalog.json`) and drives the entire generated surface.
- **`cli/src/core/http.ts`, `external-http.ts`** — Operately's own official CLI, whose
  request/response handling this SDK's hand-written `OperatelyClient` mirrors (method
  choice, auth header, query-string flattening for list-valued inputs).
- **`lib/operately_web/router.ex`, `lib/operately_web/api/plugs/require_api_token.ex`** —
  the server-side Elixir/Phoenix routes and auth plug, read directly to confirm wire shape
  rather than assumed from `docs/api.md` (which documents the *internal* GraphQL API, a
  different surface from the *external* REST-ish API this SDK targets).

## Third-party Rust dependencies

Runtime dependencies: `reqwest` (rustls-tls, no OpenSSL), `serde`/`serde_json`,
`thiserror`, `tokio`. All permissively licensed (MIT/Apache-2.0). List them with
`cargo tree`.
