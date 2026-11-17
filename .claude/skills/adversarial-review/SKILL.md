---
name: adversarial-review
description: Adversarial review of the conveyor repository — attack the code, the running binary, the release pipeline and the process, report what is bad ranked by impact, and on request file each fix as a GitHub issue labelled `adversarial-review-finding`. Use when asked for an adversarial analysis, "what's bad about this repo", "tear it apart", "don't take it easy", or to turn such findings into issues.
---

# Adversarial review

Two phases. Phase 1 always ends in a report in chat. Phase 2, filing issues, runs only when the user
asks for it, as its own step.

## Ground rules

- **Read-only until phase 2.** No commits, no pushes, no GitHub writes, no calls to clusters. Probes
  live in the scratchpad or as temporary test files that are deleted before the report; end with
  `git status --short` clean and say so.
- **This repository is public** (root `AGENTS.md`). Repros, issues and examples use invented
  `acme/*` data and `<owner>/<repo>`, never a real organisation, login, host or cluster.
- **Read root `AGENTS.md` "Known limits" first.** A known limit is reported only when it shows new
  impact, and the finding says what is new (as `AGENTS.md` allows).
- **Severity by reachability**, as root `AGENTS.md` defines it under "Review before ready": the user
  hits it, the release pipeline hits it, or a pull request reaches code or a secret. Rank the rest
  lower.
- **Evidence or it is not a finding.** Each claim is reproduced (say how) or cites `file:line`.
  Keep "reproduced", "read in the code" and "inferred" apart; never state what was not checked.
- **Be fair.** End the report with what held up, so the rest can be trusted.

## Phase 1: attack

1. **Survey.** `git log --oneline | head -20`, line counts of `src/`, `tests/`, `scripts/`, the
   `AGENTS.md` files and `PLAN.md`; the releases and the open issues (GitHub MCP `list_releases`,
   `list_issues`). A finding already tracked cites its issue instead of being re-reported as new.
2. **Build and test everything.** `cargo build --all-targets` and `cargo build --release`, then
   `cargo test --no-fail-fast -- --include-ignored`. A shallow clone or one without tags fails
   `every_released_tag_keeps_its_entry` and the fragments test: `git fetch --unshallow --tags
   origin`, rerun, and count it as a contributor-experience note, not a code bug.
3. **Read the paths where bugs hide**, in this order:
   - `src/main.rs`: the wiring, and which config is read and which silently dropped.
   - `src/sources/` (`github.rs`, `kube.rs`, `fetch.rs`): deadlines, error mapping, caches, what
     happens when a call never returns.
   - `src/refresh.rs`, `src/worker.rs`, `src/events.rs`, `src/app/run.rs`: what blocks which
     thread, and what runs on the UI thread.
   - `src/app/column.rs`, `src/app/actions/`: row keys, reselect after a refresh, the re-check
     before a write, what is keyed by number alone.
   - `src/config.rs`: what is validated versus merely parsed.
   - `src/ui/`: unchecked `u16` / `usize` arithmetic.
   - `scripts/` and `.github/workflows/`: `pull_request_target`, tokens, what runs from a pull
     request's head, guards that fail open, parsers of tool output.
4. **Attack the running binary** with [`drive-demo.sh`](drive-demo.sh), which runs `--demo` in a
   detached tmux pane of any size, sends keys and captures the screen:

   ```sh
   SCRATCH="$(mktemp -d)"
   : > "$SCRATCH/empty.toml"
   for size in "1 1" "2 2" "3 3" "30 1" "200 3" "300 80"; do
     .claude/skills/adversarial-review/drive-demo.sh target/debug/conveyor $size "$SCRATCH/empty.toml" \
       d S Escape z '?' x c x l l l d L Tab 2 G g
   done
   ```

   - Run both `target/debug` and `target/release`. Debug panics on overflow that release wraps
     silently, so a debug panic points at a release bug.
   - Then hostile configs: zero and huge values for every numeric key, duplicate `[[repo]]` and
     environment names, tables that load but are never read. `CONVEYOR_CONFIG=<file> conveyor
     config` shows what loads; the driver shows what it does.
5. **Probe state logic with throwaway TUI tests.** Write `tests/zz_probe.rs` with `mod support;` and
   drive `support::Harness`:
   - `Harness::with_size`, `prs(...)`, `key(KeyCode::...)`, `h.requests`, `h.screen()`
   - run with `cargo test --test zz_probe -- --nocapture`, then delete the file.

   Worth probing:
   - two rows that share a key, such as the same pull request number in two repositories
   - a refresh landing while the popup or a confirm is open
   - control characters in a title or a log line reaching the buffer
6. **Attack the process.** Look for:
   - claims in the `AGENTS.md` files that the code contradicts
   - rules the code breaks
   - meta-tests that are text heuristics
   - guards that skip or pass on an unreadable input
   - steps that stage or publish more than they checked (`ship.sh`, `scrub-check.sh`)

## Report

In chat, not a file. Offer a shareable page in one line. Sections, most severe first:

1. **The worst:** anything that can leak private data, write to the wrong pull request, or block
   releases.
2. **Bugs reproduced:** each with how it was reproduced, `file:line`, and its impact.
3. **Design limits that will bite real users.**
4. **Process and repository hygiene.**
5. **What held up:** probes that found nothing.
6. **What I'd fix first:** a numbered list, one fix per item.

## Phase 2: file the fixes as issues (only when asked)

- **One issue per "fix first" item.** Split an item whose parts ship separately; fold a sibling
  with the same root cause into its issue. Say which in the final reply.
- **Labels.**
  - Every issue gets `adversarial-review-finding`, plus the existing labels that fit.
  - Check a label with `get_label`. As of 2026-11-17 the labels in use are `bug`, `enhancement`,
    `documentation`, `good first issue`, `help wanted`, `development-cycle-improvement` and
    `Type: Maintenance`.
  - When `adversarial-review-finding` is missing, the first `issue_write` that names it creates it.
    Confirm with `issue_read` `get_labels`, then create the rest in parallel.
- **Body**, following the issue forms (`.github/ISSUE_TEMPLATE/`):
  - `## What happens` for a bug, or `## What you are trying to do` / `## What gets in the way` for
    an enhancement: the repro, pinned to the commit sha.
  - `## Where`: files and functions.
  - `## What would fix it`.
  - `## Tests to start from`.
  - The fragment the fix needs (`kind: fix`, `feat`, `internal`, or none when nothing feeds the
    binary, per `scripts/feeds-binary.sh`).
  - Then `Found during an adversarial review of the repository at <sha>.` and the
    Claude Code footer.
- **No `@name` outside backticks** (write `` `@me` ``): GitHub notifies real logins.
- **Final reply:** a table of number, title and labels, then:
  - the label calls a maintainer may want to undo
  - what was merged or split
  - what was found but not filed
