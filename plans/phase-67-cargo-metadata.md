# Phase 67: the release scripts read `cargo metadata` with a JSON parser

## Context

Issue #126. `root_package` in `scripts/release-rewrite.sh`, sourced by
`scripts/release-commit.sh` and `scripts/version-check.sh`, reads the output of
`cargo metadata --no-deps --offline --format-version 1` with about 45 lines of awk that walk the
JSON one character at a time. It reads only output on one line, starting with
`{"packages":[`, each package starting with `"name"`, `"version"` and `"id"` in that order, and
the array followed by `],"workspace_members":[`. None of that is part of cargo's format
promise. When cargo changes any of it, the `version` check refuses every pull request that
feeds the binary and `release-commit.sh` refuses every release. The `tag` job runs on the
runner image's `stable` on purpose, so the release side moves with every Rust release.

This is the mechanism the "two strikes, then redesign" rule points at: the redesign asks a real
JSON parser for exactly the fields needed instead of tokenizing more carefully.

## Verified facts (2026-11-17, `main` at 5dd801f)

- `jq` is already a dependency of `scripts/version-check.sh`: `overlay_at` reads `flake.lock`
  with it, and the check refuses a channel change when `jq` is not on `PATH`
  (`tests/scripts/version_check_toolchain.rs`,
  `a_machine_without_jq_refuses_a_new_channel_instead_of_passing_it`).
- Where each script runs, and its `jq`:
  - the `version` job of `version.yml`: `ubuntu-latest`, which carries `jq` (already relied
    on, see `.github/AGENTS.md`);
  - the `tag` job of `release.yml`: `ubuntu-latest` on the image's `stable`, the same image,
    so `jq` is there;
  - a developer's macOS: `/usr/bin/jq` (`jq-1.7.1-apple`) ships with the system here; the
    tests that put only `/usr/bin:/bin` on `PATH` still find it;
  - the `check` job of `ci.yml` runs the script tests on `ubuntu-latest`, `jq` included.
- `cargo metadata` of this repository, with the pinned 1.98.1, is one line starting
  `{"packages":[{"name":"conveyor","version":"0.0.0","id":"path+file://…`.
- Under `cargo test`, rustup exports `RUSTUP_TOOLCHAIN` (here `1.98.1-aarch64-apple-darwin`,
  source `toolchain-file`) to the test processes, and `version-check.sh` passes the caller's
  `RUSTUP_TOOLCHAIN` to the cargo it runs. So the script tests that run the real cargo
  (`the_manifest_of_this_repository_passes`, every passing `version_check*` test, the
  `release_commit_cargo.rs` tests, `a_checkout_whose_path_holds_a_space_is_confirmed_by_cargo`)
  feed `root_package` the real output of the pinned toolchain. That is how this change is
  proven: the `version` check on this pull request runs the copy on `main`.
- Tests that pin what `root_package` decides today:
  - `tests/scripts/version_check_head.rs`: `a_virtual_manifest_is_refused_since_conveyor_must_be_the_root_package`
    (no package at the root), `a_root_package_named_otherwise_is_refused`,
    `the_manifest_of_this_repository_passes`;
  - `tests/scripts/version_check_cargo.rs`: `a_package_version_other_than_0_0_0_is_refused_by_file`,
    `a_decoy_version_in_a_multi_line_string_does_not_hide_the_version_cargo_reads`,
    `a_scratch_directory_whose_path_holds_a_space_still_finds_the_root_package`;
  - `tests/scripts/version_check_release.rs`: the rehearsal, which compares `root_package` with
    `conveyor <probe>`;
  - `tests/scripts/release_commit_cargo.rs`: `a_version_cargo_does_not_read_after_the_rewrite_is_refused_leaving_the_tree_as_it_was`,
    `a_checkout_whose_path_holds_a_space_is_confirmed_by_cargo`,
    `without_cargo_on_path_the_release_is_written_from_the_rewrite_alone`.
  - No test pins the wording of the "shape the check does not know" refusal or the
    `unreadable` answer itself.

## What must not change (the security property)

- Nothing of the pull request head runs. `jq` reads cargo's output as data with a filter fixed
  in the script; the head chooses neither the filter nor the program.
- cargo is run exactly as before (the copy, `env -i`, the empty `cwd`), so the check is pointed
  at manifests outside its copy no more than the Known limits already accept.
- The root package is still the one whose `manifest_path` is the copy's own `Cargo.toml`, and
  the answers stay the same: one root package gives `<name> <version>`, none gives an empty
  line (the virtual manifest refusal), anything else gives `unreadable`.

## States

| `cargo metadata` output | `root_package` prints |
| --- | --- |
| one JSON document, exactly one package whose `manifest_path` is `<root>/Cargo.toml`, `name` and `version` strings | `<name> <version>` |
| the same, keys in any order, pretty-printed over several lines, extra whitespace | `<name> <version>` |
| a workspace: several packages, one of them at the root | the root one's `<name> <version>` |
| a `manifest_path` holding `"` or `\` (escaped in JSON) | matched after decoding, `<name> <version>` |
| no package at the root (a virtual manifest) | empty |
| two packages at the root | `unreadable` |
| the root package's `name` or `version` not a string | `unreadable` |
| not JSON, empty, two documents, no `packages` array | `unreadable` |

`jq` missing: `version-check.sh` refuses up front, as it does without `cargo`, naming `jq`;
`release-commit.sh`, with `cargo` on `PATH`, refuses before writing anything rather than
mistaking it for a version cargo does not read. Without `cargo` it keeps skipping the
confirmation, as today.

## Decisions

- `root_package` becomes one `jq -rs` call. `--slurp` turns several documents into an array of
  more than one, so two documents are refused instead of printing two answers.
- `version-check.sh` checks `jq` up front next to `cargo`, since every run now needs it. The
  per-channel `jq` check becomes unreachable and goes; its test changes to the new wording
  (the one pinned message this phase changes, because the refusal now covers every run, not
  only a channel change).
- The `unreadable` refusal of `version-check.sh` no longer says "starts the way cargo writes
  it"; it says the output is not JSON with at most one package at the root. No test pinned
  the old wording.

## Tasks

1. `root_package` reads the output with `jq`: tests of the function in a new
   `tests/scripts/release_rewrite.rs` for every row of the table, red against the awk first.
2. `version-check.sh` refuses up front without `jq`, the channel check's own `jq` refusal goes.
3. `release-commit.sh` with `cargo` but without `jq` refuses before writing.
4. `.github/AGENTS.md` and the root `AGENTS.md` say `jq` reads the output.

## Outcome

## Hurdles
