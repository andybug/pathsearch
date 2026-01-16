use regex::bytes::Regex;

#[derive(Debug, PartialEq)]
pub enum FilterResult {
    Matched(MatchRange),
    NoMatch,
}

#[derive(Debug, PartialEq)]
pub enum MatchRange {
    None,
    Range(usize, usize),
}

pub trait FileNameFilter {
    fn filter(&self, filename: &[u8]) -> FilterResult;
}

#[derive(Default)]
pub struct MatchAllFilter {}

impl FileNameFilter for MatchAllFilter {
    fn filter(&self, _filename: &[u8]) -> FilterResult {
        FilterResult::Matched(MatchRange::None)
    }
}

pub struct SubstringFilter {
    pattern: Vec<u8>,
}

impl SubstringFilter {
    pub fn new(pattern: &str) -> Self {
        SubstringFilter {
            pattern: pattern.as_bytes().to_vec(),
        }
    }
}

impl FileNameFilter for SubstringFilter {
    fn filter(&self, filename: &[u8]) -> FilterResult {
        match filename
            .windows(self.pattern.len())
            .position(|window| window == self.pattern.as_slice())
        {
            Some(start) => {
                FilterResult::Matched(MatchRange::Range(start, start + self.pattern.len()))
            }
            None => FilterResult::NoMatch,
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
    fn filter(&self, filename: &[u8]) -> FilterResult {
        match self.regex.find(filename) {
            Some(m) => FilterResult::Matched(MatchRange::Range(m.start(), m.end())),
            None => FilterResult::NoMatch,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn substring_filter_returns_none_when_no_match() {
        let filter = SubstringFilter::new("abc");
        let result = filter.filter(b"def");
        assert_eq!(result, FilterResult::NoMatch);
    }

    #[test]
    fn substring_filter_returns_match_range_when_pattern_found() {
        let filter = SubstringFilter::new("abc");
        /* cspell:disable-next-line */
        let result = filter.filter(b"xyzabc123");
        assert_eq!(result, FilterResult::Matched(MatchRange::Range(3, 6)));
    }

    #[test]
    fn substring_filter_returns_first_match_range_when_multiple_patterns_found() {
        let filter = SubstringFilter::new("abc");
        /* cspell:disable-next-line */
        let result = filter.filter(b"xyzabc123abc");
        assert_eq!(result, FilterResult::Matched(MatchRange::Range(3, 6)));
    }

    #[test]
    fn regex_filter_returns_none_when_no_match() {
        let filter = RegexFilter::new(r"\d+").unwrap();
        let result = filter.filter(b"abc");
        assert_eq!(result, FilterResult::NoMatch);
    }

    #[test]
    fn regex_filter_returns_match_range_when_pattern_found() {
        let filter = RegexFilter::new(r"\d+").unwrap();
        let result = filter.filter(b"abc123def");
        assert_eq!(result, FilterResult::Matched(MatchRange::Range(3, 6)));
    }

    #[test]
    fn regex_filter_returns_first_match_range_when_multiple_patterns_found() {
        let filter = RegexFilter::new(r"\d+").unwrap();
        let result = filter.filter(b"abc123def456");
        assert_eq!(result, FilterResult::Matched(MatchRange::Range(3, 6)));
    }

    #[test]
    fn regex_filter_returns_error_when_invalid_pattern() {
        let filter = RegexFilter::new(r"(").unwrap_err();
        assert_eq!(filter.to_string().contains("regex parse error"), true);
    }
}
