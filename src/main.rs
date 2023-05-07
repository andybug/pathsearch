use clap::Parser;
use std::path::PathBuf;
use std::{env, fs, process};
use strsim::jaro_winkler;

mod file_filter;
use file_filter::{ExecutableFilter, FileFilter, FuzzyFilter, RegexFilter, SubstringFilter};

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
}

#[derive(PartialEq, PartialOrd)]
enum SearchType {
    All,
    Substring,
    Regex,
    Fuzzy,
}

struct Config {
    dirs: Vec<PathBuf>,
    search: String,
    search_type: SearchType,
    sort: bool,
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

        Config {
            dirs: dirs,
            search: args.filename.unwrap_or(String::from("undefined")),
            search_type: search_type,
            sort: args.sort,
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

    let mut file_filters: Vec<Box<dyn FileFilter>> = Vec::new();

    match config.search_type {
        SearchType::All => {}
        SearchType::Substring => file_filters.push(Box::new(SubstringFilter::new(&config.search))),
        SearchType::Regex => file_filters.push(Box::new(RegexFilter::new(&config.search).unwrap())),
        SearchType::Fuzzy => file_filters.push(Box::new(FuzzyFilter::new(&config.search))),
    }

    file_filters.push(Box::new(ExecutableFilter {}));

    let mut matched_files: Vec<PathBuf> = Vec::new();

    for dir in config.dirs {
        let files = fs::read_dir(&dir).expect("Failed to read directory");

        for file in files {
            let matched = file_filters.iter().fold(true, |result, filter| {
                result && filter.filter(file.as_ref().unwrap())
            });

            if matched {
                matched_files.push(file.as_ref().unwrap().path());
            }
        }
    }

    if config.sort && (config.search_type == SearchType::Substring) {
        sort_files_by_similarity(&config.search, &mut matched_files);
    }

    for file in matched_files {
        println!("{}", file.display());
    }

    process::ExitCode::SUCCESS
}

fn sort_files_by_similarity(filename: &str, matched_files: &mut Vec<PathBuf>) {
    matched_files.sort_by_key(|path| {
        let file_name = path.file_name().unwrap().to_string_lossy();
        let similarity = jaro_winkler(file_name.as_ref(), filename);
        (similarity * -1.0 * 1000.0) as i32
    });
}
