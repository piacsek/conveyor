# conveyor

A terminal view of where your changes are: open pull requests, the merge
queue, the last main builds, and what each environment runs. One column per
stage, one repo, one screen.

Status: phase 0 (scaffolding). `conveyor --help`, `conveyor --version` and
`conveyor config` work; the TUI lands in phase 1.

## Install

Homebrew (macOS, Linux), pulls in `gh` if missing:

```sh
brew install piacsek/tap/conveyor
```

Prebuilt binary (macOS arm64/x86_64, Linux x86_64) into `~/.local/bin`, checked
against the release's SHA-256:

```sh
t="conveyor-$(uname -m | sed s/arm64/aarch64/)-$(uname -s | sed 's/Darwin/apple-darwin/;s/Linux/unknown-linux-gnu/')"
curl -fsSLO "https://github.com/piacsek/conveyor/releases/latest/download/$t.tar.gz{,.sha256}"
shasum -a 256 -c "$t.tar.gz.sha256" && mkdir -p ~/.local/bin && tar xzf "$t.tar.gz" -C ~/.local/bin
```

Releases carry a GitHub build-provenance attestation:
`gh attestation verify "$t.tar.gz" --repo piacsek/conveyor`.

From source (needs Rust 1.98+):

```sh
cargo install --git https://github.com/piacsek/conveyor --locked
```

### Dependencies

Runtime:

- `gh` (GitHub CLI), logged in: `gh auth login`. Every GitHub read goes through it, so its
  keyring login and SSO apply. Homebrew installs it as a dependency.
- `open` and `pbcopy` (macOS) or `xdg-open` and `xclip` (Linux) for opening a row in the
  browser and copying its URL.
- `kubectl` with a working context, only for the deployed column (phase 4).

Build: Rust 1.98 or newer. No other toolchain.

## Configure

`~/.config/conveyor/config.toml` (or `$XDG_CONFIG_HOME/conveyor/config.toml`, or the file
named by `CONVEYOR_CONFIG`). Every key is optional; unknown keys are an error.
`conveyor config` prints the effective values:

```toml
[prs]
query = "is:pr is:open author:@me archived:false"
limit = 20
refresh_secs = 60
```

## Develop

`scripts/gates.sh` runs fmt, clippy, tests and the ignored e2e tests; `scripts/dev-install.sh`
exposes the working tree as `conveyor-dev`. See `AGENTS.md`.
