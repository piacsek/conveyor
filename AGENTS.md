# AGENTS.md

Notes for agents and humans working on `conveyor`. Keep this file short and
factual; verify against the code before treating anything here as live state.

## What this is

A Rust + Ratatui TUI that shows where a change is on its way to production:
my open pull requests, the merge queue, the last main builds, and what each
environment runs. One column per stage, side by side; a focused column,
actions on the selected row. `conveyor config` prints the effective config.
`PLAN.md` holds the phase history, retros, and the backlog.

## Layout

```
src/main.rs      CLI dispatch, config load, wiring
src/cli.rs       `conveyor` (TUI) | `config` | `--help` | `--version`; hand-rolled, no clap
src/config.rs    Config (serde + toml, deny_unknown_fields), XDG path, `CONVEYOR_CONFIG` override
src/github.rs    Github trait (`graphql(query, vars)`), CliGh spawns `gh api graphql`, error mapping
src/fetch.rs     fetch_prs(gh, config): the search query from `src/queries/prs.graphql` → parsed rows
src/model/prs.rs PullRequest, CheckState, Check, ReviewDecision, MergeState; lenient `parse` of gh JSON
src/app.rs       App state (Mode::{Normal, Filter, Help}, focus, details), Input::{Key, Data, Tick}, run()
src/ui.rs        rendering: 4 columns or tabs below `4 × ui.min_column_width`; KEYS drives the help view
src/open.rs      Opener trait; SystemOpener (`open`/`xdg-open`, `pbcopy`/`xclip`)
src/text.rs      truncate/pad_right with `…`, age()
src/queries/     GraphQL documents, `include_str!`ed
scripts/gates.sh              the quality gates; fails loudly, never pipe it through tail
scripts/dev-install.sh        release build symlinked as ~/.local/bin/conveyor-dev
scripts/ship.sh               gates + dev-install + commit + push, aborts on any failure
scripts/homebrew-formula.sh   prints the tap formula for a released version
scripts/screenshots.sh        renders docs/*.png from a gh shim + synthetic fixture (truecolor.py converts ANSI)
tests/           outside-in: `tests/cli.rs` runs the real binary; TUI tests drive run() with a TestBackend
```

## Data sources (verified 2026-09-10, all through `gh`)

- **My PRs**: GraphQL `search(query: <prs.query>, type: ISSUE)`. Useful fields:
  `number title isDraft reviewDecision mergeStateStatus statusCheckRollup{state contexts}`
  `mergeQueueEntry{position state} updatedAt url headRefName repository{nameWithOwner}`.
  `contexts` is a union of `CheckRun` (`name status conclusion detailsUrl`) and
  `StatusContext` (`context state targetUrl`). `mergeStateStatus` is often `UNKNOWN`
  (GitHub computes it lazily): show it as `—`, never as an error.
- **Merge queue**: `repository.mergeQueue.entries` (`position state enqueuedAt
  estimatedTimeToMerge solo jump headCommit{oid} pullRequest{number title author}`).
  Entry `state` ∈ AWAITING_CHECKS | LOCKED | MERGEABLE | QUEUED | UNMERGEABLE. The
  merge-group workflow runs (`gh run list --event merge_group`) live on branches
  `gh-readonly-queue/main/pr-<N>-<base sha>`; their `displayTitle` is the workflow name.
  Rulesets return nothing for <repo> (classic protection); the GraphQL object is the source.
- **Main builds**: `repos/{r}/actions/workflows/<file>/runs?branch=main&event=push`. Many
  workflows run on main (`issue_comment` bots, per-app deploys), so the build workflow is
  per-repo config (default `CI/CD`). Squash titles end in `(#N)`; `repos/{r}/commits/{sha}/pulls`
  maps the rest.
- **Deployed**: `kubectl --context C -n NS get deploy D -o jsonpath=…image`; the tag is the
  40-hex commit sha (Argo CD image-updater, `newest-build`). Teleport sessions expire daily,
  so a failed fetch is the normal case: keep the last rows dim and show the error.
- Parse leniently: unknown fields ignored, unknown enum values → `Unknown`.
  `tests/fixtures/` carries verbatim `gh` output; refresh it when GitHub changes shape.

## Behaviour that is easy to break

- **`--help`/`-h`/`help` and `--version`/`-V`/`version`** print to stdout and exit 0 before
  the config is read, so a broken config never hides them.
- **Config is loaded before any subcommand runs.** Missing file = `Config::default()`; an
  unreadable file or unknown key is an error naming the file (`deny_unknown_fields` on every
  table). `conveyor config` exits 1 on it and doubles as the reference for defaults.
- **Errors stay inside the TUI.** A failed action or fetch sets an error drawn in the footer
  or column title; the app never exits on it.
- **Draw before the first read.** `run()` renders, then waits for input.
- **`gh` never inherits the terminal.** `CliGh::run` gives it a null stdin, so a `gh` that
  wants to prompt (no auth, `HOME` pointing elsewhere) fails fast instead of hanging the
  fetch thread behind `fetching…`.
- **Colors** come from the ANSI palette so terminal themes apply. Do not hardcode hex.

## Testing traps hit so far

- `tests/e2e.rs` runs the real binary in a scratch tmux server (`-L`, `-f /dev/null`) with
  `HOME` in a tempdir and a `gh` shell shim that prints `tests/fixtures/prs.json`. The pane
  command must be `env HOME=… PATH=… <binary>`: `respawn-pane -e PATH=…` looked right but
  the pane's login shell rebuilt `PATH` (macOS `path_helper` and rc files), the shim was
  never found and the real `gh` ran. Sockets are per test so parallel tests never share a
  server.
- Rows in a 40-column pane hold about 33 characters after the number, glyph, repo and age;
  behaviour tests use short titles, snapshots carry the truncation.

## Documentation rule

`README.md` embeds `docs/columns.png`, `docs/details.png` and `docs/tabs.png`. After any
visible layout change run `scripts/screenshots.sh` (needs `brew install
charmbracelet/tap/freeze`, Google Chrome, and the FiraCode Nerd Font in `~/Library/Fonts`) and
commit the new images. Freeze lays out an SVG with the font embedded; headless Chrome
rasterises it, because freeze's own PNG output cannot draw Nerd Font glyphs. The script never
reads the real `gh` account: `HOME` is a tempdir and a `gh` shim printing a synthetic
`acme/*` fixture is first on PATH (through `env`, see the e2e trap below). Do not screenshot
real data: this repository is public.

Whenever behaviour changes, check that `README.md` still covers it and is still true: a key,
a default, a subcommand, a config key. The README must stay terse: one line per feature,
defaults in the TOML block, no prose that repeats the code. Depth belongs here.

## Development

Strict TDD: one failing test, minimal code, refactor. Outside-in: start from the binary or
from `run()` with scripted inputs, drop to unit tests only for pure helpers. No code
comments; names and tests carry the intent.

Gates, all required before a task is done, wrapped by `scripts/gates.sh`:

```
cargo fmt --check
cargo clippy --all-targets -- -D warnings
cargo test
cargo test -- --ignored        # e2e: real binary with a `gh` shim first on PATH
```

Ship with `scripts/ship.sh "<message>"`: gates, `dev-install.sh`, commit, push, all under
`set -e`, refusing to run on `main`. The first ship on a branch also opens the draft PR
(`gh pr create --draft --assignee @me`); a branch must never carry commits without one.
A PR with a visible change embeds the `docs/*.png` screenshots in its body, as commit-pinned
`raw.githubusercontent.com` URLs so they outlive the branch. Do not hand-roll the chain: the interactive shell here is
zsh, where `PIPESTATUS` is undefined and `test "" -eq 0` is true, so a `gates.sh | grep` guard
silently passed a clippy failure into a commit on 2026-09-10.

Run the wrapper, not the commands: piping `gates.sh | tail` once hid a `cargo fmt --check`
failure in tmux-agents and an unformatted commit slipped through. Chaining gates with `&&`
and then `;` before `git commit` does the same (it happened on this repo's bootstrap commit).

Install: `scripts/dev-install.sh` builds the release binary and symlinks it as
`~/.local/bin/conveyor-dev`; `conveyor` on PATH is always the Homebrew release. Never
`cargo install` the crate into a PATH dir. A milestone is done only after the gates pass
**and** `dev-install.sh` has run, so `conveyor-dev` is the committed code.

## Releasing

Releases are GitHub Releases built by `.github/workflows/release.yml` on a `v*` tag: one flat
tarball per target (`conveyor-<target>.tar.gz` holding just the binary; targets
`aarch64-apple-darwin`, `x86_64-apple-darwin`, `x86_64-unknown-linux-gnu`) plus `.sha256`,
with generated notes. Asset names carry no version so `releases/latest/download/` links stay
valid. Both macOS targets build on `macos-latest` (Intel cross-compiled); `macos-13` is
retired and queues forever. The workflow refuses a tag whose version differs from
`Cargo.toml`. Every action is pinned to a commit SHA with the tag in a trailing comment;
bump pins with `gh api repos/<owner>/<repo>/git/ref/tags/<tag>`. Workflows default to
`contents: read`; only `publish` gets `contents: write`, `id-token` and `attestations: write`
for `actions/attest-build-provenance` (`gh attestation verify <tarball> --repo
piacsek/conveyor`). CI also runs `rustsec/audit-check`.

1. Bump `version` in `Cargo.toml` (`Cargo.lock` follows on the next build).
2. Run the gates, commit `Release vX.Y.Z`, merge to `main`.
3. `git tag vX.Y.Z && git push origin vX.Y.Z`, then `gh run watch` until `release` is green
   and `gh release view vX.Y.Z` lists three tarballs.
4. The `tap` job regenerates `Formula/conveyor.rb` in `github.com/piacsek/homebrew-tap`
   (local clone `~/projects/homebrew-tap`) with `scripts/homebrew-formula.sh <version>` and
   pushes with the `TAP_TOKEN` repo secret (fine-grained PAT, contents:write on
   homebrew-tap; set per repo with `gh secret set TAP_TOKEN --repo piacsek/conveyor`). If
   that job fails, run the script by hand and push the tap. Check with `brew audit --strict
   --online --formula piacsek/tap/conveyor` after changing the script.

Installed copies: Homebrew puts the release binary in `$(brew --prefix)/bin/conveyor`;
dev builds are only reachable as `conveyor-dev`. `rustsec/audit-check` v2.0.0 (latest as of
2026-09-10) still targets Node 20 and CI prints a deprecation annotation; bump the pin when a
newer release exists.

Semver: minor for new columns/keys/config, patch for fixes, major on a config-format break.
