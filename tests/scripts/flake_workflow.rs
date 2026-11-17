use std::fs;
use std::path::Path;

#[test]
fn every_pull_request_builds_the_flake_on_linux_and_macos_and_checks_its_version() {
    let ci =
        fs::read_to_string(Path::new(env!("CARGO_MANIFEST_DIR")).join(".github/workflows/ci.yml"))
            .unwrap();

    assert!(ci.contains(FLAKE_JOB), "{ci}");
}

const FLAKE_JOB: &str = "  flake:
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
