# SDLC

How a change gets into `master`, how a version gets out, and where each
OpenSSF Scorecard check is closed. Written so the next person does not have to
rediscover which file owns what.

## The gate

`master` is protected by a ruleset named **default branch protection (OpenSSF)**:
no force-push, no deletion, two approving human reviews, a code-owner review
from `.github/CODEOWNERS`, dismissal of stale approvals, dismissal of stale
reviews, no admin bypass, and the `Rust checks` status check required.

The ruleset is **not applied yet**. GitHub returns
`403 — Upgrade to GitHub Pro or make this repository public to enable this
feature` for `GET /repos/TWN-Systems/operately-rs/rules/rulesets` while the
repository is private. It is applied as the last step, after the visibility
flip, with:

```bash
CARETAKER_GITHUB_TOKEN=… caretakerctl ossf apply \
  --repo TWN-Systems/operately-rs \
  --crate-name operately-sdk \
  --import-entry operately_sdk::decode \
  --code-owners '@yokoszn @twn-lloyd'
```

`caretakerctl` is idempotent by ruleset name, so re-running is safe.

## Workflows

| Workflow | Trigger | Owns |
|---|---|---|
| `ci.yml` | every PR, push to `master` | `Rust checks` (fmt/build/test/clippy/doc), MSRV 1.81, generator determinism, Semgrep |
| `supply-chain.yml` | PR, push, daily 06:13 UTC | `cargo audit --deny warnings`, `cargo deny check`, CycloneDX SBOM |
| `codeql.yml` | PR, push, Wednesdays 02:23 UTC | CodeQL `security-extended` for Rust |
| `fuzz.yml` | PR, push, daily 05:17 UTC | `cargo-fuzz` — PR runs 180 s per target, nightly goes deeper |
| `semver.yml` | PR, Mondays 05:41 UTC | `cargo-semver-checks` against the last published version |
| `release.yml` | `v*` tag | package → cosign keyless sign → SLSA provenance → publish to crates.io → GitHub Release |
| `dependabot.yml` | weekly, Mondays | grouped `cargo` and `github-actions` update PRs |

Every third-party action is pinned to a full commit SHA. Adding a tag-pinned
action is a regression — `ci.yml`'s checkout line carries a `# v5.0.0` comment
purely so the SHA stays readable.

## Version authority

`release-plz` (`release-plz.toml`, `cliff.toml`) is the **only** writer of the
workspace version and `CHANGELOG.md`. Conventional commits on `master` produce a
release PR; a human merges it; the tag it pushes is the only thing that can
start `release.yml`.

`release.yml` asserts `v{workspace version} == ${TAG_NAME}` before doing
anything, so a hand-pushed tag that disagrees with the manifest fails closed
rather than publishing a version that does not exist.

## Scorecard map

| Check | Closed by |
|---|---|
| Branch-Protection | ruleset `default branch protection (OpenSSF)` — pending visibility flip |
| CI-Tests | `ci.yml` on every PR |
| SAST | `codeql.yml` (`security-extended`) + `ci.yml` semgrep job |
| Fuzzing | `fuzz/` cargo-fuzz crate with two targets + `fuzz.yml` |
| Signed-Releases | `release.yml` cosign keyless (`.sig`/`.cert`) + SLSA (`.intoto.jsonl`) |
| Packaging | crates.io publish with `cargo package` provenance |
| Pinned-Dependencies | SHA-pinned actions throughout; Dependabot groups the bumps |
| Token-Permissions | `permissions: contents: read` at workflow level, widened per job only where needed |
| Dependency-Update-Tool | `.github/dependabot.yml` |
| Security-Policy | `SECURITY.md` |
| License | `LICENSE` (MIT) + `deny.toml` licence allowlist |
| Binary-Artifacts | no binaries are committed |
| Maintained | time-gated: needs 90 days of history |
| Code-Review | converges as approved changesets accumulate (needs 5) |
| Contributors | needs genuine multi-org contributors; not fabricated |
| CII-Best-Practices | a human files the badge at bestpractices.coreinfrastructure.org (OAuth; not drivable headlessly) |

## Semgrep: vendored, not registry

`ci.yml` runs `semgrep scan --config .semgrep` against `.semgrep/rules.yml`,
which is the project's own policy in reviewable form. A registry scan
(`p/default`) changes results without a commit, which makes "CI was green
yesterday" unverifiable. The trade-off is a narrower net; if we want the wider
one, add it as a *separate scheduled* job so the PR gate stays deterministic.

## Known gaps, in the order worth closing

1. **Branch protection is not on.** Until the repo is public, nothing forces
   these workflows to pass before merge.
2. **`Cargo.lock` was untracked.** It is tracked now — CI uses `--locked`, and an
   untracked lockfile made every `--locked` invocation a coin flip.
3. **`cargo-semver-checks` has no baseline** until 0.1.0 is on crates.io. The
   workflow reports "skipped" rather than passing silently.
4. **`deny.toml` allows `multiple-versions = "warn"`.** Deliberate for now; flip
   to `deny` once the duplicate set is empty and pin the exceptions by hand.
5. **`OperatelyClient::decode` reads the response body unbounded**
   (`resp.text()` at `crates/operately-sdk/src/lib.rs:101`). Semgrep rule
   `no-unbounded-response-read` fires on it at WARNING and is the one standing
   finding in the scan. A hostile or intercepted server can make the caller
   allocate whatever it likes. The fix is a streaming body with an explicit
   size cap — a behaviour change, so it gets its own PR rather than riding in
   on a CI change.
6. **`push_query_value` is not fuzzed.** It is `pub(crate)`, so a cargo-fuzz
   target cannot reach it without widening the public API, and it carries a
   `debug_assert!(false, ...)` for the nested-object input no catalog endpoint
   currently produces. If a nested-object query input ever appears, that
   assertion becomes reachable with attacker-influenced input — make the
   decision explicit then.
