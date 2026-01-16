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

struct MatchedFile {
    path: PathBuf,
    matches: MatchRange,
}

struct Config {
    dirs: Vec<PathBuf>,
    pattern: Option<String>,
    search_type: SearchType,
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

        Config {
            dirs,
            pattern: args.pattern,
            search_type,
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

    let mut matched_files: Vec<MatchedFile> = Vec::new();

    for dir in config.dirs {
        let files = match fs::read_dir(&dir) {
            Ok(files) => files,
            Err(e) => {
                eprintln!("Failed to read directory: '{}': {}", dir.display(), e);
                continue;
            }
        };

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
                    matched_files.push(MatchedFile {
                        path: file_ref.path(),
                        matches: match_range,
                    });
                }
            }
        }
    }

    let use_color = io::stdout().is_terminal();
    for file in matched_files {
        if use_color {
            let output = io::stdout();
            print_colorized_path(file, &mut output.lock())
        } else {
            println!("{}", file.path.display());
        }
    }

    process::ExitCode::SUCCESS
}

const fn is_executable(mode: u32, is_file: bool, is_symlink: bool) -> bool {
    mode & 0o111 != 0 && (is_file || is_symlink)
}

fn print_colorized_path<W: Write>(file: MatchedFile, output: &mut W) {
    // ANSI color codes
    const FG_GREY: &str = "\u{001B}[38;5;250m";
    const RESET: &str = "\u{001B}[0m";

    let parent_dir = file.path.parent().unwrap();
    let file_name = file.path.file_name().unwrap();

    let parent_dir_str = parent_dir.to_string_lossy();
    let file_name_str = get_colorized_filename(file_name.to_string_lossy().as_ref(), &file);

    writeln!(
        output,
        "{}{}/{}{}{}",
        FG_GREY, parent_dir_str, RESET, file_name_str, RESET
    )
    .unwrap();
}

fn get_colorized_filename(filename: &str, matched_file: &MatchedFile) -> String {
    const FG_RED_BOLD: &str = "\u{001B}[1;31m";
    const RESET: &str = "\u{001B}[0m";

    match matched_file.matches {
        MatchRange::None => String::from(filename),
        MatchRange::Range(start, end) => {
            let mut colored_string = String::new();
            colored_string.push_str(&filename[..start]);
            colored_string.push_str(FG_RED_BOLD);
            colored_string.push_str(&filename[start..end]);
            colored_string.push_str(RESET);
            colored_string.push_str(&filename[end..]);

            colored_string
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Cursor;

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

    const FG_GREY: &str = "\u{001B}[38;5;240m";
    const FG_RED_BOLD: &str = "\u{001B}[1;31m";
    const FG_WHITE: &str = "\u{001B}[38;5;15m";
    const RESET: &str = "\u{001B}[0m";

    #[test]
    fn test_print_colorized_path() {
        let matched_file = MatchedFile {
            path: PathBuf::from("/path/to/file.txt"),
            matches: MatchRange::None,
        };

        let mut output = Cursor::new(Vec::new());

        print_colorized_path(matched_file, &mut output);

        let output_str = String::from_utf8(output.into_inner()).unwrap();
        let expected_output = format!(
            "{}{}/{}{}{}\n",
            FG_GREY, "/path/to", FG_WHITE, "file.txt", RESET
        );
        assert_eq!(output_str, expected_output);
    }

    #[test]
    fn test_print_colorized_path_highlight() {
        let matched_file = MatchedFile {
            path: PathBuf::from("/path/to/file.txt"),
            matches: MatchRange::Range(2, 4),
        };

        let mut output = Cursor::new(Vec::new());

        print_colorized_path(matched_file, &mut output);

        let output_str = String::from_utf8(output.into_inner()).unwrap();
        let mut expected_output = String::new();
        expected_output.push_str(FG_GREY);
        expected_output.push_str("/path/to/");
        expected_output.push_str(FG_WHITE);
        expected_output.push_str("fi");
        expected_output.push_str(FG_RED_BOLD);
        expected_output.push_str("le");
        expected_output.push_str(RESET);
        expected_output.push_str(".txt");
        expected_output.push_str(RESET);
        expected_output.push('\n');

        assert_eq!(output_str, expected_output);
    }

    #[test]
    fn test_get_colorized_filename_none_match() {
        let filename = "example.txt";
        let matched_file = MatchedFile {
            path: PathBuf::new(),
            matches: MatchRange::None,
        };

        let result = get_colorized_filename(filename, &matched_file);

        assert_eq!(result, String::from(filename));
    }

    #[test]
    fn test_get_colorized_filename_single_range_match() {
        let filename = "example.txt";
        let matched_file = MatchedFile {
            path: PathBuf::new(),
            matches: MatchRange::Range(2, 6),
        };

        let result = get_colorized_filename(filename, &matched_file);

        /* cspell:disable-next-line */
        let expected_output = format!("ex{}ampl{}e.txt", FG_RED_BOLD, RESET);

        assert_eq!(result, expected_output);
    }
}
