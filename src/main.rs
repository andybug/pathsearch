use std::ffi::OsStr;
use std::io::{self, IsTerminal, Write};
use std::os::unix::ffi::OsStrExt;
use std::os::unix::fs::PermissionsExt;
use std::path::PathBuf;
use std::{env, fs, process};

mod filename_filter;
use filename_filter::{
    FileNameFilter, FilterResult, MatchAllFilter, MatchRange, RegexFilter, SubstringFilter,
};

struct Args {
    pattern: Option<String>,
    regex: bool,
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
        let color = io::stdout().is_terminal();

        Config {
            dirs,
            pattern: args.pattern,
            search_type,
            color,
        }
    }

    fn validate(&self) -> bool {
        if self.dirs.len() == 0 {
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

        Ok(Args { pattern, regex })
    }
}

fn print_help() {
    println!("pathsearch {}", env!("CARGO_PKG_VERSION"));
    println!("Look for executables in the search path");
    println!();
    println!("USAGE:");
    println!("    pathsearch [OPTIONS] <pattern>");
    println!();
    println!("ARGUMENTS:");
    println!("    <pattern>    Search pattern (substring, or regex with -r)");
    println!();
    println!("OPTIONS:");
    println!("    -r, --regex    Interpret pattern as regex");
    println!("    -h, --help     Print help");
    println!("    -V, --version  Print version");
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

    for dir in config.dirs {
        let files = match fs::read_dir(&dir) {
            Ok(files) => files,
            Err(e) => {
                eprintln!("Failed to read directory: '{}': {}", dir.display(), e);
                continue;
            }
        };

        // convert PathBuf's OsString to String once per directory
        let dir_str = dir.display().to_string();

        for file in files {
            let file_ref = match file.as_ref() {
                Ok(dir_entry) => dir_entry,
                Err(err) => {
                    eprintln!(
                        "Failed to get directory entry in '{}': {}",
                        dir.display(),
                        err
                    );
                    continue;
                }
            };
            let file_name = file_ref.file_name();
            let filter_result = filename_filter.filter(file_name.as_bytes());

            if let FilterResult::Matched(match_range) = filter_result {
                let metadata = match file_ref.metadata() {
                    Ok(metadata) => metadata,
                    Err(err) => {
                        eprintln!(
                            "Failed to get file metadata for '{}': {}",
                            file_ref.path().display(),
                            err
                        );
                        continue;
                    }
                };

                if is_executable(
                    metadata.permissions().mode(),
                    metadata.is_file(),
                    metadata.is_symlink(),
                ) {
                    output.print(&mut output_handle, &dir_str, &file_name, match_range);
                }
            }
        }
    }

    process::ExitCode::SUCCESS
}

const fn is_executable(mode: u32, is_file: bool, is_symlink: bool) -> bool {
    mode & 0o111 != 0 && (is_file || is_symlink)
}

struct FormattedOutput {
    /// ANSI color code for the directory portion of the path
    ///
    /// The general idea is to make the directory portion fade into the
    /// background a bit so that the user can more easily see the matched
    /// executable names. It still needs to be legible since it provides
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
                dir_ansi: "\x1B[38;5;250m",
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

    fn print(&self, output: &mut impl Write, dir: &str, file: &OsStr, range: MatchRange) {
        // write directory with dimmed color
        let _ = write!(output, "{}{}/{}", self.dir_ansi, dir, self.reset_ansi);

        // write filename with match range highlighting
        let filename = file.as_bytes();
        match range {
            MatchRange::None => {
                let _ = output.write_all(filename);
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

    #[test]
    fn test_is_executable_file() {
        // Test when mode is executable, and it's a regular file
        let mode = 0o755;
        let is_file = true;
        let is_symlink = false;

        assert_eq!(is_executable(mode, is_file, is_symlink), true);
    }

    #[test]
    fn test_is_executable_symlink() {
        // Test when mode is executable, and it's a symbolic link
        let mode = 0o777;
        let is_file = false;
        let is_symlink = true;

        assert_eq!(is_executable(mode, is_file, is_symlink), true);
    }

    #[test]
    fn test_is_not_executable_file() {
        // Test when mode is not executable, and it's a regular file
        let mode = 0o644;
        let is_file = true;
        let is_symlink = false;

        assert_eq!(is_executable(mode, is_file, is_symlink), false);
    }

    #[test]
    fn test_is_not_executable_symlink() {
        // Test when mode is not executable, and it's a symbolic link
        let mode = 0o600;
        let is_file = false;
        let is_symlink = true;

        assert_eq!(is_executable(mode, is_file, is_symlink), false);
    }
}
