kind: fix

- Two pull requests with the same number in different repositories stay apart in My PRs: a
  refresh keeps the selection on the one you picked, a write or a failure on one no longer
  blocks or marks the other, and a merge of one no longer holds the other back.
- While My PRs lists more than one repository, the `S` popup and the confirm dialog name the
  repository (`Manage infra#12`, `merge infra#12 <title>?`), with its owner when two share a
  name (`Manage me/webapp#12`).
