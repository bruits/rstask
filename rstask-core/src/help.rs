use crate::constants::*;

pub fn show_help(cmd: &str) {
    let help_text = match cmd {
        CMD_NEXT | CMD_SHOW_NEXT => {
            r#"Usage: rstask next [filter] [--]
Usage: rstask [filter] [--]
Example: rstask +work +bug --

Display list of non-resolved tasks in the current context, most recent last,
optional filter. It is the default command, so "next" is unnecessary.

Bypass the current context with --.

"#
        }

        CMD_ADD => {
            r#"Usage: rstask add [template:<id>] [task summary] [--]
Example: rstask add Fix main web page 500 error +bug P1 project:website

Add a task, returning the git commit output which contains the task ID, used
later to reference the task.

Tags, project and priority can be added anywhere within the task summary.

Add -- to ignore the current context. / can be used when adding tasks to note
any words after.

A copy of an existing task can be made by including "template:<id>". See
"rstask help template" for more information on templates.

"#
        }

        CMD_TEMPLATE => {
            r#"Usage rstask template <id> [task summary] [--]
Example: rstask template Fix main web page 500 error +bug P1 project:website
Example: rstask template 34 project:

If valid task ID is supplied, a copy of the task is created as a template. If
no ID is given, a new task template is created.

Tags, project and priority can be added anywhere within the task summary.

Add -- to ignore the current context. / can be used when adding tasks to note
any words after

Template tasks are not displayed with "show-open" or "show-next" commands.
Their intent is to act as a readily available task template for commonly used
or repeated tasks.

To create a new task from a template use the command:
"rstask add template:<id> [task summary] [--]"
The template task <id> remains unchanged, but a new task is created as a copy
with any modifications made in the task summary.

Github-style task lists (checklists) are recommended for templates, useful for
performing procedures. Example:

- [ ] buy bananas
- [ ] eat bananas
- [ ] make coffee

"#
        }

        CMD_RM | CMD_REMOVE => {
            r#"Usage: rstask remove <id...>
Example: rstask 15 remove

Remove a task.

The task is deleted from the filesystem, and the change is committed.

"#
        }

        CMD_LOG => {
            r#"Usage: rstask log [task summary] [--]
Example: rstask log Fix main web page 500 error +bug P1 project:website

Add an immediately resolved task. Syntax identical to add command.

Tags, project and priority can be added anywhere within the task summary.

Add -- to ignore the current context.

"#
        }

        CMD_START => {
            r#"Usage: rstask <id...> start
Usage: rstask start [task summary] [--]
Example: rstask 15 start
Example: rstask start Fix main web page 500 error +bug P1 project:website

Mark a task as active, meaning you're currently at work on the task.

Alternatively, "start" can add a task and start it immediately with the same
syntax is the "add" command.  Tags, project and priority can be added anywhere
within the task summary.

Add -- to ignore the current context.
"#
        }

        CMD_NOTE | CMD_NOTES => {
            r#"Usage: rstask note <id>
Usage: rstask note <id> <text>
Example task 13 note problem is faulty hardware

Edit or append text to the markdown notes attached to a particular task.
"#
        }

        CMD_STOP => {
            r#"Usage: rstask <id...> stop [text]
Example: rstask 15 stop
Example: rstask 15 stop replaced some hardware

Set a task as inactive, meaning you've stopped work on the task. Optional text
may be added, which will be appended to the note.
"#
        }

        CMD_RESOLVE | CMD_DONE => {
            r#"Usage: rstask <id...> done [closing note]
Example: rstask 15 done
Example: rstask 15 done replaced some hardware

Resolve a task. Optional text may be added, which will be appended to the note.
"#
        }

        CMD_CONTEXT => {
            r#"Usage: rstask context <filter>
Example: rstask context +work -bug
Example: rstask context none

Set a global filter consisting of a project, tags or antitags. Subsequent new
tasks and most commands will then have this filter applied automatically.

For example, if you were to run "task add fix the webserver," the given task
would then have the tag "work" applied automatically.

To reset to no context, run: rstask context none

Context can also be set with the environment variable RSTASK_CONTEXT. If set,
this context string will override the context stored on disk.
"#
        }

        CMD_MODIFY => {
            r#"Usage: rstask <id...> modify <filter>
Usage: rstask modify <filter>
Example: rstask 34 modify -work +home project:workbench -project:website

Modify the attributes of the given tasks, specified by ID. If no ID is given,
the operation will be performed to all tasks in the current context subject to
confirmation.

Modifiable attributes: tags, project and priority.
"#
        }

        CMD_EDIT => {
            r#"Usage: rstask <id...> edit

Edit a task in your text editor.
"#
        }

        CMD_UNDO => {
            r#"Usage: rstask undo
Usage: rstask undo <n>

Undo the last <n> commits on the repository. Default is 1. Use

	rstask git log

To see commit history. For more complicated history manipulation it may be best
to revert/rebase/merge on the rstask repository itself. The rstask repository
is at ~/.rstask by default.
"#
        }

        CMD_SYNC => {
            r#"Usage: rstask sync

Synchronise with the remote git server. Runs git pull then git push. If there
are conflicts that cannot be automatically resolved, it is necessary to
manually resolve them in  ~/.rstask or with the "task git" command.
"#
        }

        CMD_GIT => {
            r#"Usage: rstask git <args...>
Example: rstask git status

Run the given git command inside ~/.rstask
"#
        }

        CMD_SHOW_RESOLVED => {
            r#"Usage: rstask resolved

Show a report of last 1000 resolved tasks.
"#
        }

        CMD_SHOW_TEMPLATES => {
            r#"Usage: dtask show-templates [filter] [--]

Show a report of stored template tasks with an optional filter.

Bypass the current context with --"#
        }

        CMD_OPEN => {
            r#"Usage: rstask <id...> open

Open all URLs found within the task summary and notes. If you commonly have
dozens of tabs open to later action, convert them into tasks to open later with
this command.
"#
        }

        CMD_SHOW_PROJECTS => {
            r#"Usage: rstask show-projects

Show a breakdown of projects with progress information
"#
        }

        _ => {
            r#"Usage: rstask [id...] <cmd> [task summary/filter]

Where [task summary] is text with tags/project/priority specified. Tags are
specified with + (or - for filtering) eg: +work. The project is specified with
a project:g prefix eg: project:rstask -- no quotes. Priorities run from P3
(low), P2 (default) to P1 (high) and P0 (critical). Text can also be specified
for a substring search of description and notes.

Cmd and IDs can be swapped, multiple IDs can be specified for batch
operations.

run "rstask help <cmd>" for command specific help.

Add -- to ignore the current context. / can be used when adding tasks to note
any words after.

Available commands:

next              : Show most important tasks (priority, creation date -- truncated and default)
add               : Add a task
template          : Add a task template
log               : Log a task (already resolved)
start             : Change task status to active
note              : Append to or edit note for a task
stop              : Change task status to pending
done              : Resolve a task
context           : Set global context for task list and new tasks (use "none" to set no context)
modify            : Change task attributes specified on command line
edit              : Edit task with text editor
undo              : Undo last n commits
sync              : Pull then push to git repository, automatic merge commit.
open              : Open all URLs found in summary/annotations
git               : Pass a command to git in the repository. Used for push/pull.
remove            : Remove a task (use to remove tasks added by mistake)
show-projects     : List projects with completion status
show-tags         : List tags in use
show-active       : Show tasks that have been started
show-paused       : Show tasks that have been started then stopped
show-open         : Show all non-resolved tasks (without truncation)
show-resolved     : Show resolved tasks
show-templates    : Show task templates
show-unorganised  : Show untagged tasks with no projects (global context)
help              : Get help on any command or show this message
version           : Show rstask version information

"#
        }
    };

    eprintln!("{}", help_text);
}
