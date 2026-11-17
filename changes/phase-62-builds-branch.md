kind: feat

- Main builds reads the repository's own default branch instead of a branch named `main`, so a repository built on `master`, `develop` or `trunk` fills the column and its Deployed rows tell how far behind they are.
- A new optional `branch` key under `[[repo]]` picks another branch for Main builds. When the default branch cannot be read, the column shows the error instead of staying empty.
