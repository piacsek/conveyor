kind: feat

- `S` works on another pull request while an action on the first one is still running; only the pull request with an action on its way refuses it.
- A pull request card with an action on its way shows a spinner in place of its check mark and names the action (`marking ready…`, `merging…`) on its second line until the result lands.
