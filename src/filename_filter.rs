//! Filename filtering for pattern matching.
//!
//! Provides a trait-based abstraction for different matching strategies:
//! - MatchAllFilter: matches everything (used when no pattern provided)
//! - SubstringFilter: case-sensitive substring matching
//! - RegexFilter: full regex matching via the regex crate

use regex::Regex;

#[derive(Debug, PartialEq)]
pub enum FilterResult {
    Matched(MatchRange),
    NoMatch,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum MatchRange {
    None,
    Range(usize, usize),
}

pub trait FileNameFilter {
    fn filter(&self, filename: &str) -> FilterResult;
}

#[derive(Default)]
pub struct MatchAllFilter {}

impl FileNameFilter for MatchAllFilter {
    fn filter(&self, _filename: &str) -> FilterResult {
        FilterResult::Matched(MatchRange::None)
    }
}

pub struct SubstringFilter {
    pattern: String,
}

impl SubstringFilter {
    pub fn new(pattern: &str) -> Self {
        SubstringFilter {
            pattern: pattern.to_string(),
        }
    }
}

impl FileNameFilter for SubstringFilter {
    fn filter(&self, filename: &str) -> FilterResult {
        match filename.find(&self.pattern) {
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
    fn filter(&self, filename: &str) -> FilterResult {
        match self.regex.find(filename) {
            Some(m) => FilterResult::Matched(MatchRange::Range(m.start(), m.end())),
            None => FilterResult::NoMatch,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // ========================================
    // MatchAllFilter tests
    // ========================================

    #[test]
    fn match_all_filter_returns_matched_with_no_range() {
        let filter = MatchAllFilter::default();
        let result = filter.filter("anything");
        assert_eq!(result, FilterResult::Matched(MatchRange::None));
    }

    #[test]
    fn match_all_filter_matches_empty_filename() {
        let filter = MatchAllFilter::default();
        let result = filter.filter("");
        assert_eq!(result, FilterResult::Matched(MatchRange::None));
    }

    // ========================================
    // SubstringFilter tests
    // ========================================

    #[test]
    fn substring_filter_returns_none_when_no_match() {
        let filter = SubstringFilter::new("abc");
        let result = filter.filter("def");
        assert_eq!(result, FilterResult::NoMatch);
    }

    #[test]
    fn substring_filter_returns_match_range_when_pattern_found() {
        let filter = SubstringFilter::new("abc");
        /* cspell:disable-next-line */
        let result = filter.filter("xyzabc123");
        assert_eq!(result, FilterResult::Matched(MatchRange::Range(3, 6)));
    }

    #[test]
    fn substring_filter_returns_first_match_range_when_multiple_patterns_found() {
        let filter = SubstringFilter::new("abc");
        /* cspell:disable-next-line */
        let result = filter.filter("xyzabc123abc");
        assert_eq!(result, FilterResult::Matched(MatchRange::Range(3, 6)));
    }

    #[test]
    fn substring_filter_empty_filename() {
        let filter = SubstringFilter::new("abc");
        let result = filter.filter("");
        assert_eq!(result, FilterResult::NoMatch);
    }

    #[test]
    fn substring_filter_match_at_start() {
        let filter = SubstringFilter::new("abc");
        let result = filter.filter("abcdef");
        assert_eq!(result, FilterResult::Matched(MatchRange::Range(0, 3)));
    }

    #[test]
    fn substring_filter_match_at_end() {
        let filter = SubstringFilter::new("abc");
        let result = filter.filter("defabc");
        assert_eq!(result, FilterResult::Matched(MatchRange::Range(3, 6)));
    }

    #[test]
    fn substring_filter_exact_match() {
        let filter = SubstringFilter::new("abc");
        let result = filter.filter("abc");
        assert_eq!(result, FilterResult::Matched(MatchRange::Range(0, 3)));
    }

    #[test]
    fn substring_filter_pattern_longer_than_filename() {
        let filter = SubstringFilter::new("abcdef");
        let result = filter.filter("abc");
        assert_eq!(result, FilterResult::NoMatch);
    }

    #[test]
    fn substring_filter_case_sensitive() {
        let filter = SubstringFilter::new("abc");
        let result = filter.filter("ABC");
        assert_eq!(result, FilterResult::NoMatch);
    }

    // ========================================
    // RegexFilter tests
    // ========================================

    #[test]
    fn regex_filter_returns_none_when_no_match() {
        let filter = RegexFilter::new(r"\d+").unwrap();
        let result = filter.filter("abc");
        assert_eq!(result, FilterResult::NoMatch);
    }

    #[test]
    fn regex_filter_returns_match_range_when_pattern_found() {
        let filter = RegexFilter::new(r"\d+").unwrap();
        let result = filter.filter("abc123def");
        assert_eq!(result, FilterResult::Matched(MatchRange::Range(3, 6)));
    }

    #[test]
    fn regex_filter_returns_first_match_range_when_multiple_patterns_found() {
        let filter = RegexFilter::new(r"\d+").unwrap();
        let result = filter.filter("abc123def456");
        assert_eq!(result, FilterResult::Matched(MatchRange::Range(3, 6)));
    }

    #[test]
    fn regex_filter_returns_error_when_invalid_pattern() {
        let filter = RegexFilter::new(r"(").unwrap_err();
        assert_eq!(filter.to_string().contains("regex parse error"), true);
    }

    #[test]
    fn regex_filter_empty_filename() {
        let filter = RegexFilter::new(r"\d+").unwrap();
        let result = filter.filter("");
        assert_eq!(result, FilterResult::NoMatch);
    }

    #[test]
    fn regex_filter_anchored_start() {
        let filter = RegexFilter::new(r"^foo").unwrap();
        assert_eq!(
            filter.filter("foobar"),
            FilterResult::Matched(MatchRange::Range(0, 3))
        );
        assert_eq!(filter.filter("barfoo"), FilterResult::NoMatch);
    }

    #[test]
    fn regex_filter_anchored_end() {
        let filter = RegexFilter::new(r"bar$").unwrap();
        assert_eq!(
            filter.filter("foobar"),
            FilterResult::Matched(MatchRange::Range(3, 6))
        );
        assert_eq!(filter.filter("barfoo"), FilterResult::NoMatch);
    }

    #[test]
    fn regex_filter_full_match() {
        let filter = RegexFilter::new(r"^foobar$").unwrap();
        assert_eq!(
            filter.filter("foobar"),
            FilterResult::Matched(MatchRange::Range(0, 6))
        );
        assert_eq!(filter.filter("foobar!"), FilterResult::NoMatch);
    }

    #[test]
    fn regex_filter_empty_match() {
        // Pattern that can match zero characters
        let filter = RegexFilter::new(r"a*").unwrap();
        // On "bbb", "a*" matches empty string at position 0
        let result = filter.filter("bbb");
        assert_eq!(result, FilterResult::Matched(MatchRange::Range(0, 0)));
    }
}
