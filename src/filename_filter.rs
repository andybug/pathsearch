use fuzzy_matcher::skim::SkimMatcherV2;
use fuzzy_matcher::FuzzyMatcher;
use regex::Regex;
use std::fs::DirEntry;

pub enum FileNameMatch {
    None,
    SingleRange((usize, usize)),
}

pub trait FileNameFilter {
    fn filter(&self, file: &DirEntry) -> Option<FileNameMatch>;
}

pub struct SubstringFilter {
    pattern: String,
}

impl SubstringFilter {
    pub fn new(pattern: &str) -> Self {
        SubstringFilter {
            pattern: pattern.to_owned(),
        }
    }
}

impl FileNameFilter for SubstringFilter {
    fn filter(&self, file: &DirEntry) -> Option<FileNameMatch> {
        if let Some(file_name) = file.file_name().to_str() {
            if let Some(index) = file_name.find(&self.pattern) {
                return Some(FileNameMatch::SingleRange((
                    index,
                    index + self.pattern.len(),
                )));
            }
        }
        None
    }
}

pub struct FuzzyFilter {
    pattern: String,
    skim_matcher: SkimMatcherV2,
}

impl FuzzyFilter {
    pub fn new(pattern: &str) -> FuzzyFilter {
        FuzzyFilter {
            pattern: String::from(pattern),
            skim_matcher: SkimMatcherV2::default(),
        }
    }
}

impl FileNameFilter for FuzzyFilter {
    fn filter(&self, file: &DirEntry) -> Option<FileNameMatch> {
        let file_name = file.file_name();
        if let Some(file_name_str) = file_name.to_str() {
            match self.skim_matcher.fuzzy_match(file_name_str, &self.pattern) {
                Some(_score) => return Some(FileNameMatch::None),
                None => return None,
            }
        } else {
            None
        }
    }
}

pub struct RegexFilter {
    regex: Regex,
}

impl RegexFilter {
    pub fn new(pattern: &str) -> Result<Self, regex::Error> {
        let regex = Regex::new(pattern)?;
        Ok(RegexFilter { regex })
    }
}

impl FileNameFilter for RegexFilter {
    fn filter(&self, file: &DirEntry) -> Option<FileNameMatch> {
        if let Some(file_name) = file.file_name().to_str() {
            match self.regex.find(file_name) {
                Some(first_match) => {
                    return Some(FileNameMatch::SingleRange((
                        first_match.start(),
                        first_match.end(),
                    )))
                }
                None => return None,
            }
        } else {
            None
        }
    }
}

pub struct MatchAllFilter {}

impl FileNameFilter for MatchAllFilter {
    fn filter(&self, _file: &DirEntry) -> Option<FileNameMatch> {
        Some(FileNameMatch::None)
    }
}
