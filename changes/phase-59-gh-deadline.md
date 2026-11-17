kind: fix

- A column no longer sits on `fetching…` until conveyor restarts when GitHub stops answering: a `gh` call that has not answered within 30 s is stopped, the column shows `gh did not answer within 30s` with its last rows kept, and the next refresh (or `r`) tries again.
- An action that has not answered within 60 s (merge, update, a status change, rerun) is stopped too, so later actions of the same kind are no longer stuck behind it. Its card says the action may already have happened and to press `r`, for example `gh did not answer within 60s; it may already be merged, press r`.
