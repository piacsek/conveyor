kind: fix

- A refresh of less than 10 seconds in the config is refused at start, naming the file and the
  setting, instead of polling GitHub until its rate limit runs out.
- A `limit` or a number of `builds` outside 1 to 100 is refused at start, instead of failing in
  its column or showing a page size nobody asked for.
