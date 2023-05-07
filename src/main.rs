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

fn main() -> process::ExitCode {
    let args = Args::parse();

    let filename = args.filename;
    let regex_enabled = args.regex;
    let fuzzy_enabled = args.fuzzy;
    let sort_enabled = args.sort;

    let path = env::var("PATH").expect("Failed to get PATH");

    let dirs = env::split_paths(&path);

    let mut file_filters: Vec<Box<dyn FileFilter>> = Vec::new();

    match (regex_enabled, fuzzy_enabled) {
        (true, false) => file_filters.push(Box::new(
            RegexFilter::new(filename.as_ref().unwrap_or(&String::from("")).as_str()).unwrap(),
        )),
        (false, true) => file_filters.push(Box::new(FuzzyFilter::new(
            filename.as_ref().unwrap_or(&String::from("")).as_str(),
        ))),
        (false, false) => file_filters.push(Box::new(SubstringFilter::new(
            filename.as_ref().unwrap_or(&String::from("")),
        ))),
        (true, true) => {
            eprintln!("Only specify one of --fuzzy or --regex");
            process::exit(1)
        }
    }

    file_filters.push(Box::new(ExecutableFilter {}));

    let mut matched_files: Vec<PathBuf> = Vec::new();

    for dir in dirs {
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

    if sort_enabled {
        matched_files.sort_by_key(|path| {
            let file_name = path.file_name().unwrap().to_string_lossy();
            let similarity = jaro_winkler(
                file_name.as_ref(),
                filename.as_ref().unwrap_or(&String::from("")).as_str(),
            );
            (similarity * -1.0 * 1000.0) as i32
        });
    }

    for file in matched_files {
        println!("{}", file.display());
    }

    process::ExitCode::SUCCESS
}
