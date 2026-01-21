# wire bash completions to rstask completion engine. Some of the workarounds for
# idiosyncrasies around separation with colons taken from /etc/bash_completion

_rstask() {
    # reconstruct COMP_WORDS to re-join separations caused by colon (which is a default separator)
    # yes, this method ends up splitting by spaces, but that's not a problem for the rstask parser
    # see http://tiswww.case.edu/php/chet/bash/FAQ
    original_args=( $(echo "${COMP_WORDS[@]}" | sed 's/ : /:/g' | sed 's/ :$/:/g') )

    # hand to rstask as canonical args
    COMPREPLY=( $(rstask _completions "${original_args[@]}") )

    # convert rstask's suggestions to remove prefix before colon so complete can understand it
    local last_arg="${original_args[-1]}"
    local colon_word=${last_arg%"${last_arg##*:}"}
    local i=${#COMPREPLY[*]}
    while [[ $((--i)) -ge 0 ]]; do
        COMPREPLY[$i]=${COMPREPLY[$i]#"$colon_word"}
    done
}

complete -F _rstask rstask
complete -F _rstask task
#complete -F _rstask n
#complete -F _rstask t
