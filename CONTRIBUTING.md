# Contributing

## The generated surface (`crates/operately-sdk/src/generated.rs` via `OUT_DIR`)

Never hand-edit generated output. If something about the generated client is
wrong, the fix belongs in `crates/operately-sdk/build.rs` (the generator) or in
`crates/operately-sdk/api-catalog.json` (if it's actually stale against a newer
Operately release — see the README's "Updating" section).

The generator must also be **deterministic**: the same catalog has to produce
byte-identical output on every run. `ci.yml`'s generator job builds twice and
compares the SHA-256 of `generated.rs`. If your generator change is not
deterministic, fix that before shipping it.

## Adding real test coverage

Prefer real captured response bodies over synthetic fixtures — see
`crates/operately-sdk/tests/real_responses.rs` for the pattern. A synthetic
fixture only proves the generator round-trips whatever you assumed the server
sends; a captured real response can catch the server disagreeing with its own
declared schema, which has already happened once (see the README's note on
`Option<T>` fields).

## Adding real fuzz coverage

`fuzz/` is a cargo-fuzz crate with two targets, both aimed at input this SDK
does not control:

- `fuzz_response` — a server response body fed to `serde_json::from_slice` for
  three generated types.
- `fuzz_error_body` — the `ApiErrorBody` / `ApiRaw` fork in
  `OperatelyClient::decode`, including the lossy-UTF-8 path.

If you add a new `Deserialize` shape or a new error branch, add a target rather
than a test: a fuzzer will find the panic a test never thinks to write.

```bash
cargo +nightly fuzz run fuzz_response --fuzz-dir fuzz
```

## Before opening a PR

This is the rank-0 gate. All of it must be clean:

```bash
cargo fmt --all -- --check
cargo build --locked --all-targets
cargo test --locked
cargo clippy --locked --all-targets -- -D warnings
cargo doc --locked --no-deps   # RUSTDOCFLAGS="-D warnings"
cargo deny check advisories bans licenses sources
cargo audit
```

Semgrep runs the project's own vendored rules from `.semgrep/`:

```bash
semgrep scan --config .semgrep
```

`unsafe` is forbidden workspace-wide (`workspace.lints.rust.unsafe_code =
"forbid"`) and is re-checked as a semgrep rule. If `cargo clippy` flags the
generated module specifically, fix it in `build.rs`'s emission logic, not with a
local `#[allow(...)]` — the generated module already carries a blanket allow; a
per-site allow there usually means the generator should emit better code.

## Commits, versions, and releases

The repo uses [Conventional Commits](https://www.conventionalcommits.org/), and
those messages are the input to the changelog and to the version bump — a
`fix:` becomes a patch, a `feat:` a minor, a `!:`/`BREAKING CHANGE` footer a
major. Write the message for the person reading the release notes.

[release-plz](release-plz.toml) is the **only** thing that writes
`Cargo.toml`'s version or `CHANGELOG.md`. It opens a release PR; a human merges
it; the tag it pushes triggers `.github/workflows/release.yml`, which packages,
signs, attests, and publishes to crates.io. Do not bump the version by hand and
do not run `cargo release` — two version writers racing one manifest is how a
tag and a published version drift apart.

**Agents do not merge.** Propose; a human approves.
