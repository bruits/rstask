#!/usr/bin/env fish

complete -f -c rstask -a (echo (rstask _completions) | string collect)
#complete -f -c task -a (echo (task _completions) | string collect)
#complete -f -c t -a (echo (t _completions) | string collect)
