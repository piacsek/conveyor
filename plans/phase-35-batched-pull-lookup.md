# Phase 35: one call resolves every build's pull request

## Context
Main builds is the last column to appear on the first load. It lists the runs in one call and then asks GitHub for the pull request of each run's commit, one `gh` call after another, before the column is sent. Measured 2026-11-11 on a private repository with `builds = 30`: 1.7 s for the runs, then 30 lookups of 0.7 to 0.9 s each, 24 s in a row, while the other three columns were up within 3 s. Every `gh` call costs about 0.5 s before GitHub does any work (process start and a new connection), so a design that asks per commit pays that 30 times.

One GraphQL query can ask for all of them: `repository { c0: object(oid: "<sha>") { ... on Commit { associatedPullRequests(first: 1) { nodes { … } } } } c1: … }`.

## Verified facts (checked 2026-11-11 against live GitHub)
- **Speed.** 30 commits in one query: 2.3 to 2.5 s on the private repository, 1.1 s on `cli/cli`. 100 commits (the `per_page` cap, so the most `builds` can ask for): 1.8 to 2.3 s on `cli/cli`, a 12.8 KB query passed as one argv entry. Six REST lookups in parallel took 4.5 s for 30.
- **Cost.** `rateLimit.cost` is 1 for the whole query, against one REST call per commit.
- **Same answer.** Number, title, author login and URL were identical to `repos/{r}/commits/{sha}/pulls` (first entry) for 30 of 30 merge-queue commits on the private repository, and for a rebase-merged commit of this repository (#60).
- **Bots differ.** GraphQL gives `author { login: "dependabot", __typename: "Bot" }` where REST gives `user.login: "dependabot[bot]"` (`cli/cli` #14998). The card must keep reading `dependabot[bot]`.
- **A commit GitHub does not know** is `null` under its alias, exit 0, no `errors` entry. REST answers 422 for the same sha, which `for_sha` treats as an error (no fallback, nothing cached).
- **An oid that is not a commit** (a tree) is `{}` under its alias.
- **A malformed oid fails the whole query** (`Could not coerce value "nothex" to GitObjectID`, exit 1), as does an unknown repository. So only 40-hex shas go into the query. Both callers already hold only such shas: `head_sha` from the runs API and `sha_from_image`.
- **No pull request** is `nodes: []` (4 of 100 on `cli/cli`).
- **The e2e and screenshot `gh` shims dispatch on the query text**, and `*'repository(owner'*` answers with the queue fixture. The batch query is written `repository(name: $name, owner: $owner)` so it falls to the default route, whose fixture has no `c<N>` alias: every sha then takes the REST path and both shims keep working unchanged.
- `FakeGithub::graphql` answers `response` unless a `graphql_routes` needle is in the query; the existing lookup tests answer `Null`, which holds no alias, so they keep exercising the REST chain as it is.

## Design
`Pulls::prefetch(gh, &[(sha, Option<number>)], now)` runs before the per-row `for_sha` calls and fills the cache:

1. Keep the shas that are 40 hex characters and not answered by the cache (unknown, or a miss older than `MISS_RETRY`), once each.
2. None left: no call.
3. One GraphQL call for all of them. An error from `gh` is not an answer: nothing is cached and **no REST call is made for those shas in this fetch** (a rate limit must not turn one call into thirty); the next refresh asks again.
4. Per alias: a pull request is cached as a hit (a `Bot` author gets `[bot]` appended). `nodes: []` goes straight to the existing fallback, `landed(number)` when the run title carried `(#N)` and `by_squash_suffix` otherwise, without the REST association call, and its outcome is cached as today. An alias that is missing, `null` or not a commit is left alone, so `for_sha` asks REST for it exactly as before.

`for_sha` is unchanged apart from skipping shas a failed batch covered in this fetch. `BuildsSource::fetch` prefetches all runs; `DeploySource::fetch` prefetches the shas of the environments that answered, so its up to four lookups are one call too.

No chunking: `per_page` caps a builds page at 100 and 100 was measured.

## Tasks (each starts red)
1. Main builds asks for all its runs' pull requests in one GraphQL call and makes no per-commit REST call for the ones it answers.
2. A bot author reads `name[bot]`, as it did through REST.
3. A commit the batch reports no pull request for goes to the `(#N)` fallback without the REST association call; the miss is cached for the retry window.
4. A failed batch makes no REST lookup and caches nothing; the next fetch asks again.
5. An alias the batch did not answer (missing, `null`, not a commit) and a sha that is not 40 hex take the REST path.
6. Deployed resolves its environments' shas in one call.
7. Docs (`src/sources/AGENTS.md`, root layout line), v0.24.2, `CHANGELOG.md`, outcome in `PLAN.md`.

## Verification
`scripts/gates.sh`. Then `conveyor-dev` against a repository with `builds = 30`: the log shows one `gh api graphql` after the runs call and no `commits/<sha>/pulls` line, and Main builds appears within about 5 s.
