# pathsearch

pathsearch is a Rust program that searches the user's PATH for a given query,
providing a list of executables that match the query. This program was written
with the help of AI, using the GPT-3.5 architecture from OpenAI.

## Features

- Searches the user's PATH for a given search query
- Outputs all the executables in the PATH if no search query is provided
- Supports substring, regex, and fuzzy searches
- Can sort substring searches by similarity to the given search query
- Colorizes the output for easier reading

## Usage

    Usage: pathsearch [OPTIONS] [filename]

    Arguments:
      [filename]  Search query

    Options:
      -r, --regex          Use regex matching
      -f, --fuzzy          Use fuzzy matching
      -s, --sort           Sort files by similarity to search
          --color <COLOR>  Choose whether to emit color output [default: auto] [possible values: auto, always, never]
      -h, --help           Print help
      -V, --version        Print version

## Examples

Search for executables that contain the substring "vim" and sorted by
similarity:

```shell
$ pathsearch -s vim
/usr/bin/vim
/usr/bin/rvim
/usr/bin/nvim
/usr/bin/vimdot
/usr/bin/vimdiff
/usr/bin/vimtutor
/usr/bin/nvimgdiff
```

## Interactive Fuzzy Searching

The output of `pathsearch` can be consumed by `skim` or `fzf` for easy
interactive querying.

```shell
pathsearch | sk
```

Optionally, force color on:

```shell
pathsearch --color always firefox | sk --ansi
```
