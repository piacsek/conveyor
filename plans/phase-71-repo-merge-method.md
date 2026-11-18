# Phase 71: each repository merges with its own merge method

## Context
Issue #112. The merge method (`squash`, `merge` or `rebase`) is one global setting,
`[prs] merge_method`. Someone whose repositories allow different methods (one squash only,
another merge commits only) cannot merge or enable auto-merge in both from conveyor: GitHub
refuses whichever method the repository does not allow. Phases 70, 72 and 73 run in parallel,
so this is 71.

## Verified facts (2026-11-17, at `5cae050`)
- `config::Prs::merge_method` is a `MergeMethod` (`squash` default, `merge`, `rebase`,
  lowercase in TOML); an unknown value is a serde error naming the file and the value
  (`tests/config.rs`, `an_unknown_merge_method_fails_naming_the_file`).
- `config::Repo` is `deny_unknown_fields`; `branch` is an `Option<String>` that `conveyor
  config` leaves out while unset (toml cannot write `None`), and prints when set
  (`tests/cli.rs`).
- The merge worker in `src/main.rs` reads `config.prs.merge_method` once and passes it to
  `CliGh::merge`, which runs `gh pr merge <N> -R <repo> --<method> --match-head-commit <sha>`.
  `MergeTarget` carries number, repo, title and head; no method.
- Enabling auto-merge (`a`) builds `StatusChange::EnableAutoMerge { method:
  context.config.prs.merge_method, .. }` in `src/app/stages/prs.rs`; `CliGh` sends it as
  `mergeMethod` of `enablePullRequestAutoMerge`.
- `m` on a merge-queue base sends `StatusChange::Enqueue`, `enqueuePullRequest`, which takes no
  method: the queue's own settings decide it.
- A pull request's `repo` is GitHub's `owner/name`; `[[repo]] name` is matched against it with
  ASCII case ignored elsewhere (`checkout_target`).
- Two `[[repo]]` of one name load when their `branch` or `main_workflow` differ (phase 66), so a
  repository can appear twice; `src/config/values.rs` refuses a setting conveyor would ignore.
- `y` re-derives a merge through `App::reconfirm`, comparing the whole `Choice`; the config does
  not change while conveyor runs, so a method on the target compares equal.
- `gh` 2.104.0 locally.

## Design
- `[[repo]] merge_method`, optional (`Option<MergeMethod>`), printed by `conveyor config` only
  when set, like `branch`. `Config::merge_method(repo)` answers the method of the first
  `[[repo]]` whose name matches the pull request's repository (ASCII case ignored) and sets
  one, else `[prs] merge_method`.
- Two `[[repo]]` of one name that set different methods are refused at load: one of them would
  never be read. One that sets it and one that leaves it out is fine: the set one applies.
- The method is read when the menu is built and travels on the target (`MergeTarget::method`,
  as `UpdateTarget::method` does), so the merge worker no longer reads the config.

| State | `m` (no merge queue) | `a` enable auto-merge | `m` on a merge-queue base |
| --- | --- | --- | --- |
| the pull request's repository is a `[[repo]]` with `merge_method` | `gh pr merge --<its method>` | `enablePullRequestAutoMerge` with its method | `enqueuePullRequest`, no method (unchanged) |
| a `[[repo]]` without `merge_method`, or no `[[repo]]` for it | `--<[prs] merge_method>` | `[prs] merge_method` | unchanged |
| same name in another case (`Acme/Webapp`) | its method | its method | unchanged |
| two `[[repo]]` of that name, one sets it | the set one | the set one | unchanged |
| two `[[repo]]` of that name set different methods | refused at load, naming the repository | same | same |
| an unknown value (`merge_method = "fast"`) | refused at load, naming the file and the value | same | same |

## Tasks
1. A `[[repo]]` may set `merge_method`: it loads, `conveyor config` prints it (and leaves it out
   while unset), an unknown value and two same-named repositories with different methods are
   refused; README's default config lists it. Plan, index and metrics rows, the fragment.
2. `m` merges with the method of the pull request's repository, else `[prs]`.
3. Enabling auto-merge uses the same method; `src/app/AGENTS.md` names it.

## Outcome

- `[[repo]]` takes an optional `merge_method`, printed by `conveyor config` only when set and listed in the README's default config. `Config::merge_method(repo)` answers the first `[[repo]]` of that name (ASCII case ignored) that sets one, else `[prs] merge_method`.
- Two `[[repo]]` of the same name that set different methods are refused at load, naming the repository; one set and one unset load.
- The method travels on `MergeTarget::method`, read when the popup is built like `UpdateTarget::method`, so the merge worker no longer reads the config. Enabling auto-merge reads the same method; enqueueing is unchanged.
- The merge e2e tests moved to `tests/e2e/merge.rs` to keep `tests/e2e/main.rs` under 700 lines, and gained one for a repository that rebases.
- Review round 1 was clean. Its minor findings were fixed after it: two tests for the case-insensitive name match and for skipping an entry that sets no method, each shown to fail on its mutation; the README comment aligned; the duplicate error reworded to say only one of them can be used.
- Left open: neither the merge confirm nor the enable auto-merge confirm names the method, so the user cannot see before `y` whether `m` squashes or rebases. Showing it is a visible change the owner has not decided.

## Hurdles

- `test-seam` two rows of the states table (name case, an entry that sets no method) had no test; the reviewer's mutations of `Config::merge_method` stayed green until tests were added → code
