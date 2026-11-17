# Phase 52: the Nix flake builds with the pinned Rust

## Context
Phase 51 shipped a Nix flake that builds with the `rustc` of its locked nixpkgs (1.99.1), while
`rust-toolchain.toml` pins 1.98.1 for the gates and CI. A Nix user ran a binary built by a
compiler no gate ever ran. The flake now reads the toolchain from `rust-toolchain.toml`, the one
place the version is written.

## Verified facts (2026-11-16)
- `oxalica/rust-overlay` ships the Rust release manifests with their hashes, so
  `rust-bin.fromRustupToolchainFile ./rust-toolchain.toml` needs no hash of ours. fenix's
  `fromToolchainFile` needs a `sha256` that changes on every bump; nixpkgs alone carries one
  `rustc` per revision and cannot pick a version.
- `rust-overlay.lib.mkRustBin { } pkgs` gives `rust-bin` without importing nixpkgs again with
  an overlay, so `forEachSystem` keeps `nixpkgs.legacyPackages`.
- Locked at `rust-overlay` 2ff7d39 (2026-11-16), following the flake's `nixpkgs`. In Docker
  (`nixos/nix`, `aarch64-linux`): the toolchain's `version` evaluates to `1.98.1`, its `rustc
  --version` is `rustc 1.98.1 (48a229cea 2026-09-01)`, and the built binary embeds
  `rustc/48a229ceaefd4985c50990b14116b6d856af0985`, the same commit hash. `nix flake check
  --all-systems` passes for `x86_64-linux`, `aarch64-linux` and `aarch64-darwin`.
- nixpkgs' own `rustc` 1.99.1 still appears in the build's derivation closure: it builds
  `cargo-auditable`, a build tool of `buildRustPackage`, not conveyor.

## Decisions
- The toolchain fetches the components the file lists (rustfmt and clippy): one definition,
  read as rustup reads it, and the two are small prebuilt tarballs.
- `rust-toolchain.toml` asks for the `minimal` profile. Without it the flake took rustup's
  default profile, `rust-docs` included, and the macOS `flake` job spent 1 to 2 minutes
  unpacking it (6 min 19 s once, against 3 min 9 s before the phase). CI's
  `dtolnay/rust-toolchain` already installs `minimal`, so the gates lose nothing; a local
  rustup no longer installs the docs for this toolchain.
- No cache on the `flake` job unless fetching the toolchain more than doubles its time.
- The other copies of the version stay (`toolchain:` and `components:` in `ci.yml` and
  `release.yml`, `rust-version` in `Cargo.toml`); `tests/toolchain.rs` already guards them.

## Tasks
1. `tests/toolchain.rs`: `flake.nix` reads `rust-toolchain.toml` through
   `fromRustupToolchainFile` and names no `major.minor` of its own (red first), then the flake
   and its lock, the guidelines and the fragment (`kind: internal`).
2. `tests/toolchain.rs`: the file asks for the `minimal` profile (red first), then the file.
3. Measure the `flake` job on the pull request against phase 51's times and write them down.

## Review round 1
- should: every toolchain bump breaks the flake unless the overlay lock moves too. The bump
  steps in both `AGENTS.md` files now list the lock update (a Docker command, no Nix on the
  host), and `scripts/version-check.sh` refuses a pull request that changes the `channel` of
  `rust-toolchain.toml` while `flake.lock` keeps the locked `rust-overlay` revision or holds
  none (no lock, not JSON, no `rust-overlay` input, not 40 hex digits). It reads both files
  with `git cat-file` only; a pull request that keeps the channel is never judged by it.
  Tests in `tests/scripts/version_check_toolchain.rs`.
- minor: the recovery text says a toolchain download failure in `nix build` takes a new run.
- minor: `tests/toolchain.rs` refuses any Rust version literal in `flake.nix` and matches
  `fromRustupToolchainFile ./rust-toolchain.toml` without the `rust-bin.` binding.

## Outcome

Phase 52 (`plans/phase-52-flake-pinned-rust.md`, #110): the Nix flake builds with the Rust
`rust-toolchain.toml` names (1.98.1), not nixpkgs' (1.99.1), through
`rust-overlay.lib.mkRustBin` and `fromRustupToolchainFile`, so the version is still written
in one file and a bump needs no hash. Proven in Docker: the toolchain's `rustc --version` is
1.98.1 and the binary embeds its commit hash. `tests/toolchain.rs` fails while `flake.nix`
stops reading the file or names the channel. A bump whose release the locked overlay does not
know fails the `flake` job; `nix flake update rust-overlay` fixes it. `rust-toolchain.toml`
now asks for the `minimal` profile: the default one brought `rust-docs`, and the macOS
`flake` job took 3 min 33 s and 6 min 19 s; with `minimal` it takes 3 min 10 s and Linux
1 min 33 s (phase 51: 3 min 9 s and 1 min 49 s), so no cache. The toolchain still brings
rustfmt and clippy, as the file lists them. Fragment `kind: internal`. Review round 1: every
bump now updates the locked `rust-overlay` too (Docker command in the bump steps), and the
`version` check refuses a channel change that keeps the locked revision or has none;
`flake.nix` may name no Rust version at all. Review round 2 (unprimed): no must or should;
minors fixed: the check compares the channel without its quotes or a trailing comment,
`flake.nix` may not use `pkgs.rustPlatform`, and only a `1.`-prefixed number counts as a Rust
version there.

## Hurdles

- `review-loop` Every toolchain bump would break the flake unless the locked `rust-overlay` moved too → rule in `AGENTS.md`
- `ci` The default rustup profile brought `rust-docs` and doubled the macOS `flake` job; the file asks for `minimal` → code
