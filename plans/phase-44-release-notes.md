# Phase 44: each release lists only its own changes

## Context
The release page of v0.30.1 lists every pull request since v0.24.1, and each release since
v0.24.2 does the same with a longer list. Decided with the user on 2026-11-14: keep GitHub's
generated notes (pull request titles with author and link, as lazygit has them) and tell GitHub
which tag came before.

## Cause
Since v0.24.2 a tag sits on a release commit that is reachable only from that tag, so no
release tag is an ancestor of the next one. `generate_release_notes: true` in the `publish` job
of `.github/workflows/release.yml` lets GitHub pick the previous tag, and it picks the nearest
ancestor tag: always v0.24.1, the last tag made on `main`.

## Verified facts (2026-11-14)
- `git merge-base --is-ancestor v0.30.0 v0.30.1` fails; of v0.24.1, v0.24.2, v0.30.0 and
  v0.30.1 only v0.24.1 is an ancestor of `origin/main`.
- `softprops/action-gh-release` at the pinned commit `3bb12739` has a `previous_tag` input
  (its `action.yml`).
- `gh api repos/<owner>/<repo>/releases/generate-notes -f tag_name=v0.30.1 -f previous_tag_name=v0.30.0`
  answers two pull requests (#87, #88) and `compare/v0.30.0...v0.30.1`. The same call for
  v0.24.2 against v0.24.1 answers three (#61, #63, #64). So an explicit previous tag works
  although the tags are not ancestors of each other.
- `gh api 'repos/<owner>/<repo>/tags?per_page=100' --jq '.[].name'` lists 50 tags today, one
  name per line, all `vX.Y.Z`.
- The `publish` job checks out only `scripts/` at the tag (sparse) and has `GH_TOKEN` in the
  steps that call `gh`.

## Design
A new script, `scripts/previous-tag.sh <tag>`, prints `previous_tag=<tag>` for
`$GITHUB_OUTPUT`: the highest `vX.Y.Z` tag of the repository below `<tag>`, in version order.
It reads the tag names with `gh api --paginate` (the `publish` job has no tags checked out).
The `publish` job runs it when the tag is still unfinished and passes the answer to the release
action as `previous_tag`.

| State of the repository's tags | The script prints |
| --- | --- |
| lower release tags exist | `previous_tag=` the highest of them |
| `v0.9.0` and `v0.10.0` both below the tag | `v0.10.0` (version order, not text order) |
| higher release tags exist too | they are ignored |
| the tag is the lowest release tag | `previous_tag=` with nothing after it; GitHub then picks as today |
| names that are not `vX.Y.Z` (`v1-test`, `nightly`) | they are ignored |
| the tag itself is missing from the list | still the highest below it |
| `gh` fails | exit 1 with a message naming the tag, before any release is created; a rerun resumes |

A pull request that feeds nothing into the binary (docs, CI) still shows in the notes of the
next release, because GitHub lists every pull request merged between the two tags. Accepted:
the user chose pull request titles over the fragment bullets.

## Tasks
1. `scripts/previous-tag.sh` with its tests in `tests/scripts/previous_tag.rs` (`gh` as
   `tests/fixtures/reply-call.sh`), one test per row of the table.
2. The `publish` job calls it and passes `previous_tag`; a test reads `release.yml` and pins
   both lines. `scripts/script-tests-skippable.sh` already lists `scripts/` and `.github/`.
3. Guidelines: the root `AGENTS.md` layout line for the script, the release run in
   `.github/AGENTS.md` (why the previous tag is named), the phase outcome in `PLAN.md`.

No fragment: nothing here feeds the binary, so the phase releases nothing. The first release
after it shows the result.

## After the merge, by hand and only on the user's yes
Rewrite the notes of v0.24.2 to v0.30.1 (eight releases) with the body `generate-notes` answers
for each tag against the one below it. The old bodies are saved first.
