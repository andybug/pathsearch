use fuzzy_matcher::skim::SkimMatcherV2;
use fuzzy_matcher::FuzzyMatcher;
use regex::Regex;
use std::fs::DirEntry;

pub trait FileNameFilter {
    fn filter(&self, file: &DirEntry) -> bool;
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
    fn filter(&self, file: &DirEntry) -> bool {
        if let Some(file_name) = file.file_name().to_str() {
            return file_name.contains(&self.pattern);
        }
        false
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
    fn filter(&self, file: &DirEntry) -> bool {
        let file_name = file.file_name();
        if let Some(file_name_str) = file_name.to_str() {
            self.skim_matcher
                .fuzzy_match(file_name_str, &self.pattern)
                .is_some()
        } else {
            false
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
    fn filter(&self, file: &DirEntry) -> bool {
        if let Some(file_name) = file.file_name().to_str() {
            self.regex.is_match(file_name)
        } else {
            false
        }
    }
}
