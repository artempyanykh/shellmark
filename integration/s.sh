#!/usr/bin/env bash

function {name} {
    if ! type shellmark &>/dev/null; then
        echo "shellmark is not in PATH"
        return 1
    fi

    local out
    out="$(shellmark --out posix $@)"

    if [[ -n $out ]]; then
        eval "$out"
    fi
}