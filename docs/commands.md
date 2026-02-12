# Commands

rstask uses a subcommand-based CLI. All commands accept a global `--no-context` (`-n`) flag to bypass the current context filter.

```sh
rstask [--no-context | -n] <command> [args...]
```

You can also use `--` anywhere in the arguments to ignore context.

---

## next (default)

Alias: `show-next`

Shows the most important tasks, sorted by priority then creation date. The output is truncated to fit your terminal height. This is the default command when you run `rstask` with no subcommand.

```sh
rstask
rstask next
rstask next +work
rstask next project:website P1
rstask -n next
```

---

## add

Creates a new task. Tags, project, priority, and due date can be specified inline. Everything after `/` becomes the task's notes. The current context is merged into the new task automatically.

```sh
rstask add Fix login page
rstask add Fix bug +urgent P1 project:web
rstask add Buy milk +groceries due:tomorrow
rstask add Deploy release / remember to notify the team
rstask add template:5 Weekly standup notes
```

---

## done

Alias: `resolve`

Marks one or more tasks as resolved. Sets the resolved timestamp. A task with incomplete checklist items (`- [ ]`) cannot be resolved. You can append a closing note.

```sh
rstask done 15
rstask done 3 7 12
rstask done 15 Fixed by restarting the service
```

---

## start

Transitions a task from `pending` or `paused` to `active`.

```sh
rstask start 15
rstask start 3 7
```

---

## stop

Transitions a task from `active` to `paused`.

```sh
rstask stop 15
```

---

## modify

Modifies attributes of one or more tasks. You can change tags, project, priority, and due date. If no task IDs are given, it modifies all tasks matching the current context (with a confirmation prompt).

```sh
rstask modify 15 +urgent -later P1
rstask modify 15 project:website
rstask modify 15 due:next-friday
rstask modify 3 7 +blocked
rstask modify +newtag               # applies to all tasks in context
```

---

## edit

Opens a task in your `$EDITOR` for direct editing of the full Markdown frontmatter representation. Accepts exactly one task ID.

```sh
rstask edit 15
```

---

## note

Alias: `notes`

Adds or edits Markdown notes on a task. With no text argument, opens `$EDITOR`. With text, appends it to the existing notes.

```sh
rstask note 15
rstask note 15 Waiting on response from upstream
```

---

## show

Displays a single task with full details and rendered Markdown notes.

```sh
rstask show 15
```

---

## open

Opens all URLs found in a task's summary and notes in your default browser.

```sh
rstask open 15
```

---

## remove

Alias: `rm`

Deletes a task from the repository. Prompts for confirmation in interactive terminals.

```sh
rstask remove 15
rstask rm 15
```

---

## log

Creates a task and immediately marks it as resolved. Useful for recording work that's already been completed.

```sh
rstask log Fixed the CI pipeline +ops project:infra
```

---

## template

Creates or converts a task into a template. If given an ID, the existing task becomes a template. If given text, a new template is created. Templates are hidden from `next` and `show-open`.

```sh
rstask template Weekly review checklist
rstask template 34
```

---

## context

Sets, views, or clears the persistent context filter. Context filters are applied to most commands automatically. Use `none` to clear. Context accepts tags, anti-tags, project, and priority -- but not IDs or free text.

```sh
rstask context
rstask context +work
rstask context project:website -personal
rstask context none
```

---

## sync

Synchronizes the task repository with its remote by pulling then pushing. Handles upstream branch setup automatically on first sync.

```sh
rstask sync
```

---

## undo

Reverts the last n git commits in the task repository (default: 1).

```sh
rstask undo
rstask undo 3
```

---

## git

Runs an arbitrary git command inside the task repository.

```sh
rstask git status
rstask git log --oneline
rstask git remote add origin git@github.com:user/tasks.git
```

---

## Show Commands

These commands display filtered views of your tasks. They all accept the same filter arguments as `next`.

| Command | Description |
|---|---|
| `show-open` | All non-resolved tasks (pending, active, paused, delegated, deferred). No truncation. |
| `show-active` | Only active tasks. |
| `show-paused` | Only paused tasks. |
| `show-resolved` | Resolved tasks, grouped by the week they were resolved. |
| `show-templates` | Task templates. |
| `show-unorganised` | Tasks with no tags and no project. Ignores context. |
| `show-projects` | All projects with completion progress (resolved/total). |
| `show-tags` | All unique tags currently in use. |

```sh
rstask show-open +work
rstask show-active project:website
rstask show-resolved
rstask show-projects
rstask show-tags
rstask show-unorganised
```

---

## completions

Generates shell completion scripts.

```sh
rstask completions bash
rstask completions zsh
rstask completions fish
rstask completions elvish
rstask completions powershell
```
