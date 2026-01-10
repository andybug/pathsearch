# pathsearch

A lightweight Rust utility that searches for executables in `$PATH` directories.

## What it does

Searches each directory in `$PATH` (in order) and finds executables matching a given pattern. Results are displayed in the order they would be found during command execution, showing which executable would actually run and any shadowed alternatives.

## Design Goals

### Minimal Dependencies
The primary goal of this rewrite is to eliminate or minimize external dependencies:

- **No argument parsing crate** - Use manual `std::env::args()` parsing
- **No fuzzy matching** - Removed as a feature (rarely used)
- **No `atty`** - Use `std::io::IsTerminal` (stable since Rust 1.70)
- **Regex is optional** - Simple prefix/suffix/contains patterns handled manually; `regex` crate only as fallback for complex patterns

Target: Zero dependencies for common use cases, optional `regex` dependency with `default-features = false` for complex patterns.

### Zero/Minimal Allocations
Work directly with `OsStr` bytes to avoid unnecessary string conversions:

- Use `std::os::unix::ffi::OsStrExt` to access `&[u8]` directly
- Avoid `to_string_lossy()` allocations
- Use `regex::bytes::Regex` when regex is needed
- Pattern matching operates on `&[u8]`, not `&str`

### Simple and Maintainable
- Single-threaded (PATH is typically small, I/O is kernel-cached)
- Straightforward control flow
- Target ~200 lines of code

## Command Line Interface

```
pathsearch [OPTIONS] <pattern>

Arguments:
  <pattern>    Search pattern (substring, or regex with -r)

Options:
  -r, --regex    Interpret pattern as regex
  -h, --help     Print help
  -V, --version  Print version
```

## Pattern Matching Behavior

Without `-r` (default): substring match (case-sensitive)

With `-r`: 
- `^foo` - Prefix match (optimized, no regex engine)
- `bar$` - Suffix match (optimized, no regex engine)  
- Complex patterns - Full regex via `regex::bytes::Regex`

## Output

- One matching executable per line, full path
- Ordered by PATH precedence (first = would be executed)
- Color output: auto-detect TTY (use `std::io::IsTerminal`)
  - Consider: first match in green, shadowed matches dimmed

## Platform

Unix/Linux only (uses `OsStrExt::as_bytes()`). No Windows support needed.

## File Structure

```
src/
  main.rs      # Entry point, arg parsing, output
  pattern.rs   # Pattern enum and matching logic (optional, could be in main.rs)
Cargo.toml
Claude.md
```

## Build Configuration

```toml
[package]
name = "pathsearch"
version = "0.2.0"
edition = "2021"

[dependencies]
# Empty for simple patterns, or:
# regex = { version = "1", default-features = false, features = ["std"], optional = true }

[features]
default = []
# regex = ["dep:regex"]  # Enable for complex regex support
```

## Implementation Notes

### Pattern Parsing (when regex mode enabled)
```rust
enum Pattern<'a> {
    Contains(&'a [u8]),
    Prefix(&'a [u8]),
    Suffix(&'a [u8]),
    Regex(regex::bytes::Regex),
}
```

Detect simple anchored patterns (`^...` or `...$` without other metacharacters) and use direct byte comparison. Fall back to regex engine only for complex patterns.

### Directory Iteration
```rust
for path_dir in std::env::var_os("PATH")?.as_bytes().split(|&b| b == b':') {
    // Read directory, match filenames, check executable bit
}
```

### Executable Check
Use `std::os::unix::fs::PermissionsExt` to check executable bit rather than spawning processes.
