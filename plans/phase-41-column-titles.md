# Phase 41: the column titles can be renamed in the config

Issue: [#83](https://github.com/piacsek/conveyor/issues/83).

## Context
The four column titles ("My PRs", "Merge queue", "Main builds", "Deployed") are fixed text.
A new optional `[ui.titles]` table renames each one.

## Compatibility
Every key is optional; unset keeps today's title, so an existing config loads and looks the
same. `conveyor config` prints only the keys that are set: with none set it prints no
`[ui.titles]` table, so its output for an existing config does not change. The only new error
is an empty (or blank) title, naming the file and the key.

## Design

```toml
[ui.titles]
prs = "Mine"
queue = "Queue"
builds = "Main"
deployed = "Envs"
```

| Config | Repo or system configured | Title |
| --- | --- | --- |
| unset | no | `My PRs`, `Merge queue`, `Main builds`, `Deployed` (today) |
| unset | yes | `Queue <repo>`, `Main <repo>`, `Deployed <system>` (today) |
| set | no | `<name>` |
| set | yes | `<name> <repo>` / `<name> <system>` |

The count, the `!N` marks and the `⚠` follow the name as today. The name shows on the column
border, on the column bar of the narrow layout (both drawn by `column_title`) and in the `?`
help header (`Keys — <name>`).

## Tasks
1. `src/config.rs`: `Titles` under `Ui`, `Option<String>` per stage, `deny_unknown_fields`,
   skipped when none is set; a blank title is an error naming the file and the key.
2. `src/ui/columns.rs`: `column_title` takes the base name from the config, with the repo or
   system suffix kept; on the border and the narrow layout's bar.
3. `src/ui/help.rs`: the help header reads the same name.
4. `src/ui/AGENTS.md` line on the title source, README if its config block needs it, the
   fragment `changes/phase-41-column-titles.md` (`kind: feat`).

## Not in scope
Named lists inside My PRs and any other label in the UI.
