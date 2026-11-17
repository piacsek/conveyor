# Phase 55: a place for early users to ask, report and pick up work (#99)

## Context
The backlog lives only in `PLAN.md`, and a newcomer has nowhere to ask a question, request a
deploy source or find a first task. #99 asks for Discussions, issue templates that warn against
pasting private data, a short `CONTRIBUTING.md`, and at least three labelled starter issues.

## Tasks
1. `scripts/feeds-binary.sh` lists `CONTRIBUTING.md` as not feeding the binary (a
   `tests/scripts/feeds_binary.rs` case, red first), so a docs file needs no fragment and
   releases nothing. `AGENTS.md` names it where it spells the list out.
2. `CONTRIBUTING.md`, `.github/ISSUE_TEMPLATE/` (bug report, feature request, `config.yml`
   sending questions and deploy-source requests to Discussions), the Layout of `AGENTS.md` and
   `.github/AGENTS.md`.
3. Outside the pull request: Discussions on, starter issues opened from the backlog and
   labelled. The phase outcome in `PLAN.md` names the issue each backlog item became.

## Verified facts (2026-11-16)
- `scripts/feeds-binary.sh` treats every path not on its list as feeding the binary, so a new
  top-level `CONTRIBUTING.md` would demand a fragment from `scripts/version-check.sh` (which
  pipes the changed paths through it) and take `conveyor-dev` over in `ship.sh`. `.github/`
  is already on the list, so the issue templates feed nothing.
- No source file embeds `CONTRIBUTING.md` (`include_str!` reads only `CHANGELOG.md`, the
  GraphQL queries and the demo fixture).
- `tests/readme.rs` pins the README to five sections; the README is left alone.
- Labels `good first issue` and `help wanted` already exist on the repository; Discussions are
  off.
- Backlog items still open in the code: the footer and dialogs cut text by characters
  (`src/text.rs`), `merge_method` is global (`src/config.rs`, `Prs`), there is no approve
  action (`src/app/actions/`), Main builds has no tabs (only `src/app/stages/queue.rs` does).
