#compdef rstask

_rstask() {
    local curcontext="$curcontext" state line
    typeset -A opt_args

    _arguments -C \
        '1: :_rstask_commands' \
        '*::arg:->args'

    case $state in
        args)
            case $line[1] in
                done|resolve|show|edit|open|stop|start|modify|note|notes)
                    _rstask_task_ids
                    _rstask_filters
                    ;;
                next|add|log|template)
                    _rstask_filters
                    ;;
                context)
                    _rstask_filters
                    ;;
            esac
            ;;
    esac
}

_rstask_commands() {
    local commands
    commands=(
        'next:Show most important tasks'
        'add:Add a new task'
        'remove:Remove a task'
        'template:Create or manage task templates'
        'log:Log an already completed task'
        'start:Start working on a task'
        'stop:Stop working on a task'
        'done:Mark a task as done'
        'context:Set or view the current context'
        'modify:Modify task attributes'
        'edit:Edit a task in your text editor'
        'note:Add or edit notes for a task'
        'undo:Undo last n commits'
        'sync:Synchronize with remote git repository'
        'git:Run git commands in the task repository'
        'show:Display a single task with full details'
        'open:Open URLs found in task summary and notes'
        'show-open:Show all non-resolved tasks'
        'show-active:Show active tasks'
        'show-paused:Show paused tasks'
        'show-resolved:Show resolved tasks'
        'show-templates:Show task templates'
        'show-unorganised:Show unorganised tasks'
        'show-projects:List all projects'
        'show-tags:List all tags in use'
        'completions:Generate shell completions'
    )
    _describe -t commands 'rstask commands' commands
}

_rstask_task_ids() {
    local ids
    ids=(${(f)"$(rstask _completions ids 2>/dev/null)"})
    _describe -t ids 'task IDs' ids
}

_rstask_filters() {
    # Project completion
    if [[ $PREFIX == project:* ]]; then
        local projects
        projects=(${(f)"$(rstask _completions projects 2>/dev/null)"})
        local suggestions=()
        for proj in $projects; do
            suggestions+=("project:$proj")
        done
        compadd -a suggestions
        return
    fi

    # Tag completion (with +)
    if [[ $PREFIX == +* ]]; then
        local tags
        tags=(${(f)"$(rstask _completions tags 2>/dev/null)"})
        local suggestions=()
        for tag in $tags; do
            suggestions+=("+$tag")
        done
        compadd -a suggestions
        return
    fi

    # Anti-tag completion (with -)
    if [[ $PREFIX == -* ]] && [[ ! $PREFIX =~ ^--[a-z] ]]; then
        local tags
        tags=(${(f)"$(rstask _completions tags 2>/dev/null)"})
        local suggestions=()
        for tag in $tags; do
            suggestions+=("-$tag")
        done
        compadd -a suggestions
        return
    fi

    # Priority completion
    if [[ $PREFIX == P* ]]; then
        compadd P0 P1 P2 P3
        return
    fi
}

_rstask
