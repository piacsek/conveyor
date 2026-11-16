# Phase 47: a failure records what kind it is

## Context
Issue #91. When a fetch or an action fails, the reason travels as text: `src/sources/github.rs`
and `src/sources/kube.rs` return `io::Result`, `src/sources/fetch.rs` turns that into a
`String`, and `refresh.rs`, the `Input` variants and `App` carry `Result<_, String>`. Nothing
can react to the kind of failure (signed out, rate limited, offline) without parsing messages.

This phase only makes the kind available. Every card, footer and log line reads exactly as it
does now, so it ships as `kind: internal`. What a card does with a kind is a later issue.

## Verified facts (2026-11-16, gh 2.104.0, kubectl v1.36.1)
Each `gh` line was run with a scratch `HOME` and `GH_CONFIG_DIR`, stdin null.

- **Signed out** (no token, empty config): `gh api user`, `gh api graphql` and `gh pr view`
  all exit 4 with
  `To get started with GitHub CLI, please run:  gh auth login` then
  `Alternatively, populate the GH_TOKEN environment variable with a GitHub API authentication token.`
- **A token GitHub refuses** (`GH_TOKEN=ghp_invalid`): `gh api user` and `gh api graphql` exit 1
  with `gh: Bad credentials (HTTP 401)` on stderr (the JSON body goes to stdout);
  `gh pr view` exits 1 with `HTTP 401: Bad credentials (https://api.github.com/graphql)` then
  `Try authenticating with:  gh auth login -h github.com`.
- **Cannot connect, name does not resolve** (`GH_HOST=nosuchhost.invalid`): `gh api` and
  `gh pr view` exit 1 with `error connecting to nosuchhost.invalid` then
  `check your internet connection or https://githubstatus.com`.
- **Cannot connect, connection refused** (`HTTPS_PROXY=http://127.0.0.1:1`): `gh api user`
  exits 1 with
  `Get "https://api.github.com/user": proxyconnect tcp: dial tcp 127.0.0.1:1: connect: connection refused`;
  `gh api graphql` and `gh pr view` print the same with `Post "https://api.github.com/graphql"`.
- **Rate limited: not triggered live** (it would mean spending the account's budget). GitHub's
  docs (REST "Rate limits", GraphQL "Rate limits and query limits") give the statuses, 403 or
  429 for REST and 200 (or 403 for a secondary limit) for GraphQL, and no sample message, only
  "an error message that indicates that you exceeded a secondary rate limit". So the status is
  no key (a 403 is also a permission error, and GraphQL answers 200) and the text is: `gh`
  prints the API's message as `gh: <message> (HTTP <status>)`, `HTTP <status>: <message> (<url>)`
  or `GraphQL: <message>` (`pkg/api/errors.go` of `cli/go-gh`), and the messages known for it
  (`API rate limit exceeded for …`, `You have exceeded a secondary rate limit …`) all hold the
  words `rate limit`. The rule matches those two words, in any case.
- **kubectl missing** and **gh missing**: `Command::spawn` fails with
  `io::ErrorKind::NotFound`; both runners already map it to their install hint
  (`kube::MISSING`, `Tool::missing`).
- **kubectl timed out**: `CliKubectl::run` already returns
  `kubectl did not answer within <N>s` when `Ran::TimedOut` comes back from the deadline.
- kubectl against a scratch kubeconfig, for the record (not classified in this phase, see
  "Left"): a refused server ends in
  `The connection to the server 127.0.0.1:1 was refused - did you specify the right host or port?`,
  a name that does not resolve in
  `Unable to connect to the server: dial tcp: lookup nosuchhost.invalid: no such host`, a
  missing context in `error: context "missing" does not exist`, each after klog lines.

## States

| Kind | Recognised by | Card shows |
| --- | --- | --- |
| `ProgramMissing` | spawning `gh`, `git` or `kubectl` fails with `NotFound` | unchanged |
| `RateLimited` | `gh` failed and its stderr holds `rate limit`, in any case | unchanged |
| `NotSignedIn` | `gh` failed and its stderr holds `gh auth login` or `HTTP 401` | unchanged |
| `Offline` | `gh` failed and its stderr holds `error connecting to` or `dial tcp` | unchanged |
| `TimedOut` | the kubectl deadline killed the process | unchanged |
| `Other` | none of the above: any other exit of `gh`, every failed exit of `git` and `kubectl`, invalid JSON or UTF-8, a parse error, a message conveyor wrote itself | unchanged |

The rules are tried in the order of the table, so a message that names two things takes the
first. Only a failed `gh` is classified by its text: `git` (the checkout) and `kubectl` print
other wordings, not asked for by the issue.

## Design
- `src/sources/failure.rs`: `Failure { kind: FailureKind, message: String }`, with
  `Failure::kind()`, `Display` printing the message alone, and `From<String>` giving `Other`
  (what a parse error or a message conveyor wrote becomes). `FailureKind` is the six kinds
  above.
- The `Github` and `Kube` traits return `Result<_, Failure>` instead of `io::Result`. The
  classification sits where stderr is read: `CliGh::exec` (for the `gh` tool) and
  `CliKubectl::run`. The helpers that rewrite a message (`without_warnings`, `gits_reason`,
  `not_a_repository`) keep the kind.
- `fetch.rs` returns `Result<_, Failure>` and drops its `map_err(|err| err.to_string())`.
  `refresh::spawn`, every `Input` variant that carries a result, the `receive_*` functions and
  `Column::error` hold a `Failure`. A Deployed row's `error` and `pods_error` hold one too,
  since a timeout is per row, never per column.
- The `Result<_, String>` of `menu`, `pull_target`, `build_url`, the `*_target` functions and
  `Offer::choice` stay: they say why a key does nothing on this row, written by conveyor, no
  external service involved. So do `src/model/` parsers; `From<String>` lifts them at `?`.
- Everything that draws or logs calls `to_string()` (or `Display`), so no text changes.

## Tasks
Each starts from a failing test, outside-in: a `gh` or `kubectl` shim prints the verified text,
the real runner and the real fetch run, and the test asks the column (or the row) for the kind
and for the unchanged text.

1. Signed out, on My PRs: `Failure`, the `gh auth login` rule, `Github` returning `Failure`,
   `fetch_prs`, `refresh::spawn`, `Input::Data`, `Column::error`. Ships with the fragment
   `changes/phase-47-failure-kinds.md` (`kind: internal`, no bullet) and this plan.
2. The other `gh` kinds, one test each: `HTTP 401` (both wordings), rate limited (primary and
   secondary wording), offline (both wordings), `gh` missing, and an unrecognised exit as
   `Other`.
3. The rest of `fetch.rs` (queue, builds, jobs, log, rerun, repos) returns `Failure`.
4. kubectl: missing, timed out and any other exit, asked from the Deployed row (`error`) and
   from its pods (`pods_error`).
5. The action results (`Merged`, `Rerun`, `BranchUpdated`, `StatusChanged`, `CheckedOut`,
   `Jobs`, `Log`) carry `Failure`; a failed checkout with `git` missing reads `ProgramMissing`.
6. Guidelines: the layout line for `failure.rs` in the root `AGENTS.md`, the kinds and their
   rules in `src/sources/AGENTS.md`, what `App` holds in `src/app/AGENTS.md`, the phase outcome
   in `PLAN.md`.

## Done when
A test can ask a failed column, a Deployed row and a failed action which kind of failure it
holds, and `git diff origin/main -- tests/snapshots` is empty.

## Left
- kubectl's own connection errors read `Other`. Their wording is in "Verified facts" for the
  issue that makes Deployed react to them.
- What a card does with each kind.
