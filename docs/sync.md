# Syncing and Git

rstask uses a git repository as its storage backend. Every mutation (adding, modifying, resolving, or removing a task) automatically create a commit. This makes syncing between machines straightforward and gives you a full audit trail of changes.

## Syncing

The `sync` command pulls from the remote and then pushes local changes. It handles upstream branch setup automatically on the first sync.

```sh
rstask sync
```

You can configure rstask to sync automatically after every modification by setting `sync_frequency` to `after_every_modification` in your config file (`$XDG_CONFIG_DIR/rstask/config.styx`).

## Setting Up a Remote

Use the `git` passthrough command to add a remote to your task repository:

```sh
rstask git remote add origin git@github.com:user/tasks.git
rstask sync
```

## Undoing Changes

The `undo` command reverts the most recent commits in the task repository. By default it undoes the last commit, but you can specify a number.

```sh
rstask undo
rstask undo 3
```

This performs a hard reset, so the changes are discarded entirely. Use with care if you have already synced.

## Arbitrary Git Commands

The `git` subcommand lets you run any git command inside the task repository without having to `cd` into it.

```sh
rstask git status
rstask git log --oneline -10
rstask git diff HEAD~1
```

## Bulk Commit Strategy

When modifying multiple tasks at once (e.g. `rstask modify +tag` with no IDs), the `bulk_commit_strategy` preference controls how commits are created:

- `per_task` (default) -- one commit per modified task.
- `single` -- a single commit for all changes.
