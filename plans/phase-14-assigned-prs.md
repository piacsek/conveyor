# My PRs: authored OR assigned, deduplicated

## Context
My PRs runs one GraphQL `search(type: ISSUE)` with the default query
`is:pr is:open author:@me archived:false`. PRs assigned to the user but authored by someone
else never show. Goal: the default shows authored ∪ assigned, each PR once.

## Verified facts (live, 2026-11-01, read-only `gh api graphql`)
- `type: ISSUE` with `(author:@me OR assignee:@me)` returns **0** (no boolean support).
- `type: ISSUE_ADVANCED` with the same query returns 14 nodes, 14 unique `repo#number`:
  GitHub deduplicates server-side. Author alone = 11, assignee alone = 14.
- `ISSUE_ADVANCED` with the old plain query returns 11, same as `ISSUE`:
  existing custom `prs.query` values keep working.

## Approach: server-side OR, no client merge
Two queries + a client-side merge would double calls and need ordering/limit rules.
Advanced search does it in one call, deduped, with no new Rust code.

## Tasks (TDD, `/tdd autonomous`, branch `phase-14-assigned-prs`)
1. **Search type.** Red: change the assertion in `tests/fetch.rs:100` to expect
   `type: ISSUE_ADVANCED`. Green: edit `src/sources/queries/prs.graphql`.
2. **Default query.** Red: `tests/cli.rs:55` expects
   `is:pr is:open archived:false (author:@me OR assignee:@me)`. Green: `Prs::default` in
   `src/config.rs:27`.
3. **Docs.** README `[prs]` default (`README.md:57`), AGENTS.md data-source line
   (`AGENTS.md:64`: `type: ISSUE_ADVANCED`, OR/parentheses need it, dedup is GitHub's).
4. **Release** v0.14.0 (observable default change), per AGENTS.md Releasing.

## Caveats
- Users with an explicit `prs.query` in their config keep their own query; they must add
  `(author:@me OR assignee:@me)` themselves.
- The `e2e`/screenshot `gh` shims match on query text; check they don't key on `type: ISSUE`
  (grep showed only `tests/fetch.rs`).
- Card shows author already (`author { login }`), so assigned PRs by others are
  distinguishable; no UI change planned.

## Verification
- `scripts/gates.sh`
- `conveyor-dev config` prints the new default query.
- `conveyor-dev`: My PRs shows the count matching the advanced-search probe (14), no duplicates.
