---
cargo/rstask: minor
cargo/rstask-core: minor
---

Rework CLI and convert yaml to Markdown.

`rstask` now uses Markdown files instead of YAML for tasks. This allows you to use and edit tasks notes as actual Markdown instead of cramping it inside the `notes` property like in `dstask`. Additionally, `rstask show <id>` will nicely render the task's notes to the terminal.
