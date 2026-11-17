kind: fix

- A crash in a background fetch or action no longer leaves the terminal broken (keys echoed, `q` not quitting) or a column, pane or card waiting forever. The crash shows where the result would have, as `crashed: <reason>`: as the column's error, in the jobs or log pane, or as the failure mark on the card. The next refresh or action works again, and the crash is written to the call log (`conveyor log`).
