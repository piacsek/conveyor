kind: fix

- With several `[[repo]]`, Main builds shows the builds of the repository the Merge queue
  shows, and switches with it. Every repository's builds refresh in the background on its own
  `refresh_secs`, and Deployed keeps reading its own repository's builds.
