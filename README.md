# pathsearch

Search for executables in your PATH.

## Features

- Searches PATH directories in order (first match = what your shell executes)
- Substring matching (default) or regex matching (-r)
- Color output with match highlighting (auto-detects TTY)
- Handles non-UTF8 filenames correctly

## Installation

### From source

```shell
cargo build --release
cp target/release/pathsearch ~/.local/bin/
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

Find executables containing "vim":

```shell
$ pathsearch vim
/usr/bin/vim
/usr/bin/gvim
/usr/bin/nvim
```

Find executables starting with "python" (regex):

```shell
$ pathsearch -r '^python'
/usr/bin/python
/usr/bin/python3
```

List all executables in PATH:

```shell
$ pathsearch
```

Pipe to fzf/skim for interactive selection:

```shell
$ pathsearch | fzf
$ pathsearch --color always | sk --ansi
```
