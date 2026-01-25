# pathsearch

Search for files in your PATH.

## Features

- **Respects PATH order**: Results are shown in the exact order they appear in your PATH. The first result is the file that would actually run when you type the command in your shell.
- Substring matching (default) or regex matching (-r)
- Color output with match highlighting (auto-detects TTY)

## Installation

### Cargo install

```shell
cargo install --git https://github.com/andybug/pathsearch.git
```

### Arch Linux (AUR)

```shell
yay -S pathsearch
# or
paru -S pathsearch
```

## Usage

```
pathsearch [OPTIONS] [pattern]

Arguments:
  [pattern]    Search pattern (substring match by default)

Options:
  -r, --regex        Interpret pattern as regex
      --color WHEN   Control color output [auto, always, never]
  -h, --help         Print help
  -V, --version      Print version
```

## Examples

Find files containing "vim":

```shell
$ pathsearch vim
/usr/bin/vim
/usr/bin/gvim
/usr/bin/nvim
```

Find which Python binary your shell would execute (check for shadowed versions):

```shell
$ pathsearch python3
/home/user/.local/bin/python3     # This one would run
/usr/bin/python3                  # Shadowed by the above
```

Find files starting with "python" (regex):

```shell
$ pathsearch -r '^python'
/usr/bin/python
/usr/bin/python3
```

List all files in PATH:

```shell
$ pathsearch
```

Pipe to fzf/skim for interactive selection:

```shell
$ pathsearch | fzf
$ pathsearch --color always | sk --ansi
```
