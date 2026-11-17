# Phase 31: a kubectl that hangs is cut off

## Context

The Deployed column sat on `fetching…` for 40 s, twice in one session, when the cluster proxy of
one environment stopped answering. The log shows both calls:

```
kubectl --context <context> -n <ns> get pods … --request-timeout=10s 40068ms exit 1: E1109 … memcache.go:265] "Unhandled Error" err="couldn't get current server API group list: …" | … | Unable to connect to the server: context deadline exceeded
kubectl --context <context> -n <ns> get deploy … --request-timeout=10s 40039ms exit 1: …
```

`AGENTS.md` says the 10 s timeout returns the row to an error instead of hanging. It does not
when the server is unreachable.

## Verified facts

- `--request-timeout=10s` bounds one HTTP request, not the command. kubectl retries API
  discovery: the stderr above carries three failed attempts 10 s apart before the final error,
  and the call took 40 s (session log, 2026-11-09, kubectl as installed that day).
- `CliKubectl::run` uses `Command::output()`, which waits for the process and for both pipes
  to close, with no limit of its own.
- The error a row shows is the whole stderr. Its first line is kubectl's klog noise
  (`E1109 20:36:29.585690   14272 memcache.go:265] "Unhandled Error" err="…"`), so the card,
  truncated to its width, never reaches `Unable to connect to the server`.
- `Log::call` writes `did not start: <err>` for any `Err`, which would be wrong for a call
  that started and was killed.

## Design

1. **A deadline on the process.** `CliKubectl::run` spawns kubectl with piped stdout and
   stderr, drains both on threads, polls `try_wait` and kills the child at `DEADLINE` (15 s:
   one full request timeout plus room for a slow but answering proxy). The error is
   `kubectl did not answer within 15s` (`io::ErrorKind::TimedOut`), and the log line ends in
   `timed out` (plus the stderr kubectl had printed) instead of `did not start`.
   `with_deadline` lets a test use a short one. No new dependency.
   kubectl runs in its own process group and the kill goes to the group, so a helper it
   started dies with it (added after review: killing kubectl alone left the helper, two reader
   threads and two pipes behind on every timed-out call).
2. **The pipes are bounded too.** A helper kubectl started (a login plugin) can outlive it and
   keep the pipes open, which is what `output()` waits for. Once kubectl has exited, the
   output gets `DRAIN` (500 ms) to arrive; then the group is killed, which closes the pipes,
   and kubectl's own output and exit status are used. (Changed after review: the first cut
   waited until the deadline and reported a timeout, losing kubectl's answer.) Only a helper
   that escaped the group leaves the call a timeout.
3. **The row says why.** klog lines (`E<mmdd> <time> <pid> <file>.go:<line>] …`) are dropped
   from the error a row shows when anything else is left, so the card reads
   `Unable to connect to the server: …`. The log keeps every line.
4. `--request-timeout=10s` stays: it is what makes a slow single request fail with a real
   message before the deadline has to kill anything.

## Out of scope

- Fetching the environments in parallel. One dead environment now delays the column by at most
  15 s per call instead of 40 s; parallel fetches are a larger change to `DeploySource`.
- The My PRs query taking 5 to 7 s. Slow, never failed.

## Tasks

1. Deadline on the process and on the pipes, logged as `timed out`.
2. klog lines dropped from the row's error.
3. `AGENTS.md`, `PLAN.md`, v0.23.1 and its changelog entry.

## Outcome

v0.23.1 (phase 31, `plans/phase-31-kubectl-deadline.md`): a kubectl that hangs is cut off.
Found in the session log: with one environment's proxy not answering, `get deploy` and
`get pods` each took 40 s under `--request-timeout=10s`, because the flag bounds one request
and kubectl retries discovery. `CliKubectl::run` now kills the process at 15 s with every
helper it started (its process group), gives an exited kubectl's pipes 500 ms before killing
what holds them and using kubectl's own answer, logs
`timed out` with what kubectl had printed, and a failed row shows
kubectl's own reason instead of the klog line that came first on stderr. Retro: the phase 4
note "the 10 s timeout returns the row to an error" was never tested against an unreachable
server; the call log added in phase 28 is what showed it. The first cut killed only kubectl; review
measured one orphan helper, two threads and two pipes left per timed-out call, hence the
process group; the second called an exited kubectl whose helper held the pipes a timeout and
lost its answer. Not done: fetching the environments in parallel (one dead environment still
delays the others by about 15 s, 30 s when only its pods call hangs).
