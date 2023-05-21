use clap::{Parser, ValueEnum};
use std::os::unix::fs::PermissionsExt;
use std::path::PathBuf;
use std::{env, fs, process};
use strsim::jaro_winkler;

mod filename_filter;
use filename_filter::{
    FileNameFilter, FileNameMatch, FuzzyFilter, MatchAllFilter, RegexFilter, SubstringFilter,
};

#[derive(Parser, Debug)]
#[command(name = "pathsearch", about = "Look for executables in the search path")]
#[command(version = "0.1")]
struct Args {
    #[arg(name = "filename", index = 1, help = "Search query")]
    filename: Option<String>,
    #[arg(short, long, default_value = "false", help = "Use regex matching")]
    regex: bool,
    #[arg(short, long, default_value = "false", help = "Use fuzzy matching")]
    fuzzy: bool,
    #[arg(
        short,
        long,
        default_value = "false",
        help = "Sort files by similarity to search"
    )]
    sort: bool,
    #[clap(
        long,
        value_enum,
        default_value = "auto",
        help = "Choose whether to emit color output"
    )]
    color: ColorOption,
}

#[derive(ValueEnum, Clone, Debug)]
enum ColorOption {
    Auto,
    Always,
    Never,
}

#[derive(PartialEq, PartialOrd)]
enum SearchType {
    All,
    Substring,
    Regex,
    Fuzzy,
}

struct MatchedFile {
    path: PathBuf,
    matches: FileNameMatch,
}

struct Config {
    dirs: Vec<PathBuf>,
    search: String,
    search_type: SearchType,
    sort: bool,
    color: bool,
}

impl Config {
    fn new() -> Config {
        let args = Args::parse();
        let path = env::var("PATH").expect("Failed to get PATH");
        let dirs = env::split_paths(&path).collect();
        let search_type = match (args.filename.is_some(), args.regex, args.fuzzy) {
            (false, _, _) => SearchType::All,
            (true, false, false) => SearchType::Substring,
            (true, true, false) => SearchType::Regex,
            (true, false, true) => SearchType::Fuzzy,
            (true, true, true) => SearchType::Substring, // TODO: print warning here?
        };
        let color = match args.color {
            ColorOption::Always => true,
            ColorOption::Never => false,
            ColorOption::Auto => atty::is(atty::Stream::Stdout),
        };

        Config {
            dirs: dirs,
            search: args.filename.unwrap_or(String::from("undefined")),
            search_type: search_type,
            sort: args.sort,
            color: color,
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

fn main() -> process::ExitCode {
    let config = Config::new();
    if !config.validate() {
        process::exit(1);
    }

    let filename_filter: Box<dyn FileNameFilter> = match config.search_type {
        SearchType::All => Box::new(MatchAllFilter {}),
        SearchType::Substring => Box::new(SubstringFilter::new(&config.search)),
        SearchType::Regex => Box::new(RegexFilter::new(&config.search).unwrap()),
        SearchType::Fuzzy => Box::new(FuzzyFilter::new(&config.search)),
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
            let file_name = String::from(file_ref.file_name().to_string_lossy());
            let matched = filename_filter.filter(&file_name);

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

            if matched.is_some()
                && is_executable(
                    metadata.permissions().mode(),
                    metadata.is_file(),
                    metadata.is_symlink(),
                )
            {
                matched_files.push(MatchedFile {
                    path: file_ref.path(),
                    matches: matched.unwrap(),
                });
            }
        }
    }

    if config.sort && (config.search_type == SearchType::Substring) {
        sort_files_by_similarity(&config.search, &mut matched_files);
    }

    for file in matched_files {
        if config.color {
            print_colorized_path(file)
        } else {
            println!("{}", file.path.display());
        }
    }

    process::ExitCode::SUCCESS
}

fn sort_files_by_similarity(filename: &str, matched_files: &mut Vec<MatchedFile>) {
    matched_files.sort_by_key(|matched_file| {
        let file_name = matched_file.path.file_name().unwrap().to_str();
        let similarity = jaro_winkler(file_name.unwrap(), filename);
        // Convert the similarity score to a negative integer for descending order sorting
        (similarity * -1.0 * 1000.0) as i32
    });
}

fn is_executable(mode: u32, is_file: bool, is_symlink: bool) -> bool {
    mode & 0o111 != 0 && (is_file || is_symlink)
}

fn print_colorized_path(file: MatchedFile) {
    // ANSI color codes
    const FG_GREY: &str = "\u{001B}[38;5;240m";
    const FG_WHITE: &str = "\u{001B}[38;5;15m";
    const RESET: &str = "\u{001B}[0m";

    let parent_dir = file.path.parent().unwrap();
    let file_name = file.path.file_name().unwrap();

    let parent_dir_str = parent_dir.to_string_lossy();
    let file_name_str = get_colorized_filename(file_name.to_string_lossy().as_ref(), &file);

    println!(
        "{}{}/{}{}{}",
        FG_GREY, parent_dir_str, FG_WHITE, file_name_str, RESET
    );
}

fn get_colorized_filename(filename: &str, matched_file: &MatchedFile) -> String {
    const FG_RED_BOLD: &str = "\u{001B}[1;31m";
    const RESET: &str = "\u{001B}[0m";

    match matched_file.matches {
        FileNameMatch::None => String::from(filename),
        FileNameMatch::SingleRange((start, end)) => {
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

    #[test]
    fn test_sort_files_by_similarity() {
        let filename = "example";
        let mut matched_files = vec![
            MatchedFile {
                path: PathBuf::from("file1.txt"),
                matches: FileNameMatch::None,
            },
            MatchedFile {
                path: PathBuf::from("test-example.txt"),
                matches: FileNameMatch::None,
            },
            MatchedFile {
                /* cspell:disable-next-line */
                path: PathBuf::from("examlpe.txt"),
                matches: FileNameMatch::None,
            },
        ];

        sort_files_by_similarity(filename, &mut matched_files);

        // Check if the files are sorted in descending order of similarity
        let mut prev_similarity = f64::MAX;
        for matched_file in matched_files {
            let file_name = matched_file.path.file_name().unwrap().to_str().unwrap();
            let similarity = jaro_winkler(file_name, filename);
            assert!(similarity <= prev_similarity);
            prev_similarity = similarity;
        }
    }

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
