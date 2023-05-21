use fuzzy_matcher::skim::SkimMatcherV2;
use fuzzy_matcher::FuzzyMatcher;
use regex::Regex;

#[derive(Debug, PartialEq)]
pub enum FileNameMatch {
    None,
    SingleRange((usize, usize)),
}

pub trait FileNameFilter {
    fn filter(&self, filename: &str) -> Option<FileNameMatch>;
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
    fn filter(&self, filename: &str) -> Option<FileNameMatch> {
        if let Some(index) = filename.find(&self.pattern) {
            return Some(FileNameMatch::SingleRange((
                index,
                index + self.pattern.len(),
            )));
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
    fn filter(&self, filename: &str) -> Option<FileNameMatch> {
        match self.skim_matcher.fuzzy_match(filename, &self.pattern) {
            Some(_score) => return Some(FileNameMatch::None),
            None => return None,
        }
    }
}

#[derive(Debug)]
pub struct RegexFilter {
    regex: Regex,
}

impl RegexFilter {
    pub fn new(pattern: &str) -> Result<Self, regex::Error> {
        match Regex::new(pattern) {
            Ok(regex) => Ok(RegexFilter { regex }),
            Err(err) => Err(err),
        }
    }
}

impl FileNameFilter for RegexFilter {
    fn filter(&self, filename: &str) -> Option<FileNameMatch> {
        match self.regex.find(filename) {
            Some(first_match) => {
                return Some(FileNameMatch::SingleRange((
                    first_match.start(),
                    first_match.end(),
                )))
            }
            None => return None,
        }
    }
}

pub struct MatchAllFilter {}

impl FileNameFilter for MatchAllFilter {
    fn filter(&self, _filename: &str) -> Option<FileNameMatch> {
        Some(FileNameMatch::None)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn substring_filter_returns_none_when_no_match() {
        let filter = SubstringFilter::new("abc");
        let result = filter.filter("def");
        assert_eq!(result, None);
    }

    #[test]
    fn substring_filter_returns_match_range_when_pattern_found() {
        let filter = SubstringFilter::new("abc");
        /* cspell:disable-next-line */
        let result = filter.filter("xyzabc123");
        assert_eq!(result, Some(FileNameMatch::SingleRange((3, 6))));
    }

    #[test]
    fn substring_filter_returns_first_match_range_when_multiple_patterns_found() {
        let filter = SubstringFilter::new("abc");
        /* cspell:disable-next-line */
        let result = filter.filter("xyzabc123abc");
        assert_eq!(result, Some(FileNameMatch::SingleRange((3, 6))));
    }

    #[test]
    fn fuzzy_filter_returns_none_when_no_match() {
        let filter = FuzzyFilter::new("abc");
        let result = filter.filter("def");
        assert_eq!(result, None);
    }

    #[test]
    fn fuzzy_filter_returns_none_when_match_found() {
        let filter = FuzzyFilter::new("abc");
        let result = filter.filter("abracadabra");
        assert_eq!(result, Some(FileNameMatch::None));
    }

    #[test]
    fn regex_filter_returns_none_when_no_match() {
        let filter = RegexFilter::new(r"\d+").unwrap();
        let result = filter.filter("abc");
        assert_eq!(result, None);
    }

    #[test]
    fn regex_filter_returns_match_range_when_pattern_found() {
        let filter = RegexFilter::new(r"\d+").unwrap();
        let result = filter.filter("abc123def");
        assert_eq!(result, Some(FileNameMatch::SingleRange((3, 6))));
    }

    #[test]
    fn regex_filter_returns_first_match_range_when_multiple_patterns_found() {
        let filter = RegexFilter::new(r"\d+").unwrap();
        let result = filter.filter("abc123def456");
        assert_eq!(result, Some(FileNameMatch::SingleRange((3, 6))));
    }

    #[test]
    fn regex_filter_returns_error_when_invalid_pattern() {
        let filter = RegexFilter::new(r"(").unwrap_err();
        assert_eq!(filter.to_string().contains("regex parse error"), true);
    }

    #[test]
    fn match_all_filter() {
        let ma_filter = MatchAllFilter {};
        let m = ma_filter.filter("");
        assert_eq!(m.is_some(), true);
        assert_eq!(m.unwrap(), FileNameMatch::None);
    }
}
