# operately-rs

A **Rust SDK for [Operately](https://github.com/operately/operately)'s external API**
(`/api/external/v1`) — point it at any Operately deployment, self-hosted or cloud.

> **Disclaimer:** operately-rs is an independent project and is **not affiliated with,
> endorsed by, or supported by the Operately project**. "Operately" is the name of the
> upstream open-source product this SDK talks to. Unlike a clean-room reverse-engineering
> effort, though: Operately itself is MIT-licensed and publishes a machine-readable
> description of its own external API (`cli/src/generated/api-catalog.json`) specifically
> so third-party clients can be generated from it — which is exactly what this SDK does.
> See [ATTRIBUTIONS.md](ATTRIBUTIONS.md).

Workspace:

- **`crates/operately-sdk`** — the library. A `build.rs` code generator reads a vendored
  copy of Operately's own `api-catalog.json` (the same file their official CLI generates
  its TypeScript client from) and emits a full, typed Rust client — nothing hand-maintained.

## Why generate instead of hand-write

Operately's external API has 255 endpoints across 15 namespaces (spaces, projects, goals,
tasks, people, milestones, documents, links, kpis, comments, notifications, resource hubs,
reactions, files, project templates). Hand-writing and maintaining that surface by hand
would drift from the real API the moment either side changes. Generating from the same
catalog Operately's own CLI uses means this SDK stays exactly as current as that catalog is.

## How it works

Confirmed directly against the real Elixir source (not assumed from `docs/api.md`, which
describes Operately's *internal* GraphQL layer and would lead a client astray for the
*external* API specifically):

- Mutations are `POST {base_url}{path}` with the raw input object as the JSON body.
- Queries are `GET {base_url}{path}?...` with inputs flattened into the query string
  (array-valued fields as repeated `key[]=value`, matching how Elixir Plug's query decoder
  expects list-shaped params).
- Auth is always `Authorization: Bearer <token>` — a personal API token from your account's
  API Tokens page.

Primary sources: `lib/operately_web/router.ex`'s
`scope "/api/external" do forward("/v1", OperatelyWeb.Api.External) end`,
`lib/operately_web/api/plugs/require_api_token.ex`, and the official CLI's own
`cli/src/core/http.ts` / `external-http.ts`.

## Coverage

255 endpoints, 792 generated structs, 61 generated enums — the full catalog as of the
vendored `api-catalog.json`. Activity-log union types (`activity_content`,
`update_content`, etc. — large, dozens-of-variant unions used only by the activity feed)
are deliberately left as raw `serde_json::Value` rather than modeled as tagged enums;
nothing else in the catalog is skipped.

## A real, load-bearing design choice

Every generated field is `Option<T>`, regardless of what the catalog's own `optional`/
`nullable` flags declare. This isn't laziness — it's a confirmed fact about the real
server: the catalog declares `person.title` as `optional: false, nullable: false`, and a
real `people/list` call against a live deployment returned `"title": null` for two of
three people anyway (see `crates/operately-sdk/tests/real_responses.rs`, which asserts on
that exact captured response). The declared schema isn't reliable enough to gate
deserialization strictness on — a client library should stay usable against real server
behavior even when it disagrees with its own spec.

## Usage

```rust
use operately_sdk::{OperatelyClient, TasksListInput};

let client = OperatelyClient::new("https://your-operately-deployment.example", token);
let tasks = client.tasks_list(&TasksListInput {
    project_id: Some(project_id),
}).await?;
```

## Updating to a newer Operately release

1. Clone `github.com/operately/operately` and copy `cli/src/generated/api-catalog.json`
   into `crates/operately-sdk/api-catalog.json`.
2. `cargo build` — `build.rs` regenerates automatically.
3. Fix whatever the real compiler flags. Every fix so far has been a genuine shape the
   catalog can produce (a hyphenated enum variant, a multi-line docstring, self- and
   indirect-referential object cycles) — expect more of the same class, not novel ones.

## License

MIT — see [LICENSE](LICENSE).
