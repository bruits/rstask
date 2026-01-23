_rstask() {
    local i cur prev opts cmd
    COMPREPLY=()
    cur="${COMP_WORDS[COMP_CWORD]}"
    prev="${COMP_WORDS[COMP_CWORD-1]}"
    cmd="${COMP_WORDS[1]}"

    # Basic command completion
    if [[ ${COMP_CWORD} -eq 1 ]] ; then
        opts="next add remove template log start stop done context modify edit note undo sync git show open show-open show-active show-paused show-resolved show-templates show-unorganised show-projects show-tags completions help"
        COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
        return 0
    fi

    # Dynamic completions for specific contexts
    case "${cmd}" in
        done|resolve|show|edit|open|stop|start|modify|note|notes)
            # Task ID completion
            if [[ "${cur}" =~ ^[0-9] ]] || [[ ${COMP_CWORD} -eq 2 ]]; then
                local ids=$(rstask _completions ids 2>/dev/null)
                COMPREPLY=( $(compgen -W "${ids}" -- "${cur}") )
                return 0
            fi
            ;;
    esac

    # Project completion after project: prefix
    if [[ "${cur}" == project:* ]]; then
        local prefix="project:"
        local search="${cur#project:}"
        local projects=$(rstask _completions projects 2>/dev/null)
        local suggestions=()
        for proj in ${projects}; do
            suggestions+=("${prefix}${proj}")
        done
        COMPREPLY=( $(compgen -W "${suggestions[*]}" -- "${cur}") )
        return 0
    fi

    # Tag completion after + prefix
    if [[ "${cur}" == +* ]]; then
        local prefix="+"
        local search="${cur#+}"
        local tags=$(rstask _completions tags 2>/dev/null)
        local suggestions=()
        for tag in ${tags}; do
            suggestions+=("${prefix}${tag}")
        done
        COMPREPLY=( $(compgen -W "${suggestions[*]}" -- "${cur}") )
        return 0
    fi

    # Anti-tag completion after - prefix
    if [[ "${cur}" == -* ]] && [[ ! "${cur}" =~ ^--[a-z] ]]; then
        local prefix="-"
        local search="${cur#-}"
        local tags=$(rstask _completions tags 2>/dev/null)
        local suggestions=()
        for tag in ${tags}; do
            suggestions+=("${prefix}${tag}")
        done
        COMPREPLY=( $(compgen -W "${suggestions[*]}" -- "${cur}") )
        return 0
    fi

    # Priority completion
    if [[ "${cur}" == P* ]]; then
        COMPREPLY=( $(compgen -W "P0 P1 P2 P3" -- "${cur}") )
        return 0
    fi

    # Default: no completion
    return 0
}

complete -F _rstask rstask
