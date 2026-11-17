kind: fix

- A pull request title with emoji or Chinese, Japanese or Korean characters now ends inside
  the confirm dialog instead of being cut at its border, and keeps a card's age in place.
- A failing check with such a name no longer pushes the next one past the card's edge, and a
  tab bar of such tab names ends with `…` at the column's edge.
- A Deployed card whose branch name holds such characters keeps its last one inside the card,
  and a tab key set to such a character keeps the help's descriptions in line.
