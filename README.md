# operately

Rust bindings for [Operately](https://github.com/operately/operately)'s external API
(`/api/external/v1`), for a self-hosted deployment at `projects.tas.twn.network`.

## How this was built

Operately's own official CLI (`cli/` in their repo) generates its TypeScript client from a
single machine-readable file, `cli/src/generated/api-catalog.json` — a structural description
of every namespace, endpoint, input, output, object, enum, and union in the external API.
This crate's `build.rs` reads that same file (vendored here, see "Updating" below) and
generates a full Rust client the same way: nothing in `src/generated.rs`-via-`OUT_DIR` is
hand-maintained.

Confirmed directly against the real Elixir source (not assumed from `docs/api.md`, which
describes the internal GraphQL layer and is misleading for this purpose):

- Mutations are `POST {base}{path}` with the raw input object as the JSON body.
- Queries are `GET {base}{path}?...` with inputs flattened into the query string
  (array-valued fields as repeated `key[]=value`, matching how Plug's own query decoder
  expects list params).
- Auth is always `Authorization: Bearer <token>` (a personal API token from
  Account → API Tokens).

See `lib/operately_web/router.ex`'s `scope "/api/external" do forward("/v1", ...) end`,
`lib/operately_web/api/plugs/require_api_token.ex`, and the CLI's own
`cli/src/core/http.ts`/`external-http.ts` for the primary sources.

## Coverage

255 endpoints, 792 generated structs, 61 generated enums — the full catalog as of the
vendored `api-catalog.json`'s `schema_version`. Activity-log union types
(`activity_content`, `update_content`, etc. — large, dozens-of-variants unions used by the
activity feed) are deliberately left as raw `serde_json::Value` rather than modeled as tagged
enums; nothing else in the catalog is skipped.

## A real, load-bearing design choice

Every generated field is `Option<T>`, regardless of what the catalog's own `optional`/
`nullable` flags say. This isn't laziness — it's a confirmed fact about the real server:
`person.title` is declared `optional: false, nullable: false` in this exact catalog, and a
real `people/list` call against the live deployment returned `"title": null` for two of
three people anyway (see `tests/real_responses.rs`, which asserts on that exact response).
The declared schema is not reliable enough to gate deserialization strictness on.

## Updating

Operately releases a new `api-catalog.json` with each version. To pick up a newer schema:

1. Clone `github.com/operately/operately` and copy `cli/src/generated/api-catalog.json` here.
2. `cargo build` — `build.rs` regenerates automatically (see its `cargo:rerun-if-changed`).
3. Fix whatever the real compiler flags. Every fix so far has been a genuine shape the
   catalog can produce (a hyphenated enum variant, a multi-line docstring, self- and
   indirect-referential object cycles) — expect more of the same class, not novel categories.

## Usage

```rust
let client = operately::OperatelyClient::new("https://projects.tas.twn.network", token);
let tasks = client.tasks_list(&operately::TasksListInput {
    project_id: Some(project_id),
}).await?;
```
