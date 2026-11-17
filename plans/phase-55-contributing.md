# Phase 55: a place for early users to ask, report and pick up work (#99)

## Context
The backlog lives only in `PLAN.md`, and a newcomer has nowhere to ask a question, request a
deploy source or find a first task. #99 asks for Discussions, issue templates that warn against
pasting private data, a short `CONTRIBUTING.md`, and at least three labelled starter issues.

## Tasks
1. `.github/CONTRIBUTING.md`, `.github/ISSUE_TEMPLATE/` (bug report, feature request, `config.yml`
   sending questions and deploy-source requests to Discussions), the Layout of `AGENTS.md` and
   `.github/AGENTS.md`.
2. Outside the pull request: Discussions on, starter issues opened from the backlog and
   labelled. The phase outcome in `PLAN.md` names the issue each backlog item became.

## Verified facts (2026-11-16)
- `scripts/feeds-binary.sh` treats every path not on its list as feeding the binary, so a
  top-level `CONTRIBUTING.md` demands a fragment. Adding it to the list does not help this pull
  request: `version.yml` checks out the base and runs the base's `scripts/version-check.sh`,
  which refused a local commit holding both the list change and the file ("CONTRIBUTING.md
  feeds the binary"). GitHub reads `.github/CONTRIBUTING.md` too, and `.github/` is on the
  list, so the file lives there and the script stays as it is (a first commit adding it to the
  list was reverted).
- No source file embeds `CONTRIBUTING.md` (`include_str!` reads only `CHANGELOG.md`, the
  GraphQL queries and the demo fixture).
- `tests/readme.rs` pins the README to five sections; the README is left alone.
- Labels `good first issue` and `help wanted` already exist on the repository; Discussions are
  off.
- Backlog items still open in the code: the footer and dialogs cut text by characters
  (`src/text.rs`), `merge_method` is global (`src/config.rs`, `Prs`), there is no approve
  action (`src/app/actions/`), Main builds has no tabs (only `src/app/stages/queue.rs` does).

## Outcome

Phase 55 (`plans/phase-55-contributing.md`, #109, closes #99): a place for early users.
`.github/CONTRIBUTING.md` points to `AGENTS.md`, `scripts/gates.sh`, `scripts/ship.sh` and
the fragment rule. It lives under `.github/` because a top-level file feeds the binary by
`scripts/feeds-binary.sh`, and listing it there would not have helped: `version.yml` runs the
base's script, which refused it locally. So the pull request needs no fragment and releases
nothing. `.github/ISSUE_TEMPLATE/` holds a bug report
and a feature request form, each opening with a reminder not to paste private names, and a
`config.yml` with blank issues off that sends questions to Q&A and deploy-source requests to
the Ideas category of Discussions. Outside the pull request: Discussions turned on (Q&A is a
default category; categories cannot be created through the API, so a dedicated deploy-sources
category is left to the owner); `good first issue` and `help wanted` already existed. Backlog
items opened as starter issues: #111 (wide characters, `good first issue`), #112
(`merge_method` per `[[repo]]`, `good first issue`), #113 (approve, `help wanted`), #114
(Main builds tabs, `help wanted`).

## Hurdles

- `ci` A top-level `CONTRIBUTING.md` feeds the binary by path, and listing it would not help since `version.yml` runs the base's script; it moved under `.github/` → code
- `tooling` Discussion categories cannot be created through the API; a deploy-sources category is left to the owner → none
