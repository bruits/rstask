# rstask

rstask is a personal task tracker written in Rust, designed to help you focus on what matters. It is a Rust port of [dstask](https://github.com/naggie/dstask).

## How It Works

Tasks are stored as individual Markdown files with YAML frontmatter inside a git repository at `~/.rstask`. Every change you make: adding, modifying, or resolving a task, is automatically committed. This gives you a full history of your task management and makes syncing between machines as simple as `git push` and `git pull`.

## Key Concepts

- **Tasks** have a summary, optional notes (Markdown), tags, a project, a priority, and a due date.
- **Statuses** control the lifecycle of a task: `pending`, `active`, `paused`, `resolved`, `template`, `delegated`, `deferred`, and `recurring`.
- **Priority** ranges from `P0` (critical) to `P3` (low). The default is `P2` (normal).
- **Context** is a persistent filter you can set so that commands only show tasks relevant to what you're currently working on. For example, you could start your day by setting your context to `work` and only see work-related tasks until you change it again.
- **Templates** let you define reusable task blueprints for repeated workflows.

## Task Lifecycle

A typical task moves through these statuses:

```
pending  -->  active  -->  resolved
                |
                v
             paused   -->  resolved
```

A `pending` task can also be converted into a `template` or directly marked as `resolved`.

## File Format

Each task is a `.md` file named by its UUID. The frontmatter holds structured metadata and the body holds freeform Markdown notes:

```yaml
---
summary: Fix the login page
tags: [frontend, urgent]
project: website
priority: P1
status: pending
created: 2025-11-05T10:00:00Z
due: 2025-11-12T00:00:00Z
---
The login form throws a 500 when the email contains a `+` character.
```

## Non-TTY Output

When stdout is not a terminal (e.g. when piping to another command), rstask outputs JSON instead of a colored table. This makes it easy to integrate with other tools.
