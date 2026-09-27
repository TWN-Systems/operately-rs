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

use serde::de::DeserializeOwned;
use serde::Serialize;
use serde_json::Value;

#[allow(dead_code, clippy::all)]
mod generated {
    include!(concat!(env!("OUT_DIR"), "/generated.rs"));
}
pub use generated::*;

/// The real TurboConnect error-response shape (`app/lib/turbo_connect/plugs/dispatch.ex`):
/// `{"error": "<category>", "message": "<text>"}`, with `details` present only on some
/// `bad_request` responses. `error` is a fixed category string ("Bad request", "Unauthorized",
/// "Forbidden", "Not found", "Internal server error") but modeled as `String`, not an enum —
/// an upstream proxy (502/504) or a crash before this plug runs can return a body this shape
/// doesn't cover at all, which is why `Error::Api` below falls back to the raw string rather
/// than requiring this to parse.
#[derive(Debug, Clone, serde::Deserialize)]
pub struct ApiErrorBody {
    pub error: String,
    pub message: String,
    #[serde(default)]
    pub details: Option<Value>,
}

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("http transport error: {0}")]
    Transport(#[from] reqwest::Error),
    /// A structured TurboConnect error response.
    #[error("operately api error, status {status}: {} — {}", body.error, body.message)]
    Api { status: u16, body: ApiErrorBody },
    /// A non-2xx response whose body didn't parse as `ApiErrorBody` — e.g. the auth plug's
    /// plain-text `"Unauthorized"` (`require_api_token.ex` sends `send_resp(conn, 401,
    /// "Unauthorized")`, not JSON) or a proxy error page.
    #[error("operately api error, status {status}: {body}")]
    ApiRaw { status: u16, body: String },
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
        let resp = self.http.post(&url).bearer_auth(&self.token).json(input).send().await?;
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
        let resp = self.http.get(&url).bearer_auth(&self.token).query(&pairs).send().await?;
        Self::decode(resp).await
    }

    async fn decode<Out: DeserializeOwned>(resp: reqwest::Response) -> Result<Out> {
        let status = resp.status();
        let body = resp.text().await.unwrap_or_default();
        if !status.is_success() {
            return Err(match serde_json::from_str::<ApiErrorBody>(&body) {
                Ok(parsed) => Error::Api { status: status.as_u16(), body: parsed },
                Err(_) => Error::ApiRaw { status: status.as_u16(), body },
            });
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

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn flatten_query_skips_null_fields() {
        let pairs = flatten_query(&json!({"id": "abc", "milestone_id": null}));
        assert_eq!(pairs, vec![("id".to_string(), "abc".to_string())]);
    }

    #[test]
    fn flatten_query_encodes_scalars() {
        let pairs = flatten_query(&json!({"name": "hi", "done": true, "count": 3}));
        assert!(pairs.contains(&("name".to_string(), "hi".to_string())));
        assert!(pairs.contains(&("done".to_string(), "true".to_string())));
        assert!(pairs.contains(&("count".to_string(), "3".to_string())));
        assert_eq!(pairs.len(), 3);
    }

    #[test]
    fn flatten_query_encodes_arrays_as_bracket_key() {
        let pairs = flatten_query(&json!({"ids": ["a", "b", "c"]}));
        assert_eq!(
            pairs,
            vec![
                ("ids[]".to_string(), "a".to_string()),
                ("ids[]".to_string(), "b".to_string()),
                ("ids[]".to_string(), "c".to_string()),
            ]
        );
    }

    #[test]
    fn flatten_query_array_skips_null_elements() {
        let pairs = flatten_query(&json!({"ids": ["a", null, "b"]}));
        assert_eq!(pairs, vec![("ids[]".to_string(), "a".to_string()), ("ids[]".to_string(), "b".to_string())]);
    }

    #[test]
    fn flatten_query_non_object_top_level_is_empty() {
        assert_eq!(flatten_query(&json!("just a string")), Vec::new());
        assert_eq!(flatten_query(&json!(null)), Vec::new());
    }

    #[test]
    #[cfg_attr(not(debug_assertions), ignore = "debug_assert! only fires in debug builds")]
    #[should_panic(expected = "flatten_query needs an object convention decided first")]
    fn flatten_query_panics_on_nested_object() {
        flatten_query(&json!({"filter": {"status": "open"}}));
    }
}
