# Contributing

Thanks for helping. Issues labelled
[`good first issue`](https://github.com/piacsek/conveyor/labels/good%20first%20issue) and
[`help wanted`](https://github.com/piacsek/conveyor/labels/help%20wanted) are a good place to
start. Questions and requests for a new deploy source go to
[Discussions](https://github.com/piacsek/conveyor/discussions).

## Before you change anything

- Read [`AGENTS.md`](../AGENTS.md): the rules, the layout, and how work is tested and shipped.
  Start with [This repository is public](../AGENTS.md#this-repository-is-public), which holds for
  issues and screenshots too.
- Before touching a part of the code, read the `AGENTS.md` that the
  [Where the rest lives](../AGENTS.md#where-the-rest-lives) table names for it.

## Checks and shipping

- [`scripts/gates.sh`](../scripts/gates.sh) runs every check a change must pass.
- [`scripts/ship.sh`](../scripts/ship.sh) is the maintainer's: after the checks it
  installs `~/.local/bin/conveyor-dev`, commits everything with `git add -A`, pushes, and opens
  a draft pull request assigned to whoever runs it. From a fork, `scripts/gates.sh` and an
  ordinary pull request are enough.
- A pull request that changes what the binary is built from adds a fragment under
  [`changes/`](../changes/); see Releasing in [`AGENTS.md`](../AGENTS.md#releasing).
