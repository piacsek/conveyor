kind: fix

- A config with a `refresh_secs` below 5 no longer loads: conveyor starts on the defaults and
  names the file and the setting. A refresh of every second or so ran out the GitHub rate
  limit; write 5 or more.
- A very large `refresh_secs` no longer stops its column refreshing after the first load.
- A `limit` or a number of `builds` outside 1 to 100 is refused at start, instead of failing in
  its column or showing a page size nobody asked for.
- A `details_percent` of 0 or 100 and above, or a `min_column_width` above 16383 is refused at start,
  instead of hiding the columns or wrapping the width around.
- A `[[repo]]` name that is not `owner/name`, or an environment with a blank name or
  deployment, is refused at start instead of failing later in its column. A missing `context`
  or `namespace` still means kubectl's current ones.
- Two `[[repo]]` of one name (in any case) on the same branch and workflow are refused at start
  instead of showing the same tab twice; one repository on two branches still loads. Two
  environments of one name in a deploy are refused instead of sharing one card's selection
  and marks.
- A `[[repo.deploy]]` with no environment, a second `[[repo.deploy]]` on a repository, or a `[[repo.deploy]]` on more than one
  repository, is refused at start, naming the one conveyor reads, instead of being ignored
  without a word.
