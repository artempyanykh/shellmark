# `shellmark`: bookmark manager for shell

_(Aspirational description)_

`shellmark` is a cross-platform bookmark mananger for your shell. 
The main features are:
1. `shellmark add` to bookmark directories and files.
2. `shellmark browse` to interactively search and act on bookmarks.

For convenience setup an `alias b='shellmark'`.

## Integration with shell

### Bash/Zsh

```
if type shellmark &>/dev/null; then
    eval "$(shellmark --out posix plug)"
fi
```

### Fish

```
if type -q shellmark
    shellmark --out fish plug | source
end
```

### PowerShell

```
if (Get-Command shellmark -ErrorAction SilentlyContinue) {
    Invoke-Expression (@(&shellmark --out powershell plug) -join "`n")
}
```