kind: fix

- A refresh of less than 10 seconds in the config is refused at start, naming the file and the
  setting, instead of polling GitHub until its rate limit runs out.
- A `limit` or a number of `builds` outside 1 to 100 is refused at start, instead of failing in
  its column or showing a page size nobody asked for.
- A `details_percent` outside 10 to 90 or a `min_column_width` above 1000 is refused at start,
  instead of hiding the columns or wrapping the width around.
