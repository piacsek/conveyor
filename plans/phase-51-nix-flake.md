# Phase 51: install conveyor with Nix

## Context
Issue #96, "Make installing and trying conveyor easier", is done when at least one install
channel besides Homebrew is published from the release pipeline. Phase 49 (`--demo`) and phase
50 (plain icons) cover the rest of it. The user chose a Nix flake: the name `conveyor` is taken
on crates.io, and the AUR needs an account and a key.

## Verified facts (2026-11-16)
- `nix eval --raw github:NixOS/nixpkgs/nixos-unstable#rustc.version` answers `1.99.1`
  (`nixpkgs-unstable` too); `nixos-25.11` answers `1.91.1` and `nixos-26.05` `1.95.0`. The crate needs 1.98
  (`rust-version`), so the flake follows `nixos-unstable` with its own `rustc` and needs no
  `rust-overlay`. The flake does not pin Rust: it builds with whatever its locked nixpkgs
  carries, and cargo refuses a `rustc` below `rust-version`, so a too old lock fails the build
  instead of building something else.
- Nixpkgs 26.11 (`nixos-unstable`) has dropped `x86_64-darwin`: evaluating the package there
  stops with "Nixpkgs 26.11 has dropped support for x86_64-darwin", and the 26.05 branch it
  points to carries Rust 1.95. So the flake covers `x86_64-linux`, `aarch64-linux` and
  `aarch64-darwin`; an Intel Mac keeps Homebrew or the tarball.
- Docker 29.9.3 on this machine (arm64) runs `nixos/nix` (Nix 2.35.6): `nix flake lock`,
  `nix flake check --all-systems` and `nix build` then `./result/bin/conveyor --version` all
  run there, natively for `aarch64-linux`.
- The only file outside `src/` the binary bundles is `CHANGELOG.md` (`src/changelog.rs`), so
  the flake's source is `Cargo.toml`, `Cargo.lock`, `CHANGELOG.md` and `src/`.
- `--version` prints `conveyor <CARGO_PKG_VERSION>` (`src/main.rs`).
- The `main` ruleset (id 26900132) is the only ruleset; its `conditions.ref_name.include` is
  `["~DEFAULT_BRANCH"]`, so it does not cover `stable`.
- `delete-closed-branch.yml` deletes only the head branch of a pull request closed without
  merging, and `delete_branch_on_merge` only the head branch of a merged one. `stable` is never
  the head of a pull request, so neither touches it.
- `ci.yml` runs on pushes to `main` and on pull requests, `release.yml` on pushes to `main` and
  by hand, `version.yml` on `pull_request_target` into `main`. A push of `stable` starts none
  of them, and a push made with `github.token` starts no workflow anyway.
- `main` carries `version = "0.0.0"`; the real version and changelog live only on the release
  commit each `v*` tag points at (`scripts/release-tag.sh`), which is not on `main`. So
  `nix run github:piacsek/conveyor` builds `v0.0.0`.

## Design
1. `flake.nix` and `flake.lock` at the root: `packages.default` and `apps.default` for
   `x86_64-linux`, `aarch64-linux` and `aarch64-darwin` (not `x86_64-darwin`, see above), built with
   `rustPlatform.buildRustPackage` from the four paths above, `cargoLock.lockFile =
   ./Cargo.lock`, the name, version and description read from `Cargo.toml`, `doCheck = false`
   (the suite needs tmux, git and tags). `gh`, `git` and `kubectl` stay on `PATH`, as the README
   says; the flake does not bring them. One input, nixpkgs.
2. **A `stable` branch at the newest release.** The release run moves `stable` to the commit of
   the newest published release, so `nix run github:piacsek/conveyor/stable` and
   `nix profile install github:piacsek/conveyor/stable` (with
   `--extra-experimental-features 'nix-command flakes'` in the README, so a default Nix install
   runs it) give the newest release with its real
   version and changelog. Two new jobs, `stable-build` and `stable`, in `release.yml`, after
   `publish`, built the same way as `tap`: they reconcile rather than replay.
   - `scripts/stable-version.sh` decides, like `scripts/formula-version.sh`: it reads the
     version in `Cargo.toml` at `stable` (`gh api`, a 404 meaning no `stable` yet) and prints
     `version=<v>` for the newest published `complete` release above it, nothing otherwise.
     The release walk and the version comparison move from `formula-version.sh` into
     `scripts/releases.sh`, shared by both.
   - When it names a version, `stable-build` (`contents: read`) checks out that tag without
     keeping a credential, runs `nix build`, refuses unless `./result/bin/conveyor --version`
     prints `conveyor <v>`, and only then sets its `version` output.
   - `stable` (`contents: write`, no Nix, runs nothing from the tag) starts only when
     `stable-build` succeeded with a version, asks the script again and pushes the tag's commit
     to `refs/heads/stable`, forced (release commits are siblings, not descendants), only when
     the answer is still that version: another run may have moved `stable` or published a newer
     release while this one built.
   - `stable-build`: `needs: [tag, publish]`, `if: !cancelled()`, so it runs after `publish` in
     every run, built or not. `stable` alone holds `concurrency: stable` with no cancelling, so
     the second ask and the push of two runs never interleave. No `github.repository` guard: a
     fork's own `stable` follows the fork's own releases, and the throwaway repository proves it.
   - Review round 1 split the first single job (`contents: write` while it built the tag, and
     the Nix install writes the token into `nix.conf`) into these two.
3. **The flake in CI**: a `flake` job in `ci.yml`, beside `check` and `audit`, builds the flake
   on Linux and macOS and checks `--version`. It is not a required check (the ruleset is not
   changed): a Nix or nixpkgs outage must never block a merge, and the release run guards
   `stable` anyway. It runs on every pull request, with no path filter: it is not required, so
   it never blocks; it runs beside `check`, so it never slows it; and a filter would need a
   separate workflow or another action for a job whose input set (`src/`, the manifests, the
   flake, `CHANGELOG.md`) most pull requests touch anyway. Measured on #106: Linux 1 min 49 s,
   macOS 3 min 9 s, beside a 1 min 35 s `check`. No Nix store cache: the time is the crate and
   its dependencies built from source, and nothing waits on the job.
4. **Fragment and "feeds the binary".** `flake.nix` and `flake.lock` stay off the list in
   `scripts/feeds-binary.sh`: they decide what a Nix user runs, so a flake-only change needs a
   fragment and releases a patch, which is what moves `stable` to it. This pull request ships
   `changes/phase-51-nix-flake.md`, `kind: feat`.

| State the `stable` job meets | What it does |
| --- | --- |
| first release after the merge (no `stable` yet, `gh api` 404) | builds the newest complete release with Nix and creates `stable` there |
| normal release (a newer complete release than `stable`) | builds it and moves `stable` to it |
| rerun of a finished run | `stable` already names the newest complete release: prints nothing, skips the rest, green |
| rerun of an old run after newer releases | `stable` is at or above every release it could name: nothing, green; `stable` never goes back |
| a run with nothing pending (`build` and `publish` skipped) | reconciles: nothing when `stable` is current, else moves it (a run cancelled between `publish` and `stable` is repaired) |
| the newest release is a draft or short of an asset | passed over for the newest complete one above `stable`, if any |
| the tag's flake does not build, or `--version` differs | the job fails and `stable` stays where it was; every later run retries until a newer release builds |
| the newest complete release predates the flake (only if the first release after the merge never publishes) | `nix build` finds no flake and fails, as above, until that release is finished |
| `stable`'s `Cargo.toml` names no `X.Y.Z` version | exit 1, fix by hand |
| `gh` cannot read `stable` or the releases | exit 1, nothing pushed; the next run retries |

Order: until the merge of this pull request is released, `stable` does not exist and the README
command fails. The merge releases (the fragment), and the `stable` job of that same run creates
the branch.

## Tasks
1. `flake.nix`, `flake.lock`, this plan, the fragment and the README install lines; proven with
   Docker (`nix flake check --all-systems`, `nix build`, `--version`).
2. `scripts/stable-version.sh` with `tests/scripts/stable_version.rs` (`gh` as
   `tests/fixtures/reply-call.sh`), one test per row of the decision; the shared release walk
   moves into `scripts/releases.sh`.
3. The `stable` job in `release.yml`, pinned by a test; proven on the throwaway repository
   (first release, a second release, a rerun of a finished run).
4. The `flake` job in `ci.yml`, measured.
5. Guidelines: root `AGENTS.md` (layout, Releasing), `.github/AGENTS.md` (the job, why it is
   not required, recovery), `PLAN.md`.
