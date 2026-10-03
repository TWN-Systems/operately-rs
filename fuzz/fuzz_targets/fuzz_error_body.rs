//! Fuzz the error path, which is the one place this SDK parses a body it
//! expects *not* to be JSON.
//!
//! `OperatelyClient::decode` forks: it tries `ApiErrorBody` first and falls
//! back to `Error::ApiRaw { body: String }` when that fails — the auth plug
//! (`require_api_token.ex`) really does answer a bare `Unauthorized` string,
//! and an upstream proxy can answer with an HTML error page. Both the
//! structured parse and the raw fallback must be total over arbitrary bytes.
//!
//! This mirrors `decode`'s body handling exactly, so a crash here is a crash
//! there.

#![no_main]

use libfuzzer_sys::fuzz_target;
use operately_sdk::ApiErrorBody;

fuzz_target!(|data: &[u8]| {
    // `resp.text()` substitutes U+FFFD on invalid UTF-8 rather than failing,
    // so the lossy path is the reachable one — not `from_utf8`'s Err arm.
    let body: String = String::from_utf8_lossy(data).into_owned();

    match serde_json::from_str::<ApiErrorBody>(&body) {
        Ok(parsed) => {
            // `Error::Api`'s Display impl interpolates both fields; a crafted
            // body must not be able to panic on formatting.
            let _ = format!(
                "operately api error, status {}: {} — {}",
                500, parsed.error, parsed.message
            );
        }
        Err(_) => {
            // The ApiRaw branch, including a lossy body containing interior
            // nulls or lone surrogates.
            let _ = format!("operately api error, status 500: {body}");
        }
    }
});
