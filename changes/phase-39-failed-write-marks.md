kind: feat

- A failed action (merge, update, status change, checkout or rerun) leaves a red `! <action> failed: <reason>` line under the title of the card it was started from, so a later result can no longer wipe it from the footer before you read it. An update that hit conflicts and a rerun where any run failed count as failures.
- `d` on a marked card starts with the whole reason and the time it landed.
- The column title counts the marked cards (`My PRs (5) !1`), in the tabs of a narrow terminal too.
- A mark goes away when you start a new action on that card, press `Esc` with the card selected (after the details pane and zoom, before the filter), or when the card leaves its column.
