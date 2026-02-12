# Context

Context is a persistent filter that is automatically applied to most rstask commands. It lets you focus on a subset of your tasks without having to re-type filter arguments every time.

## Setting Context

```sh
rstask context +work
rstask context project:website
rstask context +work -personal project:website
```

After setting a context, commands like `next`, `show-open`, and `show-active` will only show tasks matching that filter.

## Viewing Context

Run `context` with no arguments to see the current context.

```sh
rstask context
```

## Clearing Context

Use `none` to remove the active context.

```sh
rstask context none
```

## What Context Accepts

Context supports tags, anti-tags, projects, anti-projects, priorities, and due date filters. It does **not** accept task IDs or free text.

```sh
rstask context +frontend P1          # valid
rstask context project:api -legacy   # valid
rstask context 15                    # invalid -- IDs not allowed
rstask context server bug            # invalid -- free text not allowed
```

## Bypassing Context

There are three ways to temporarily ignore the active context for a single command:

1. The `--no-context` (or `-n`) global flag:

```sh
rstask -n next
```

2. Including `--` anywhere in the arguments:

```sh
rstask next --
```

3. The `RSTASK_CONTEXT` environment variable overrides the on-disk context entirely. When this variable is set, the `context` command cannot modify the stored context.

```sh
RSTASK_CONTEXT="+ops" rstask next
```

## Context and Task Creation

When you `add` a task, the current context is merged into the new task. For example, if your context is `+work project:website`, any task you add will automatically receive the `work` tag and the `website` project.
