# Changelog

## Unreleased

- Initial release: `operately-sdk` generated from Operately's `api-catalog.json` —
  255 endpoints, 792 structs, 61 enums. Every field modeled `Option<T>` (see README for
  why). Real deserialization tests against captured live-server responses.
- `Error::Api` now carries a structured `ApiErrorBody { error, message, details }`, matching
  the real TurboConnect error shape (`app/lib/turbo_connect/plugs/dispatch.ex`). Non-JSON
  error bodies (e.g. the auth plug's plain-text `401 Unauthorized`) fall back to a new
  `Error::ApiRaw { status, body }` variant instead of failing to parse.
