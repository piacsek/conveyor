# Phase 10 — the merge queue shows its build

## Why

A queue card's details pane said `run: b opens it` and nothing else. The queue is exactly
where "is my build passing?" is asked, and answering it meant a browser trip, while the Main
builds column has answered the same question in place since phase 6.

## How

The REST call that already runs for the queue (`actions/runs?event=merge_group`) returns the
same run shape as the builds workflow endpoint, so `parse_merge_group_runs` now builds a real
`Build` through `builds::parse_run` and `attach_runs` hangs it on `QueueEntry::run` instead of
keeping only its URL. The details pane then reuses the Main builds rendering — the run line
plus every job with its failed step — and `App::selected_run` makes the jobs machinery key on
whichever of the two columns is focused. No new call: the run was already being fetched.

## Tasks

1. `QueueEntry::run: Option<Build>`, parsed and attached.
2. The queue details pane renders the run line and its jobs; the card carries the run number.
3. Jobs are fetched for the focused queue entry and forgotten while its run is unsettled.
