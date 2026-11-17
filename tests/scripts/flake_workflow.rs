use std::fs;
use std::path::Path;

#[test]
fn the_paths_job_decides_from_what_changed_since_the_base() {
    assert!(ci().contains(PATHS_JOB), "{}", ci());
}

#[test]
fn the_flake_builds_on_linux_and_macos_unless_the_paths_job_said_false() {
    assert!(ci().contains(FLAKE_JOB), "{}", ci());
}

#[test]
fn flake_ok_judges_the_paths_and_flake_jobs_whatever_they_ended_in() {
    assert!(ci().contains(FLAKE_OK_JOB), "{}", ci());
}

#[test]
fn the_workflow_starts_on_every_pull_request_and_every_push_to_main() {
    assert!(ci().starts_with(TRIGGERS), "{}", ci());
}

fn ci() -> String {
    fs::read_to_string(Path::new(env!("CARGO_MANIFEST_DIR")).join(".github/workflows/ci.yml"))
        .unwrap()
}

const TRIGGERS: &str = "name: ci

on:
  push:
    branches: [main]
  pull_request:

";

const PATHS_JOB: &str = "  paths:
    runs-on: ubuntu-latest
    outputs:
      build: ${{ steps.decide.outputs.build }}
    steps:
      - uses: actions/checkout@fbc6f3992d24b796d5a048ff273f7fcc4a7b6c09 # v5
        with:
          fetch-depth: 2
      - id: decide
        env:
          BASE: ${{ github.event_name == 'pull_request' && 'HEAD^1' || github.event.before }}
        run: scripts/flake-build-needed.sh \"$BASE\" | tee -a \"$GITHUB_OUTPUT\"
";

const FLAKE_JOB: &str = "  flake:
    needs: paths
    if: ${{ !cancelled() && needs.paths.outputs.build != 'false' }}
    strategy:
      fail-fast: false
      matrix:
        os: [ubuntu-latest, macos-latest]
    runs-on: ${{ matrix.os }}
    steps:
      - uses: actions/checkout@fbc6f3992d24b796d5a048ff273f7fcc4a7b6c09 # v5
      - uses: cachix/install-nix-action@75c75c31ebece974e48ebc36b4ad125bd97bd97b # v31.11.3
      - run: nix build
      - run: test \"$(./result/bin/conveyor --version)\" = \"conveyor $(grep -m1 '^version = ' Cargo.toml | cut -d'\"' -f2)\"
";

const FLAKE_OK_JOB: &str = "  flake-ok:
    needs: [paths, flake]
    if: always()
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@fbc6f3992d24b796d5a048ff273f7fcc4a7b6c09 # v5
      - env:
          PATHS: ${{ needs.paths.result }}
          BUILD: ${{ needs.paths.outputs.build }}
          FLAKE: ${{ needs.flake.result }}
        run: scripts/flake-ok.sh \"$PATHS\" \"$BUILD\" \"$FLAKE\"
";
