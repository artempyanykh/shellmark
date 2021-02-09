#!/usr/bin/env bash

function s {
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