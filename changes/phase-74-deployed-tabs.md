kind: feat

- The Deployed column has one tab per `[[repo.deploy]]`, named by its `system`: add several deploys, on one `[[repo]]` or on several, and switch between them with `Tab`, `S-Tab` or `1`-`9` while Deployed is focused. Switching leaves Merge queue and Main builds where they are.
- Every Deployed tab is read in the background on its own `refresh_secs`, so a tab is already filled when you switch to it.
- The `↓N` behind count and the `b` and `Y` links of a Deployed row follow the repository of the tab shown.
- `conveyor` refuses a config where two deploys share a `system` (case ignored), or where one of several deploys has no `system`, naming the system or the repository.
