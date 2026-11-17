kind: fix

- A refresh of less than 10 seconds in the config is refused at start, naming the file and the
  setting, instead of polling GitHub until its rate limit runs out.
