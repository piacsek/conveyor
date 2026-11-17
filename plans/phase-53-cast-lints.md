# Phase 53: casts that can lose a value fail the gates

## Context
Issue #90: `Cargo.toml` has no `[lints]` table, so clippy runs only its default set. Nothing
forbids `unsafe`, and an `as` cast can cut a value off silently. Nothing a user can see changes:
no terminal is 65535 columns wide.

## Verified facts (2026-11-16, `origin/main` at 8609f88, pinned clippy of `rust-toolchain.toml`)
- `cargo clippy --all-targets -- -W unsafe_code -W clippy::cast_possible_truncation
  -W clippy::cast_sign_loss -W clippy::cast_possible_wrap -W clippy::cast_lossless` reports 16
  warnings on 14 lines under `src/` and 8 on 7 lines under `tests/`; no `unsafe_code`.
- `src/`: `app/navigation.rs` 68, 70, 74, 79 (`u16`/`i32` scroll arithmetic); `text.rs` 89
  (`u64 as i64` in `clock`); `usize as u16` widths and heights in `ui/columns.rs` 314,
  `ui/details.rs` 120, `ui/dialog.rs` 60, 61, 119, 120, `ui/footer.rs` 69, `ui/link.rs` 13,
  `ui/mod.rs` 62.
- `tests/`: `queue.rs` 398, `events.rs` 137 (`u128 as u64`), `changelog.rs` 132,
  `manage/failures.rs` 381 (two), `deployed.rs` 542, `prs/cards.rs` 212, `prs/details.rs` 145.
- `scripts/gates.sh` runs clippy with `-D warnings`, so a lint at `warn` fails the gates.
- The `version` check reads `Cargo.toml` with cargo and rewrites it line by line under
  `[package]`; a `[lints]` table after `[dev-dependencies]` leaves that shape alone.

## Design
1. `text::cells(count: usize) -> u16`, saturating at `u16::MAX`, used at every `usize as u16`.
2. `text::scroll(value: i32) -> u16`, clamping to `0..=u16::MAX`, used by both scroll setters in
   `navigation.rs`; `u16` to `i32` goes through `i32::from`.
3. `clock` adds the offset in `i128`, which holds any `u64` second count without a wrap.
4. The table, `unsafe_code = "forbid"` and the four cast lints at `warn`; the seven test sites
   use `cells` or `try_from`.
5. Fragment `changes/phase-53-cast-lints.md`, `kind: internal`.

## Tasks
1. Red: `cells` saturates above 65535. Green: the helper and the nine width sites.
2. Red: `scroll` clamps a negative and a too large value. Green: the helper in `navigation.rs`.
3. `clock` in `i128` (the red is the `cast_possible_wrap` lint; no `SystemTime` above
   `i64::MAX` seconds can be built, so no test can reach the wrap).
4. The `[lints]` table, the test sites, the fragment and the phase outcome in `PLAN.md`.

## Not in scope
The rest of `clippy::pedantic` (mostly `must_use_candidate`, `unnecessary_wraps`,
`missing_errors_doc`).
