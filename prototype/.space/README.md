# Space runtime data

This directory is the project-local persistence boundary for a Space.

- `space.json` will store Space metadata, the main thread id, and View registry state.
- `main/` will store the generated Main Page.
- `views/<view-id>/` will store each generated View page and its local assets.

Generated pages remain ordinary web files so they can be inspected, diffed, and restored with Git.
