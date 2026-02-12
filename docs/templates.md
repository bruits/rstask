# Templates

Templates are reusable task blueprints. They are useful for recurring work that follows the same structure each time.

## Creating a Template

You can create a template from scratch or convert an existing task into one.

```sh
# Create a new template directly
rstask template Weekly review +recurring project:admin

# Convert an existing task into a template
rstask template 34
```

## Using a Template

When adding a task, reference a template by its ID with `template:ID`. The new task inherits the template's summary, tags, project, priority, and notes, but you can override any of these inline.

```sh
rstask add template:5
rstask add template:5 +extra-tag due:next-monday
```

## Viewing Templates

Templates are hidden from `next` and `show-open`. Use `show-templates` to see them.

```sh
rstask show-templates
```
