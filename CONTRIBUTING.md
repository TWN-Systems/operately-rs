# Contributing

## The generated surface (`crates/operately-sdk/src/generated.rs` via `OUT_DIR`)

Never hand-edit generated output. If something about the generated client is wrong, the
fix belongs in `crates/operately-sdk/build.rs` (the generator) or in
`crates/operately-sdk/api-catalog.json` (if it's actually stale against a newer Operately
release — see the README's "Updating" section).

## Adding real test coverage

Prefer real captured response bodies over synthetic fixtures — see
`crates/operately-sdk/tests/real_responses.rs` for the pattern. A synthetic fixture only
proves the generator round-trips whatever you assumed the server sends; a captured real
response can catch the server disagreeing with its own declared schema, which has already
happened once (see the README's note on `Option<T>` fields).

## Before opening a PR

```bash
cargo build
cargo test
cargo clippy --all-targets
```

All three should be clean. If `cargo clippy` on the generated module specifically flags
something, fix it in `build.rs`'s emission logic, not with a local `#[allow(...)]` — the
generated module already carries a blanket allow; a per-site allow there usually means the
generator should just emit better code.
