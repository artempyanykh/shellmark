# `shellmark`: bookmark manager for shell

_(Aspirational description)_

`shellmark` is a cross-platform bookmark mananger for your shell. 
The main features are:
1. `shellmark add` to bookmark directories and files.
2. `shellmark browse` to interactively search and act on bookmarks.

For convenience setup an `alias b='shellmark'`.

## Integration with shell

### Fish

```
function s
    if ! type -q shellmark
        echo "shellmark is not in PATH" 1>&2
        return 1
    end

    set -l out (shellmark --out posix $argv)

    if test -n "Sout"
        eval "$out"
    end
end
```

### PowerShell

```
function Invoke-Shellmark {
    if (-Not (Get-Command shellmark -ErrorAction SilentlyContinue)) {
        Write-Output "shellmark not found in path"
        return
    }
    $OUT = shellmark --out powershell @args
    if ($OUT) {
        try {
            Invoke-Expression $OUT
        }
        catch {
            Write-Output $OUT
        }
    }
}
```