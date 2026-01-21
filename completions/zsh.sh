#compdef rstask
#autoload


_rstask() {
    compadd -- $(rstask _completions "${words[@]}")
}

compdef _rstask rstask
