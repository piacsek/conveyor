kind: fix

- A config with a `refresh_secs` below 10 no longer loads: conveyor starts on the defaults and
  names the file and the setting. Write 10 or more (at most 86400, a day); below 10 it polled
  GitHub until the rate limit ran out, and a huge value stopped that column refreshing.
- A `limit` or a number of `builds` outside 1 to 100 is refused at start, instead of failing in
  its column or showing a page size nobody asked for.
- A `details_percent` outside 10 to 90 or a `min_column_width` above 16383 is refused at start,
  instead of hiding the columns or wrapping the width around.
- A `[[repo]]` name that is not `owner/name`, or an environment with a blank name or
  deployment, is refused at start instead of failing later in its column. A missing `context`
  or `namespace` still means kubectl's current ones.
- Two `[[repo]]` of one name (in any case) are refused at start instead of showing twice, and two
  environments of one name in a deploy instead of sharing one card's selection and marks.
- A second `[[repo.deploy]]` on a repository, or a `[[repo.deploy]]` on more than one
  repository, is refused at start, naming the one conveyor reads, instead of being ignored
  without a word.
- A My PRs column two cells wide or less no longer crashes while drawing its tabs.
