//! pathsearch - Search for files in PATH
//!
//! Searches each directory in the PATH environment variable for files
//! matching a given pattern. Results are displayed in PATH order, so the first
//! match is the file that would run if you typed the command.

use std::io::{self, IsTerminal, Write};
use std::path::{MAIN_SEPARATOR, PathBuf};
use std::{env, fs, process};

mod filename_filter;
use filename_filter::{
    FileNameFilter, FilterResult, MatchAllFilter, MatchRange, RegexFilter, SubstringFilter,
};

struct Args {
    pattern: Option<String>,
    regex: bool,
    color: ColorOption,
}

#[derive(Default, Clone, Copy)]
enum ColorOption {
    #[default]
    Auto,
    Always,
    Never,
}

#[derive(PartialEq, PartialOrd)]
enum SearchType {
    /// Match all files on path if no pattern provided
    MatchAll,
    Substring,
    Regex,
}

struct Config {
    dirs: Vec<PathBuf>,
    pattern: Option<String>,
    search_type: SearchType,
    /// Enable color output. Default true unless not a TTY.
    color: bool,
}

impl Config {
    fn new() -> Config {
        let args = match Args::parse_manual() {
            Ok(args) => args,
            Err(err) => {
                eprintln!("Error: {}", err);
                eprintln!();
                print_help();
                process::exit(1);
            }
        };
        let path = env::var("PATH").expect("Failed to get PATH");
        let dirs = env::split_paths(&path).collect();
        let search_type = if args.pattern.is_none() {
            SearchType::MatchAll
        } else if args.regex {
            SearchType::Regex
        } else {
            SearchType::Substring
        };
        let color = match args.color {
            ColorOption::Auto => io::stdout().is_terminal(),
            ColorOption::Always => true,
            ColorOption::Never => false,
        };

        Config {
            dirs,
            pattern: args.pattern,
            search_type,
            color,
        }
    }

    fn validate(&self) -> bool {
        if self.dirs.is_empty() {
            eprintln!("No directories in PATH");
            return false;
        }

        true
    }
}

// Manual argument parser (replaces clap)
impl Args {
    fn parse_manual() -> Result<Args, String> {
        let mut args_iter = env::args().skip(1);
        let mut pattern = None;
        let mut regex = false;
        let mut color = ColorOption::Auto;

        while let Some(arg) = args_iter.next() {
            match arg.as_str() {
                "-r" | "--regex" => regex = true,
                "-h" | "--help" => {
                    print_help();
                    process::exit(0);
                }
                "-V" | "--version" => {
                    println!("pathsearch {}", env!("CARGO_PKG_VERSION"));
                    process::exit(0);
                }
                "--color" => {
                    let value = args_iter
                        .next()
                        .ok_or("--color requires a value (auto, always, never)")?;
                    color = parse_color_option(&value)?;
                }
                s if s.starts_with("--color=") => {
                    let value = &s["--color=".len()..];
                    color = parse_color_option(value)?;
                }
                s if s.starts_with("-") => {
                    return Err(format!("Unknown option: {}", s));
                }
                s => {
                    if pattern.is_some() {
                        return Err("Multiple patterns provided".to_string());
                    }
                    pattern = Some(s.to_string());
                }
            }
        }

        Ok(Args {
            pattern,
            regex,
            color,
        })
    }
}

fn parse_color_option(s: &str) -> Result<ColorOption, String> {
    match s {
        "auto" => Ok(ColorOption::Auto),
        "always" => Ok(ColorOption::Always),
        "never" => Ok(ColorOption::Never),
        _ => Err(format!(
            "Invalid color option '{}'. Use 'auto', 'always', or 'never'",
            s
        )),
    }
}

fn print_help() {
    println!("pathsearch {}", env!("CARGO_PKG_VERSION"));
    println!("Look for files in PATH");
    println!();
    println!("USAGE:");
    println!("    pathsearch [OPTIONS] <pattern>");
    println!();
    println!("ARGUMENTS:");
    println!("    <pattern>    Search pattern (substring, or regex with -r)");
    println!();
    println!("OPTIONS:");
    println!("    -r, --regex              Interpret pattern as regex");
    println!("        --color <WHEN>       Control color output [auto, always, never]");
    println!("    -h, --help               Print help");
    println!("    -V, --version            Print version");
}

fn main() -> process::ExitCode {
    let config = Config::new();
    if !config.validate() {
        process::exit(1);
    }

    let filename_filter: Box<dyn FileNameFilter> = match config.search_type {
        SearchType::MatchAll => Box::new(MatchAllFilter::default()),
        SearchType::Substring => Box::new(SubstringFilter::new(
            &config
                .pattern
                .expect("pattern required for substring search"),
        )),
        SearchType::Regex => {
            let pattern = config.pattern.expect("pattern required for regex search");
            let filter = RegexFilter::new(&pattern).unwrap_or_else(|err| {
                eprintln!("Invalid regex pattern '{}': {}", pattern, err);
                process::exit(1);
            });
            Box::new(filter)
        }
    };

    let output = FormattedOutput::new(config.color);
    let mut output_handle = io::stdout().lock();

    // Iterate PATH directories in order. First match = what the shell would execute.
    for dir in config.dirs {
        let files = match fs::read_dir(&dir) {
            Ok(files) => files,
            Err(_) => {
                // users often have nonexistent directories in their PATH, silently ignore them
                continue;
            }
        };

        // convert PathBuf's OsString to String once per directory
        let dir_str = dir.display().to_string();

        for file in files {
            let file_ref = match file.as_ref() {
                Ok(dir_entry) => dir_entry,
                Err(err) => {
                    eprintln!("Failed to get directory entry in '{}': {}", &dir_str, err);
                    continue;
                }
            };
            let file_name = file_ref.file_name().display().to_string();
            let filter_result = filename_filter.filter(&file_name);

            if let FilterResult::Matched(match_range) = filter_result {
                output.print(&mut output_handle, &dir_str, &file_name, match_range);
            }
        }
    }

    process::ExitCode::SUCCESS
}

struct FormattedOutput {
    /// ANSI color code for the directory portion of the path
    ///
    /// The general idea is to make the directory portion fade into the
    /// background a bit so that the user can more easily see the matched
    /// filenames. It still needs to be legible since it provides
    /// important information.
    dir_ansi: &'static str,
    /// ANSI color code for the foreground color of the matched range
    match_ansi: &'static str,
    /// ANSI reset code
    reset_ansi: &'static str,
}

impl FormattedOutput {
    fn new(color: bool) -> Self {
        match color {
            true => Self {
                // decreased intensity
                dir_ansi: "\x1B[2m",
                // bold red foreground
                match_ansi: "\x1B[1;31m",
                reset_ansi: "\x1B[0m",
            },
            false => Self {
                dir_ansi: "",
                match_ansi: "",
                reset_ansi: "",
            },
        }
    }

    /// Print a matching file path with optional color highlighting.
    fn print(&self, output: &mut impl Write, dir: &str, file: &str, range: MatchRange) {
        // write directory with dimmed color
        let _ = write!(
            output,
            "{}{}{}{}",
            self.dir_ansi, dir, MAIN_SEPARATOR, self.reset_ansi
        );

        // write filename with match range highlighting
        let filename = file.as_bytes();
        match range {
            MatchRange::None => {
                let _ = output.write(filename);
            }
            MatchRange::Range(start, end) => {
                let _ = output.write_all(&filename[..start]);
                let _ = write!(output, "{}", self.match_ansi);
                let _ = output.write_all(&filename[start..end]);
                let _ = write!(output, "{}", self.reset_ansi);
                let _ = output.write_all(&filename[end..]);
            }
        }

        let _ = writeln!(output, "{}", self.reset_ansi);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // ========================================
    // FormattedOutput tests
    // ========================================

    mod formatted_output {
        use super::*;

        // ANSI escape code constants for test assertions
        const DIM: &str = "\x1B[2m";
        const BOLD_RED: &str = "\x1B[1;31m";
        const RESET: &str = "\x1B[0m";

        // --- Construction tests ---

        #[test]
        fn new_with_color_enabled() {
            let output = FormattedOutput::new(true);
            assert_eq!(output.dir_ansi, DIM);
            assert_eq!(output.match_ansi, BOLD_RED);
            assert_eq!(output.reset_ansi, RESET);
        }

        #[test]
        fn new_with_color_disabled() {
            let output = FormattedOutput::new(false);
            assert_eq!(output.dir_ansi, "");
            assert_eq!(output.match_ansi, "");
            assert_eq!(output.reset_ansi, "");
        }

        // --- Print output tests (no color) ---

        #[test]
        fn print_no_color_no_match_range() {
            let output = FormattedOutput::new(false);
            let mut buf = Vec::new();
            output.print(&mut buf, "/usr/bin", "ls", MatchRange::None);
            assert_eq!(String::from_utf8(buf).unwrap(), "/usr/bin/ls\n");
        }

        #[test]
        fn print_no_color_with_match_range() {
            let output = FormattedOutput::new(false);
            let mut buf = Vec::new();
            output.print(&mut buf, "/usr/bin", "grep", MatchRange::Range(0, 4));
            assert_eq!(String::from_utf8(buf).unwrap(), "/usr/bin/grep\n");
        }

        // --- Print output tests (with color) ---

        #[test]
        fn print_color_no_match_range() {
            let output = FormattedOutput::new(true);
            let mut buf = Vec::new();
            output.print(&mut buf, "/usr/bin", "ls", MatchRange::None);
            let result = String::from_utf8(buf).unwrap();
            // Directory should be dimmed, filename plain, ends with reset
            let expected = format!("{DIM}/usr/bin/{RESET}ls{RESET}\n");
            assert_eq!(result, expected);
        }

        #[test]
        fn print_color_match_at_start() {
            let output = FormattedOutput::new(true);
            let mut buf = Vec::new();
            output.print(&mut buf, "/usr/bin", "grep", MatchRange::Range(0, 2));
            let result = String::from_utf8(buf).unwrap();
            // "gr" highlighted, "ep" plain
            let expected = format!("{DIM}/usr/bin/{RESET}{BOLD_RED}gr{RESET}ep{RESET}\n");
            assert_eq!(result, expected);
        }

        #[test]
        fn print_color_match_at_end() {
            let output = FormattedOutput::new(true);
            let mut buf = Vec::new();
            output.print(&mut buf, "/usr/bin", "grep", MatchRange::Range(2, 4));
            let result = String::from_utf8(buf).unwrap();
            // "gr" plain, "ep" highlighted
            let expected = format!("{DIM}/usr/bin/{RESET}gr{BOLD_RED}ep{RESET}{RESET}\n");
            assert_eq!(result, expected);
        }

        #[test]
        fn print_color_match_in_middle() {
            let output = FormattedOutput::new(true);
            let mut buf = Vec::new();
            output.print(&mut buf, "/usr/bin", "cargo", MatchRange::Range(1, 3));
            let result = String::from_utf8(buf).unwrap();
            // "c" plain, "ar" highlighted, "go" plain
            let expected = format!("{DIM}/usr/bin/{RESET}c{BOLD_RED}ar{RESET}go{RESET}\n");
            assert_eq!(result, expected);
        }

        #[test]
        fn print_color_full_filename_match() {
            let output = FormattedOutput::new(true);
            let mut buf = Vec::new();
            output.print(&mut buf, "/usr/bin", "ls", MatchRange::Range(0, 2));
            let result = String::from_utf8(buf).unwrap();
            // Entire "ls" highlighted
            let expected = format!("{DIM}/usr/bin/{RESET}{BOLD_RED}ls{RESET}{RESET}\n");
            assert_eq!(result, expected);
        }

        // --- Edge case tests ---

        #[test]
        fn print_empty_filename() {
            let output = FormattedOutput::new(false);
            let mut buf = Vec::new();
            output.print(&mut buf, "/usr/bin", "", MatchRange::None);
            assert_eq!(String::from_utf8(buf).unwrap(), "/usr/bin/\n");
        }

        #[test]
        fn print_filename_with_ansi_escape_in_name() {
            // Filenames could theoretically contain ANSI sequences
            let output = FormattedOutput::new(false);
            let mut buf = Vec::new();
            let filename = "file\x1B[31mred";
            output.print(&mut buf, "/tmp", filename, MatchRange::None);
            // Should pass through unchanged (no sanitization)
            assert_eq!(String::from_utf8(buf).unwrap(), "/tmp/file\x1B[31mred\n");
        }

        #[test]
        fn print_empty_directory() {
            let output = FormattedOutput::new(false);
            let mut buf = Vec::new();
            output.print(&mut buf, "", "ls", MatchRange::None);
            assert_eq!(String::from_utf8(buf).unwrap(), "/ls\n");
        }

        // --- Match range boundary tests ---

        #[test]
        fn print_match_range_single_byte() {
            let output = FormattedOutput::new(true);
            let mut buf = Vec::new();
            output.print(&mut buf, "/usr/bin", "abc", MatchRange::Range(1, 2));
            let result = String::from_utf8(buf).unwrap();
            // "a" plain, "b" highlighted, "c" plain
            let expected = format!("{DIM}/usr/bin/{RESET}a{BOLD_RED}b{RESET}c{RESET}\n");
            assert_eq!(result, expected);
        }

        #[test]
        fn print_match_range_with_multibyte_utf8() {
            // Test that byte-based slicing works correctly with UTF-8
            let output = FormattedOutput::new(true);
            let mut buf = Vec::new();
            // "café" - the 'é' is 2 bytes (0xc3 0xa9)
            let filename = "café";
            // Match "af" which spans bytes 1-3 (the 'a' and first byte of 'é')
            // This tests that we're doing byte slicing, not character slicing
            output.print(&mut buf, "/tmp", filename, MatchRange::Range(1, 3));
            // The output will slice at byte boundaries
            let result = buf;
            // "c" then highlighted "af" (bytes 1-3) then "é" remainder
            assert!(result.len() > 0); // Just verify it doesn't panic
        }
    }
}
