# pathsearch

A lightweight Rust utility that searches for files in `$PATH` directories.

## What it does

Searches each directory in `$PATH` (in order) and finds files matching a given pattern. Results are displayed in the order they would be found during command execution, showing which file would actually run and any shadowed alternatives.

## Design Goals

### Minimal Dependencies
The primary goal of this rewrite is to eliminate or minimize external dependencies:

- **No argument parsing crate** - Use manual `std::env::args()` parsing
- **No fuzzy matching** - Removed as a feature (rarely used)
- **No `atty`** - Use `std::io::IsTerminal` (stable since Rust 1.70)
- **Minimal regex dependency** - Uses `regex` crate with `default-features = false` and only `std` and `unicode-perl` features

Target: Single minimal dependency (`regex`) with reduced features for pattern matching.

### Simple String Processing
Modern implementation using Rust's String type:

- Pattern matching uses standard String operations
- Uses `regex::Regex` for regex patterns (operates on UTF-8 strings)
- Simple and maintainable code prioritized over micro-optimizations

### Simple and Maintainable
- Single-threaded (PATH is typically small, I/O is kernel-cached)
- Straightforward control flow
- Target ~200 lines of code

## Command Line Interface

```
pathsearch [OPTIONS] [pattern]

Arguments:
  [pattern]      Search pattern (substring match by default, optional)

Options:
  -r, --regex        Interpret pattern as regex
      --color WHEN   Control color output [auto, always, never]
  -h, --help         Print help
  -V, --version      Print version
```

## Pattern Matching Behavior

Without `-r` (default): substring match (case-sensitive)

With `-r`: 
- `^foo` - Prefix match (optimized, no regex engine)
- `bar$` - Suffix match (optimized, no regex engine)  
- Complex patterns - Full regex via `regex::bytes::Regex`

## Output

- One matching file per line, full path
- Ordered by PATH precedence (first = would be executed)
- Color output: auto-detect TTY (use `std::io::IsTerminal`)
  - Consider: first match in green, shadowed matches dimmed

## Platform

Cross-platform support for Unix-like systems (Linux, macOS) and Windows.

## File Structure

```
src/
  main.rs             # Entry point, arg parsing, output
  filename_filter.rs  # Pattern matching via trait-based filters
Cargo.toml
CLAUDE.md
README.md
doc/
  pathsearch.1        # Man page
```

## Build Configuration

```toml
[package]
name = "pathsearch"
version = "0.2.2"
edition = "2024"

[dependencies]
regex = { version = "1", default-features = false, features = ["std", "unicode-perl"] }

[profile.release]
lto = true
opt-level = "z"
strip = true
```

## Implementation Notes

### Pattern Matching
Implemented via trait-based filters in `filename_filter.rs`:

```rust
pub trait FileNameFilter {
    fn filter(&self, filename: &str) -> FilterResult;
}

pub enum FilterResult {
    Matched(MatchRange),
    NoMatch,
}
```

Three filter implementations:
- `MatchAllFilter`: Matches everything (when no pattern provided)
- `SubstringFilter`: Case-sensitive substring matching using `String::find()`
- `RegexFilter`: Full regex matching via `regex::Regex`

### Directory Iteration
```rust
for path_dir in std::env::var_os("PATH")?.to_string_lossy().split(':') {
    // Read directory, match filenames
}
```
