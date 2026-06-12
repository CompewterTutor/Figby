# Figby — Master Todo Index

Master index for all milestones. Ralph loop reads task lines from
versioned files below. Do not add task lines directly here.

## Milestone Index

| Milestone | File | Description | Status |
|-----------|------|-------------|--------|
| v1 | [todo-v1.md](todo-v1.md) | C-to-Rust Port | Active |
| v2 | [todo-v2.md](todo-v2.md) | Polish & Extensions | Active |

## Conventions

- Tasks: `- [ ] \`X.Y.Z\`` — checked off on merge
- Phase merge: `- [ ] release/X.Y → main` — checked after review
- Each minor version maps to one `release/X.Y` branch
- Each task maps to one `task-X.Y.Z` branch off the release branch
- Major versions (X.0) require human sign-off
