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
- The toolchain fetches the components the file lists (rustfmt and clippy) rather than a
  minimal profile: one definition, read as rustup reads it, and the two components are a few
  prebuilt tarballs. A minimal profile would need the channel read out of the file by hand.
- No cache on the `flake` job unless fetching the toolchain more than doubles its time.
- The other copies of the version stay (`toolchain:` and `components:` in `ci.yml` and
  `release.yml`, `rust-version` in `Cargo.toml`); `tests/toolchain.rs` already guards them.

## Tasks
1. `tests/toolchain.rs`: `flake.nix` reads `rust-toolchain.toml` through
   `fromRustupToolchainFile` and names no `major.minor` of its own (red first), then the flake
   and its lock, the guidelines and the fragment (`kind: internal`).
2. Measure the `flake` job on the pull request against phase 51's times and write them down.
