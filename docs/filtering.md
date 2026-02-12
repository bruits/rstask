# Filtering and Queries

Most rstask commands accept filter arguments that let you narrow down which tasks are displayed or acted upon. Filters can be combined freely in any order.

## Tags

Prefix a word with `+` to filter by tag, or `-` to exclude a tag.

```sh
rstask next +work
rstask next +urgent +frontend
rstask next -blocked
rstask next +work -personal
```

When used with `add` or `modify`, `+tag` adds the tag and `-tag` removes it.

## Projects

Use `project:name` to filter by project. Prefix with `-` to exclude.

```sh
rstask next project:website
rstask next -project:legacy
rstask add Fix styles project:website
```

`+project:name` is also accepted and behaves identically to `project:name`.

## Priority

Specify a priority level directly.

```sh
rstask next P0
rstask next P1
rstask add Critical outage P0 +ops
```

| Priority | Meaning |
|---|---|
| `P0` | Critical |
| `P1` | High |
| `P2` | Normal (default) |
| `P3` | Low |

## Due Dates

Filter or set due dates with the `due:` prefix. Several operators are available for filtering.

```sh
rstask next due:today
rstask next due:overdue
rstask next due.before:friday
rstask next due.after:2025-01-01
rstask next due.on:next-monday
rstask add Submit report due:2025-12-25
rstask modify 15 due:tomorrow
```

### Supported Date Formats

| Format | Example |
|---|---|
| `today` | Current date |
| `tomorrow` | Next day |
| `yesterday` | Previous day |
| `monday` - `sunday` | Next occurrence of that weekday |
| `next-monday` - `next-sunday` | Explicitly next week |
| `this-monday` - `this-sunday` | This week (or next if already past) |
| `YYYY-MM-DD` | `2025-12-25` |
| `MM-DD` | `12-25` (current year) |
| `DD` | `25` (current month and year) |

## Task IDs

Numeric arguments are treated as task IDs. Multiple IDs can be specified and must come before other filter tokens.

```sh
rstask done 15
rstask done 3 7 12
rstask show 15
```

## Text Search

Any unrecognized words are treated as a text search, matching against task summaries and notes as substrings.

```sh
rstask next server error
rstask next login bug
```

## Notes Separator

When using `add` or `log`, everything after `/` is treated as the task's notes rather than part of the summary.

```sh
rstask add Buy milk / at the grocery store near the office
```

## Combining Filters

All filter types can be combined in a single command.

```sh
rstask next +work -blocked project:website P1 due.before:friday
rstask add Deploy v2.0 +ops P0 project:infra due:next-monday / coordinate with the SRE team
```
