kind: fix

- A refresh of less than 10 seconds in the config is refused at start, naming the file and the
  setting, instead of polling GitHub until its rate limit runs out.
- A `limit` or a number of `builds` outside 1 to 100 is refused at start, instead of failing in
  its column or showing a page size nobody asked for.
- A `details_percent` outside 10 to 90 or a `min_column_width` above 1000 is refused at start,
  instead of hiding the columns or wrapping the width around.
- A `[[repo]]` name that is not `owner/name`, or an environment with a blank name, context,
  namespace or deployment, is refused at start instead of failing later in its column.
- Two `[[repo]]` of one name (in any case) are refused at start instead of showing twice, and two
  environments of one name in a deploy instead of sharing one card's selection and marks.
- A second `[[repo.deploy]]`, or a deploy on a later repository, is refused at start, naming the
  one conveyor reads, instead of being ignored without a word.
