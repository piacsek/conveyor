kind: fix

- Main builds appears much sooner on the first load: the pull requests of all its runs are asked for in one request instead of one request per run (about 4 s instead of 27 s for 30 builds).
- The Deployed column asks for the pull requests of all its environments in one request as well.
