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
- Each directory has its own `AGENTS.md` with the facts about its code; read it before touching
  that directory.

## Checks and shipping

- [`scripts/gates.sh`](../scripts/gates.sh) runs every check a change must pass.
- [`scripts/ship.sh`](../scripts/ship.sh) runs the checks, then commits and pushes.
- A pull request that changes what the binary is built from adds a fragment under
  [`changes/`](../changes/); see Releasing in [`AGENTS.md`](../AGENTS.md#releasing).
