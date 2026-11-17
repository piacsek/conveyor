# Phase 63: the flake build runs only when its inputs change, and gates the merge

## Context

Issue #117. The `flake` job of `ci.yml` builds the Nix flake on Linux and macOS for every pull
request and every push to `main`, whatever they change, and it is not required to merge. A pull
request that breaks the flake can merge with a red `flake`; a Linux break is caught after the
merge by `stable-build`, a macOS break by nothing.

This phase makes the build run only when one of the flake's inputs changed, and adds one gate
job, `flake-ok`, that the `main` ruleset can require. Adding `flake-ok` to the ruleset is the
owner's step by hand once this is on `main`; this phase changes no repository setting.

## Verified facts (2026-11-17, `main` at 6512665)

- `flake.nix` reads `./Cargo.toml` (`readFile`, the fileset), `./Cargo.lock` (the fileset,
  `cargoLock.lockFile`), `./CHANGELOG.md` and `./src` (the fileset), `./rust-toolchain.toml`
  (`fromRustupToolchainFile`), and `./.` as the fileset's root.
- A required check whose workflow never starts stays pending, so no workflow-level `paths:`.
  A skipped matrix job reports under its unexpanded name, so the legs cannot be required
  directly either (#117).
- `tests/scripts/flake_workflow.rs` pins the text of the `flake` job.
- `scripts/script-tests-skippable.sh` must list every path the script tests read; the new
  test reads `flake.nix`, which it does not list.
- On `pull_request` the checkout is GitHub's merge commit, whose first parent is the base
  branch's tip, so `git diff HEAD^1 HEAD` is what the pull request changes against the `main` it
  would merge into. On a push, `github.event.before` is the previous tip.

## Decisions

- The list of inputs lives in one script, `scripts/flake-inputs.sh`, a filter like
  `scripts/feeds-binary.sh`: paths on stdin, the inputs on stdout. A test feeds it every path
  `flake.nix` names, so a path the flake starts reading cannot be left off.
- The list also holds the workflow file and the three flake scripts: a change to what decides
  the gate is exercised by running it.
- `scripts/flake-build-needed.sh <base>` prints `build=true` or `build=false` for
  `$GITHUB_OUTPUT`. Anything it cannot decide (an unknown base, a shallow history, a failing
  `git`) prints `build=true`: a failure to decide never skips the build.
- `scripts/flake-ok.sh <paths result> <build> <flake result>` is the gate's verdict, so every
  state of the table below has a test.
- The `flake` job runs on `!cancelled() && needs.paths.outputs.build != 'false'`, so a failed
  `paths` job, which outputs nothing, still builds.

## States

| State | `paths` | `build` | `flake` | `flake-ok` |
| --- | --- | --- | --- | --- |
| Inputs changed, both legs pass | success | `true` | success | passes |
| Inputs changed, one leg fails | success | `true` | failure | fails |
| Inputs changed, one leg cancelled | success | `true` | cancelled | fails |
| Inputs unchanged | success | `false` | skipped | passes |
| `paths` fails, the build runs and passes | failure | empty | success | passes |
| `paths` fails, the build runs and fails | failure | empty | failure | fails |
| The run is cancelled before the build | cancelled | empty | skipped | fails |
| `flake` skipped although `build` is not `false` | success | `true` | skipped | fails |
| A push to `main` | as above, the base is the previous tip of `main` | | | as above |

## Tasks

1. `scripts/flake-inputs.sh` and its test against `flake.nix`; `flake.nix` joins the paths
   `scripts/script-tests-skippable.sh` lists.
2. `scripts/flake-build-needed.sh`: `build=true` when an input changed or nothing can be
   decided, `build=false` otherwise.
3. `scripts/flake-ok.sh`: the verdict of the table above.
4. `ci.yml`: the `paths` job, the `flake` job behind it, the `flake-ok` gate (red first in
   `tests/scripts/flake_workflow.rs`); `.github/AGENTS.md` and the root `AGENTS.md` (layout,
   Known limits).

## Outcome

## Hurdles
