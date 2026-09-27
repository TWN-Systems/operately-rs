//! Rust bindings for Operately's external API (`/api/external/v1`), generated at build time
//! from `api-catalog.json` — vendored from `github.com/operately/operately`'s own
//! `cli/src/generated/api-catalog.json`, the same source their official CLI generates its
//! TypeScript client from (`docs/api.md` names the mechanism as TurboConnect, not the
//! GraphQL layer that doc otherwise describes — confirmed against `router.ex`'s
//! `scope "/api/external" do forward("/v1", OperatelyWeb.Api.External) end` and against the
//! CLI's own `core/http.ts`, which is authoritative for wire shape: mutations are `POST` with
//! the raw input object as the JSON body, queries are `GET` with inputs flattened to a query
//! string, and auth is always `Authorization: Bearer <token>` — confirmed against
//! `lib/operately_web/api/plugs/require_api_token.ex`).
//!
//! Scope: the `spaces`, `projects`, `goals`, `tasks`, `people`, `documents`, `links`,
//! `milestones` (nested under `projects`), `kpis`, `comments`, `notifications`,
//! `resource_hubs`, `reactions`, `files`, and `project_templates` namespaces — every endpoint
//! the catalog lists, all generated the same mechanical way. Activity-log union types
//! (`activity_content`, `update_content`, etc.) are deliberately left as raw `serde_json::Value`
//! rather than modeled as tagged enums — see `build.rs`'s comment at the union-emission site.

use serde::Serialize;
use serde::de::DeserializeOwned;
use serde_json::Value;

#[allow(dead_code, clippy::all)]
mod generated {
    include!(concat!(env!("OUT_DIR"), "/generated.rs"));
}
pub use generated::*;

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("http transport error: {0}")]
    Transport(#[from] reqwest::Error),
    #[error("operately api error, status {status}: {body}")]
    Api { status: u16, body: String },
    #[error("failed to decode response: {0}")]
    Decode(#[from] serde_json::Error),
}

pub type Result<T> = std::result::Result<T, Error>;

/// A client bound to one Operately deployment and one bearer token (a personal API token —
/// see Operately's "Account -> API Tokens" page, `AccountApiTokensPage` in `turboui/src`).
pub struct OperatelyClient {
    base_url: String,
    token: String,
    http: reqwest::Client,
}

impl OperatelyClient {
    pub fn new(base_url: impl Into<String>, token: impl Into<String>) -> Self {
        Self {
            base_url: base_url.into().trim_end_matches('/').to_string(),
            token: token.into(),
            http: reqwest::Client::new(),
        }
    }

    /// `POST {base_url}{path}` — body is `input` as-is (no `{inputs: ...}` wrapper; confirmed
    /// against `cli/src/core/http.ts`'s `callEndpoint`: `axios.post(url, options.inputs, ...)`).
    pub(crate) async fn mutation<In: Serialize, Out: DeserializeOwned>(&self, path: &str, input: &In) -> Result<Out> {
        let url = format!("{}{path}", self.base_url);
        let resp = self.http
            .post(&url)
            .bearer_auth(&self.token)
            .json(input)
            .send()
            .await?;
        Self::decode(resp).await
    }

    /// `GET {base_url}{path}?...` — inputs flattened into the query string. Array-valued
    /// fields are encoded as repeated `key[]=value` pairs, matching how Elixir's `Plug.Conn.Query`
    /// (the standard Phoenix/Plug query-string decoder) parses list-shaped params — plain
    /// repeated `key=a&key=b` without brackets does NOT decode as a list under Plug.
    pub(crate) async fn query<In: Serialize, Out: DeserializeOwned>(&self, path: &str, input: &In) -> Result<Out> {
        let url = format!("{}{path}", self.base_url);
        let value = serde_json::to_value(input)?;
        let pairs = flatten_query(&value);
        let resp = self.http
            .get(&url)
            .bearer_auth(&self.token)
            .query(&pairs)
            .send()
            .await?;
        Self::decode(resp).await
    }

    async fn decode<Out: DeserializeOwned>(resp: reqwest::Response) -> Result<Out> {
        let status = resp.status();
        let body = resp.text().await.unwrap_or_default();
        if !status.is_success() {
            return Err(Error::Api { status: status.as_u16(), body });
        }
        Ok(serde_json::from_str(&body)?)
    }
}

fn flatten_query(value: &Value) -> Vec<(String, String)> {
    let mut pairs = Vec::new();
    if let Value::Object(map) = value {
        for (key, v) in map {
            push_query_value(&mut pairs, key, v);
        }
    }
    pairs
}

fn push_query_value(pairs: &mut Vec<(String, String)>, key: &str, value: &Value) {
    match value {
        Value::Null => {} // matches callInternalQuery's own `if (value !== undefined && value !== null)` skip
        Value::String(s) => pairs.push((key.to_string(), s.clone())),
        Value::Bool(b) => pairs.push((key.to_string(), b.to_string())),
        Value::Number(n) => pairs.push((key.to_string(), n.to_string())),
        Value::Array(items) => {
            let bracket_key = format!("{key}[]");
            for item in items {
                push_query_value(pairs, &bracket_key, item);
            }
        }
        Value::Object(_) => {
            // No query endpoint in the current catalog needs a nested-object input — flagged
            // here rather than silently dropped if one ever does.
            debug_assert!(false, "nested object in a GET query input for key `{key}` — flatten_query needs an object convention decided first");
        }
    }
}
