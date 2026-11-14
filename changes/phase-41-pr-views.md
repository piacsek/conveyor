kind: feat

- My PRs can show several lists: add a `[[prs.view]]` with a `name` and a `query` (and an
  optional `limit`) for each, and name the first one with `prs.name`. A strip at the top of the
  column names them and highlights the active one.
- Tab and Shift-Tab go to the next and previous list in My PRs, and `1`-`9` jump to one. Each
  list keeps its own selection and filter.
- `[keys].next_view` sets the key that goes to the next list: `"tab"` (the default) or one
  character no other key uses.
- Tab and `1`-`4` no longer move between columns; use `h` and `l`.
