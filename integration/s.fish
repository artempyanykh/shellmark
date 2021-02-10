#!/usr/bin/env fish

function {name} 
    if ! type -q shellmark
        echo "shellmark is not in PATH" 1>&2
        return 1
    end

    set -l out (shellmark --out fish $argv)

    if test -n "Sout"
        eval "$out"
    end
end