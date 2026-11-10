# Phase 33: check a pull request out from the `S` popup

## Context

Reviewing or fixing a pull request from My PRs means leaving conveyor, finding the clone and
typing `gh pr checkout <N>`. The `S` popup gets a `c` offer that does it.

## Verified facts (gh v2.103.3, 2026-11-10)

- Outside a git repository `gh pr checkout 1 -R <owner>/<repo>` prints
  `failed to run git: fatal: not a git repository (or any of the parent directories): .git`
  and exits 1.
- Inside a clone of **another** repository, `gh pr checkout 1 -R <other>/<repo>` succeeds: it
  fetches `refs/pull/1/head` from the other repository into this clone and switches to it
  (exit 0). `-R` alone is therefore no guard; the clone has to be checked first.
- `gh repo view --json nameWithOwner --jq .nameWithOwner` inside a clone prints its
  `<owner>/<repo>`; it is the call `Github::current_repo` already makes.
- A failed checkout prints git's reason first and `failed to run git: exit status N` last, on
  stderr, and exits 1.
- My PRs is a search across repositories, and `[[repo]]` is optional, so the directory
  conveyor was started in is often not a clone of the selected pull request's repository.
- `c` opens the changelog in Normal mode. The popup's letters avoided every Normal-mode key
  until now.

## Decisions (the user's, 2026-11-10)

- **Where**: an optional `[[repo]] path`, else the directory conveyor was started in.
- **Key**: `c`, although it is a Normal-mode key. Checkout only switches a local branch, and
  it only fires after `S`.

## Design

1. **Config.** `Repo::path: Option<PathBuf>`, left out of `conveyor config` when unset. A
   leading `~` is expanded from `HOME` by `config::expand_home` when the checkout runs.
2. **The seam.** `Github::checkout(repo, number, dir)`: `CliGh` asks `gh repo view` in `dir`
   (the working directory when `None`), refuses when the answer is not `repo`, then runs
   `gh pr checkout <N> -R <repo>` there. The refusal names what to configure.
3. **The offer.** `c  check out locally` in My PRs, always live: whether the directory is a
   clone of the repository is only known by asking `gh`, which the popup must not wait for.
   It acts on the letter, like the other cheap actions. `Choice::Checkout(CheckoutTarget)`,
   `Action::Checkout`, `Request::Checkout`; `App::checking_out` joins the one-write-at-a-time
   rule. `Input::CheckedOut` puts `#N checked out in <dir>` or `#N: <error>` in the footer.
   Nothing is refreshed: a checkout changes nothing on GitHub.
4. **The worker** in `main` expands the path and calls `checkout`.

## Out of scope

- Cloning a repository that has no local clone.
- Worktrees, `--force`, `--detach`, stashing a dirty tree: git's refusal is shown as it is.

## Tasks

1. Config key and `expand_home`.
2. `CliGh::checkout` behind a shim.
3. The offer, the in-flight state and the footer result.
4. The worker and an e2e run through the `gh` shim.
5. Docs, v0.24.0, changelog, outcome in `PLAN.md`.
