# `shellmark`: bookmark manager for shell

[![Build & Test](https://github.com/artempyanykh/shellmark/actions/workflows/push.yml/badge.svg?branch=main)](https://github.com/artempyanykh/shellmark/actions/workflows/push.yml)

<img src="./assets/shellmark.gif" alt="Shellmark demonstration: CLI and TUI"/>

`shellmark` is a cross-platform bookmark mananger for your shell. 
The main features are:
1. `shellmark add` to bookmark directories and files.
2. `shellmark browse` to interactively search and act on bookmarks.

## How to use

1. Install `shellmark` following [installation instructions below](#installation-instructions).
   Make sure `shellmark` is in your `PATH`.
2. Integrate `shellmark` with your shell following [integration instructions below](#integration-with-shell).
   This will add a shell alias `s`. The name of the alias is configurable. Run `shellmark plug --help` to learn more.
3. Invoke `shellmark` via `s` shell alias.

## Installation instructions

### Pre-built binary

1. Go to [Releases](https://github.com/artempyanykh/shellmark/releases) page
   and download the binary for your OS.
2. Rename the binary to remove the OS suffix, so it becomes just `shellmark`
   or `shellmark.exe`.
3. Drop the binary somewhere in your `PATH`.

### `cargo install`

1. Run `cargo install shellmark`.
2. The binary will be built and installed under a local Cargo folder, usually `$HOME/.cargo/bin`. 
   Make sure this directory is in your `PATH`.

### From source

Make sure you have Rust toolchain set up (1.49+ should work). Then run the following commands:

```bash
$ git clone https://github.com/artempyanykh/shellmark.git
$ cd shellmark
$ cargo install --path .
```
This will install `shellmark` under `~/.cargo/bin`.

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
