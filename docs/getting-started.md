# Getting Started

## Installation

Build from source using Cargo:

```sh
cargo install --path crates/rstask
```

## Initial Setup

On first run, rstask initializes a git repository at `~/.rstask`. This is where all your tasks are stored. You can override this path with the `RSTASK_GIT_REPO` environment variable:

```sh
export RSTASK_GIT_REPO=~/my-tasks
```

## Configuration

User preferences are stored in `$XDG_CONFIG_DIR/rstask/config.styx`. Available settings:

| Setting | Values | Default | Description |
|---|---|---|---|
| `sync_frequency` | `never`, `after_every_modification` | `never` | When to auto-sync with the remote |
| `bulk_commit_strategy` | `single`, `per_task` | `per_task` | How to commit bulk modifications |

## Shell Completions

rstask can generate shell completions with dynamic project, tag, and ID suggestions:

```sh
# Bash
rstask completions bash > ~/.local/share/bash-completion/completions/rstask

# Zsh
rstask completions zsh > ~/.zfunc/_rstask

# Fish
rstask completions fish > ~/.config/fish/completions/rstask.fish
```

## Quick Start

Add your first task:

```sh
rstask add Buy groceries +personal
```

See your tasks:

```sh
rstask
```

Start working on one:

```sh
rstask start 1
```

Mark it done:

```sh
rstask done 1
```

## Environment Variables

| Variable | Description |
|---|---|
| `RSTASK_GIT_REPO` | Override the task repository path (default: `~/.rstask`) |
| `RSTASK_CONTEXT` | Override the context filter (bypasses the on-disk context) |
| `EDITOR` | Text editor used by `edit` and `note` commands (default: `vim`) |

## Migrating from dstask

If you have an existing `.dstask` repository, rename it to `.rstask`. rstask reads the legacy `.yml` format and will save tasks in the new `.md` format going forward.
