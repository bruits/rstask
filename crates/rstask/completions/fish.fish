# Fish completion for rstask

# Commands
complete -c rstask -f -n "__fish_use_subcommand" -a "next" -d "Show most important tasks"
complete -c rstask -f -n "__fish_use_subcommand" -a "add" -d "Add a new task"
complete -c rstask -f -n "__fish_use_subcommand" -a "remove" -d "Remove a task"
complete -c rstask -f -n "__fish_use_subcommand" -a "template" -d "Create or manage task templates"
complete -c rstask -f -n "__fish_use_subcommand" -a "log" -d "Log an already completed task"
complete -c rstask -f -n "__fish_use_subcommand" -a "start" -d "Start working on a task"
complete -c rstask -f -n "__fish_use_subcommand" -a "stop" -d "Stop working on a task"
complete -c rstask -f -n "__fish_use_subcommand" -a "done" -d "Mark a task as done"
complete -c rstask -f -n "__fish_use_subcommand" -a "context" -d "Set or view context"
complete -c rstask -f -n "__fish_use_subcommand" -a "modify" -d "Modify task attributes"
complete -c rstask -f -n "__fish_use_subcommand" -a "edit" -d "Edit a task"
complete -c rstask -f -n "__fish_use_subcommand" -a "note" -d "Add or edit notes"
complete -c rstask -f -n "__fish_use_subcommand" -a "undo" -d "Undo last n commits"
complete -c rstask -f -n "__fish_use_subcommand" -a "sync" -d "Sync with remote"
complete -c rstask -f -n "__fish_use_subcommand" -a "git" -d "Run git commands"
complete -c rstask -f -n "__fish_use_subcommand" -a "show" -d "Display a single task"
complete -c rstask -f -n "__fish_use_subcommand" -a "open" -d "Open URLs in task"
complete -c rstask -f -n "__fish_use_subcommand" -a "show-open" -d "Show all non-resolved tasks"
complete -c rstask -f -n "__fish_use_subcommand" -a "show-active" -d "Show active tasks"
complete -c rstask -f -n "__fish_use_subcommand" -a "show-paused" -d "Show paused tasks"
complete -c rstask -f -n "__fish_use_subcommand" -a "show-resolved" -d "Show resolved tasks"
complete -c rstask -f -n "__fish_use_subcommand" -a "show-templates" -d "Show task templates"
complete -c rstask -f -n "__fish_use_subcommand" -a "show-unorganised" -d "Show unorganised tasks"
complete -c rstask -f -n "__fish_use_subcommand" -a "show-projects" -d "List all projects"
complete -c rstask -f -n "__fish_use_subcommand" -a "show-tags" -d "List all tags"
complete -c rstask -f -n "__fish_use_subcommand" -a "completions" -d "Generate shell completions"

# Global options
complete -c rstask -s n -l no-context -d "Ignore the current context filter"
complete -c rstask -s h -l help -d "Print help"
complete -c rstask -s V -l version -d "Print version"

# Dynamic task ID completions for commands that take IDs
function __fish_rstask_task_ids
    set -l token (commandline -t)
    # Only complete IDs if the token is empty or starts with a digit
    if test -z "$token"; or string match -qr '^\d' -- "$token"
        rstask _completions ids 2>/dev/null
    end
end

complete -c rstask -f -n "__fish_seen_subcommand_from done resolve show edit open stop start modify note notes" -a "(__fish_rstask_task_ids)"

# Dynamic argument completions that check token prefix inside the function
function __fish_rstask_dynamic_args
    set -l token (commandline -t)

    # Only complete if we've seen a relevant subcommand
    set -l cmd (commandline -opc)[2]
    if not contains -- $cmd next add log template modify context start done resolve show edit open stop note notes
        return
    end

    # Project completion (only when token starts with "project:")
    if string match -q -- "project:*" $token
        set -l projects (rstask _completions projects 2>/dev/null)
        for proj in $projects
            echo "project:$proj"
        end
        return
    end

    # Tag completion (only when token starts with "+")
    if string match -q -- "+*" $token
        set -l tags (rstask _completions tags 2>/dev/null)
        for tag in $tags
            echo "+$tag"
        end
        return
    end

    # Anti-tag completion (only when token starts with "-" but not "--")
    if string match -q -- "-*" $token
        if not string match -q -- "--*" $token
            set -l tags (rstask _completions tags 2>/dev/null)
            for tag in $tags
                echo "-$tag"
            end
            return
        end
    end

    # Priority completion (only when token starts with "P")
    if string match -q -- "P*" $token
        echo P0
        echo P1
        echo P2
        echo P3
        return
    end
end

# Register the dynamic completions
complete -c rstask -f -a "(__fish_rstask_dynamic_args)"
